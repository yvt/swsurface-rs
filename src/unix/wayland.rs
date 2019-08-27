use fragile::Fragile;
use log::trace;
use owning_ref::OwningRefMut;
use smithay_client_toolkit::utils::MemPool;
use std::{
    cell::{Cell, RefCell},
    fmt,
    ops::{Deref, DerefMut},
    os::raw::c_void,
    rc::Rc,
};
use wayland_client::{
    self as wl,
    protocol::{wl_buffer, wl_display, wl_shm, wl_surface},
};
use wayland_sys::{client::WAYLAND_CLIENT_HANDLE, ffi_dispatch};
use winit::window::WindowId;

use super::super::{align::Align, Config, ContextBuilder, Format, ImageInfo, ReadyCb};

#[derive(Clone)]
pub struct ContextImpl {
    // The following objects' lifetime is bound to the originating
    // `winit::event_loop::EventLoop`'s underlying `wl::Display` object. They
    // are valid as long as the `winit::event_loop_EventLoop` or
    // at least one instance of `winit::window::Window` created from it are
    // alive.
    wl_dpy: wl_display::WlDisplay,
    wl_shm: wl_shm::WlShm,
    ready_cb: Rc<ReadyCb>,
}

impl fmt::Debug for ContextImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextImpl").finish()
    }
}

impl ContextImpl {
    pub unsafe fn new<T: 'static>(wl_dpy_ptr: *mut c_void, builder: ContextBuilder<'_, T>) -> Self {
        let wl_dpy: wl_display::WlDisplay = wl::Proxy::from_c_ptr(wl_dpy_ptr as _).into();

        let manager = wl::GlobalManager::new(&wl_dpy);

        // Retrieve the globals metadata (without this, we will fail to get
        // the global `wl_shm`)
        for _ in 0..2 {
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, wl_dpy_ptr as _);
        }

        let wl_shm: wl_shm::WlShm = manager
            .instantiate_range(1, 1, |wl_shm| {
                wl_shm.implement_closure(
                    move |evt, _| {
                        // `wl_shm` sends suppored formats via events
                        if let wl_shm::Event::Format { format } = evt {
                            let _ = format;
                            // TODO: examine supported formats
                        }
                    },
                    (),
                )
            })
            .expect("server does not advertise `wl_shm`");

        Self {
            wl_dpy,
            wl_shm,

            ready_cb: Rc::new(builder.ready_cb),
        }
    }
}

#[derive(Debug)]
pub struct SurfaceImpl {
    state: Rc<State>,
}

/// This object is shared between `SharedImpl` and the event handler of
/// `wl_buffer`.
struct State {
    ctx: ContextImpl,

    wnd_id: WindowId,
    wl_srf: wl_surface::WlSurface,

    images: Box<[Image]>,

    /// If `true`, the `release` event handler will call `ready_cb` when
    /// called for the next time.
    enable_ready_cb: Cell<bool>,

    image_info: Cell<ImageInfo>,
    scanline_align: Align,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("ctx", &self.ctx)
            .field("wnd_id", &self.wnd_id)
            .field("images", &self.images)
            .field("enable_ready_cb", &self.enable_ready_cb)
            .field("image_info", &self.image_info)
            .finish()
    }
}

struct Image {
    /// `wl_shm_pool`, an associated shared memory region, and a `wl_buffer`
    /// created from it.
    ///
    /// `None` at the initial state (i.e., before `update_surface` is called
    /// for the first time).
    ///
    /// `wl_buffer` is created only when we are about to present it. Thus, the
    /// valid states are:
    ///
    ///  1. `mem = Some(_, None)`, `presenting = false`
    ///  1. `mem = Some(_, Some(_))`, `presenting = true`
    ///  1. `mem = Some(_, Some(_))`, `presenting = false`
    ///
    mem: RefCell<Option<(MemPool, Option<wl_buffer::WlBuffer>)>>,

    /// `true` if `mem` is currently in use by the server, i.e., we have sent
    /// it via `wl_surface::attach` but haven't received the `release` event.
    /// FIXME: Could be merged into `MemPool::is_used()`
    presenting: Cell<bool>,
}

impl Drop for Image {
    fn drop(&mut self) {
        let mem = self.mem.get_mut();
        if let Some(mem) = mem {
            if let Some(wl_buf) = mem.1.take() {
                trace!("Destroying `wl_buffer` {:?}", wl_buf.as_ref().c_ptr());

                // `wl_buf` could be still in use by the presenter, but there
                // isn't much we can do. The Wayland connection might not even
                // exist after this call to `drop`... (Remember that the
                // connection is managed by `winit`)
                wl_buf.destroy();
            }
        }
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image").finish()
    }
}

