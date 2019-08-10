use std::{
    fmt,
    ops::{Deref, DerefMut},
    os::raw::c_void,
};
use wayland_client::{self as wl, protocol::wl_display, protocol::wl_shm};
use wayland_sys::{client::WAYLAND_CLIENT_HANDLE, ffi_dispatch};
use winit::window::WindowId;

use super::super::{Config, ContextBuilder, Format, ImageInfo};

pub struct ContextImpl {
    wl_dpy: wl_display::WlDisplay,
    wl_shm: wl_shm::WlShm,
}

impl fmt::Debug for ContextImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextImpl").finish()
    }
}

impl ContextImpl {
    pub unsafe fn new<T: 'static>(wl_dpy_ptr: *mut c_void, _: ContextBuilder<'_, T>) -> Self {
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

        Self { wl_dpy, wl_shm }
    }
}

#[derive(Debug)]
pub struct SurfaceImpl {}

impl SurfaceImpl {
    pub unsafe fn new(
        wl_dpy: *mut c_void,
        _wl_srf: *mut c_void,
        _wnd_id: WindowId,
        context: &ContextImpl,
        _config: &Config,
    ) -> Self {
        assert_eq!(wl_dpy, context.wl_dpy.as_ref().c_ptr() as _);

        unimplemented!()
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        unimplemented!()
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        [].iter().cloned()
    }

    pub fn image_info(&self) -> ImageInfo {
        unimplemented!()
    }

    pub fn num_images(&self) -> usize {
        unimplemented!()
    }

    pub fn does_preserve_image(&self) -> bool {
        unimplemented!()
    }

    pub fn poll_next_image(&self) -> Option<usize> {
        unimplemented!()
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        &mut [][..]
    }

    pub fn present_image(&self, i: usize) {
        unimplemented!()
    }
}