impl SurfaceImpl {
    pub unsafe fn new(
        wl_dpy: *mut c_void,
        wl_srf_ptr: *mut c_void,
        wnd_id: WindowId,
        context: &ContextImpl,
        config: &Config,
        scanline_align: Align,
    ) -> Self {
        assert_eq!(wl_dpy, context.wl_dpy.as_ref().c_ptr() as _);

        let images: Vec<_> = (0..config.image_count)
            .map(|_| Image {
                mem: RefCell::new(None),
                presenting: Cell::new(false),
            })
            .collect();

        let wl_srf: wl_surface::WlSurface = wl::Proxy::from_c_ptr(wl_srf_ptr as _).into();

        Self {
            state: Rc::new(State {
                ctx: context.clone(),
                wnd_id,
                wl_srf,
                images: images.into_boxed_slice(),
                enable_ready_cb: Cell::new(false),
                image_info: Cell::new(ImageInfo::default()),
                scanline_align,
            }),
        }
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        assert_ne!(extent[0], 0);
        assert_ne!(extent[1], 0);

        // Fail-fast if some images are locked by the appliction
        let mut mems: Vec<_> = self
            .state
            .images
            .iter()
            .map(|image| image.mem.try_borrow_mut().expect("some images are locked"))
            .collect();

        // Check the value range
        assert!(extent[0] <= <i32>::max_value() as u32);
        assert!(extent[1] <= <i32>::max_value() as u32);

        use std::convert::TryInto;
        let extent_usize: [usize; 2] = [
            extent[0].try_into().expect("overflow"),
            extent[1].try_into().expect("overflow"),
        ];

        let stride = extent_usize[0]
            .checked_mul(4)
            .and_then(|x| self.state.scanline_align.align_up(x))
            .expect("overflow");

        // `stride` must fit in `i32`
        let _bytes_per_line: i32 = stride.try_into().unwrap();

        // Calculate a new `ImageInfo`
        let image_info = ImageInfo {
            extent,
            stride,
            format,
        };

        trace!("{:?}: New image info = {:?}", self.state.wnd_id, image_info);

        let size = stride
            .checked_mul(image_info.extent[1] as usize)
            .expect("overflow");

        // Resize mempools
        for (i, mem) in mems.iter_mut().enumerate() {
            let (mem_pool, _) = mem.get_or_insert_with(|| {
                // `MemPool` isn't created yet, so make one now
                let state = Rc::clone(&self.state);

                // `MemPool` doesn't call the event handler from another thread
                // (AFAIK). It requires it to be `Send` only to allow you to
                // create a `MemPool` for a `WlShm` originaing from another
                // thread.  So assert that `state` will be used by the current
                // thread.
                let state = Fragile::new(state);

                let on_release = move || {
                    // Assert that we are using it from the correct thread
                    let state = state.get();

                    trace!("{:?}: Swapchain image {} was released", state.wnd_id, i);

                    state.images[i].presenting.set(false);

                    // Does the application want to receive a notification?
                    // If so, reset this flag and call the ready callback.
                    if state.enable_ready_cb.replace(false) {
                        trace!("Calling `ready_cb`");
                        (state.ctx.ready_cb)(state.wnd_id);
                    }
                };

                trace!("Creating `MemPool`");

                let mem_pool = MemPool::new(&self.state.ctx.wl_shm, on_release)
                    .expect("could not create `wl_shm_pool`");

                (mem_pool, None)
            });

            trace!("Resizing `MemPool` to {}", size);
            mem_pool
                .resize(size)
                .expect("could not resize the memory-mapped file");
        }

        self.state.image_info.set(image_info);
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        [Format::Argb8888].iter().cloned()
    }

    pub fn image_info(&self) -> ImageInfo {
        self.state.image_info.get()
    }

    pub fn num_images(&self) -> usize {
        self.state.images.len()
    }

    pub fn does_preserve_image(&self) -> bool {
        true
    }

    pub fn poll_next_image(&self) -> Option<usize> {
        let result = self
            .state
            .images
            .iter()
            .position(|image| image.presenting.get() == false);

        if let Some(i) = result {
            trace!(
                "{:?}: Swapchain image {} is available, returning it",
                self.state.wnd_id,
                i
            );
        } else {
            if self.state.enable_ready_cb.get() {
                trace!(
                    "{:?}: No swapchain image is available. `ready_cb` is already enabled.",
                    self.state.wnd_id
                );
            } else {
                trace!(
                    "{:?}: No swapchain image is available. Enabling `ready_cb`.",
                    self.state.wnd_id
                );
            }

            // Enable the ready callback
            self.state.enable_ready_cb.set(true);
        }

        result
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        let image = &self.state.images[i];

        assert_eq!(
            image.presenting.get(),
            false,
            "the image is currently in use by the compositor"
        );

        OwningRefMut::new(image.mem.borrow_mut()).map_mut(|x| {
            // `update_surface` should have been called at least one.
            // Otherwise, panic
            x.as_mut()
                .expect("surface is not initialized")
                .0
                // Get the underlying data of the memory-mapped file
                .mmap()
                .as_mut()
        })
    }

    pub fn present_image(&self, i: usize) {
        let image = &self.state.images[i];

        assert_eq!(
            image.presenting.get(),
            false,
            "the image is currently in use by the compositor"
        );

        let mut mem = image.mem.try_borrow_mut().expect("the image is locked");
        let (mem_pool, buffer_cell) = mem.as_mut().expect("surface is not initialized");

        let image_info = self.state.image_info.get();
        let format = match image_info.format {
            Format::Argb8888 => wl_shm::Format::Argb8888,
            Format::Xrgb8888 => wl_shm::Format::Xrgb8888,
        };

        // Create `wl_buffer`.
        let buffer = mem_pool.buffer(
            0,
            image_info.extent[0] as i32,
            image_info.extent[1] as i32,
            image_info.stride as i32,
            format,
        );

        trace!(
            "{:?}: Presenting swapchain image {} using `wl_buffer` {:?}",
            self.state.wnd_id,
            i,
            buffer.as_ref().c_ptr()
        );

        // The previous statement also updates `MemPool`'s flag to indicate
        // that `wl_buffer` is attached to a `wl_surface` and will raise the
        // `release` event in the near future.
        debug_assert!(mem_pool.is_used());

        // Attach the `wl_buffer` to the `wl_surface`.
        self.state.wl_srf.attach(Some(&buffer), 0, 0);
        self.state
            .wl_srf
            .damage_buffer(0, 0, image_info.extent[0] as _, image_info.extent[1] as _);
        self.state.wl_srf.commit();

        if let Some(old_buffer) = buffer_cell.take() {
            old_buffer.destroy();
        }

        *buffer_cell = Some(buffer);

        image.presenting.set(true);
    }
}
