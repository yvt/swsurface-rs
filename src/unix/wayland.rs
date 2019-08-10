use std::ops::{Deref, DerefMut};
use std::os::raw::c_void;
use wayland_client::{self as wl, protocol::wl_shm};

use super::super::{Config, Format, ImageInfo};

#[derive(Debug)]
pub struct SurfaceImpl {}

impl SurfaceImpl {
    pub unsafe fn new(wl_dpy: *mut c_void, wl_srf: *mut c_void) -> Self {
        let dpy = DpyRef::from_dpy(wl_dpy);

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

    pub fn wait_next_image(&self) -> Option<usize> {
        unimplemented!()
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        &mut [][..]
    }

    pub fn present_image(&self, i: usize) {
        unimplemented!()
    }
}

// ---------------------------------------------------------------------------
// Display connection management
//
// We could create a `wayland_client::Display` start an event loop thread for
// every `SurfaceImpl` created, but we don't want to do that because of several
// reasons: (1) Every spawned thread incurs a moderate memory/runtime cost.
// (2) `wl_display::get_registry` creates a server side resource, which is
// released only when the client disconnects. This means a long-running
// application that repeatedly opens and closes windows would leak memory.
//
// We maintain a list of connections we know in `DPYS`. If we encountered a
// connection, we start up an event loop thread. and construct `Dpy`. When the
// number of users of the `Dpy` hits zero (i.e., when all `SurfaceImpl`
// pertaining to the connection are dropped), the `Dpy` is torn down.
//
// Known issue: If the application close all windows, we tear down the `Dpy`.
// If the application opens a new window later, we create `Dpy` again. If this
// is repeated, we'll leak memory.
//
// Possible work-around: Create our own "context" object that gets `wl_display`
// from `EventLoopExtUnix`.
use lazy_static::lazy_static;
use std::{
    fmt,
    sync::{mpsc::sync_channel, Arc, Mutex},
    thread,
};

lazy_static! {
    static ref DPYS: Mutex<Vec<Arc<Dpy>>> = Mutex::new(Vec::new());
}

struct DpyRef {
    dpy: Option<Arc<Dpy>>,
}

impl DpyRef {
    unsafe fn from_dpy(wl_dpy: *mut c_void) -> Self {
        let mut dpys = DPYS.lock().unwrap();

        // Find an existing suitable `Dpy`
        let dpy = dpys
            .iter()
            .find(|dpy| dpy.wl_dpy.get_display_ptr() as *mut c_void == wl_dpy);

        if let Some(dpy) = dpy {
            return Self {
                dpy: Some(Arc::clone(&dpy)),
            };
        }

        // Construct a new `Dpy`
        let dpy = Dpy::new(wl_dpy);
        dpys.push(Arc::clone(&dpy));

        Self { dpy: Some(dpy) }
    }
}

impl Deref for DpyRef {
    type Target = Dpy;

    fn deref(&self) -> &Self::Target {
        self.dpy.as_ref().unwrap()
    }
}

impl Drop for DpyRef {
    fn drop(&mut self) {
        let dpy = self.dpy.take().unwrap();

        if Arc::strong_count(&dpy) == 2 {
            // `DPYS` and we are the only owners, so destroy `Dpy`
            let mut dpys = DPYS.lock().unwrap();

            // Check the ref count again, someone else might have changed the
            // situation while we were waiting for lock
            if Arc::strong_count(&dpy) != 2 {
                return;
            }

            dpys.retain(|d| !Arc::ptr_eq(&dpy, d));
        }
    }
}

// ---------------------------------------------------------------------------
// Display connection

/// Display
struct Dpy {
    wl_dpy: wl::Display,
    // `Mutex` makes it `Sync`
    req_send: Mutex<calloop::channel::Sender<DpyReq>>,
    join_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl fmt::Debug for Dpy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dpy")
            .field("wl_dpy", &self.wl_dpy.get_display_ptr())
            .finish()
    }
}

enum DpyReq {
    Close,
    // TODO
}

/// Represents a shared memory object. Shared between the main thread and the
/// event loop thread
struct Shm {
    // TODO
}

impl Dpy {
    unsafe fn new(wl_dpy: *mut c_void) -> Arc<Self> {
        let wl_dpy = wl_dpy as usize; // make ptr sendable ;)

        let (this_send, this_recv) = sync_channel(1);

        let join_handle = thread::spawn(move || {
            // Construct a `Display` and `EventQueue` from an externally supplied
            // `wl_display`. These must be dropped before the `wl_display` is
            // disconnected.
            let (wl_dpy, mut wl_evq) = wl::Display::from_external_display(wl_dpy as _);

            // Create other things
            let manager = wl::GlobalManager::new(&wl_dpy);

            // Retrieve the globals metadata (without this, we will fail to get
            // the global `wl_shm`)
            wl_evq.sync_roundtrip().unwrap();
            wl_evq.sync_roundtrip().unwrap();

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

            // Use `calloop`-based event loop because we want to handle
            // custom events too
            let mut ca_evl = calloop::EventLoop::new().unwrap();
            let ca_evl_signal = ca_evl.get_signal();

            // Receive requests via this channel
            let (req_send, req_chan) = calloop::channel::channel();

            ca_evl
                .handle()
                .insert_source(req_chan, move |evt, &mut ()| {
                    // This closure handles `DpyReq`.
                    if let calloop::channel::Event::Msg(msg) = evt {
                        match msg {
                            DpyReq::Close => ca_evl_signal.stop(),
                        }
                    }
                })
                .unwrap();

            // Connect Wayland client to `ca_evl`
            ca_evl
                .handle()
                .insert_source(wl_evq, |(), &mut ()| {})
                .unwrap();

            // Construct `Self` in the event loop thread, and send it back to
            // the original thread
            let this = Arc::new(Self {
                wl_dpy,
                req_send: Mutex::new(req_send),
                join_handle: Mutex::new(None),
            });
            this_send.send(Arc::clone(&this)).unwrap();

            // Start the event loop. This will not return until
            // `ca_evl_signal.stop()` is called.
            ca_evl.run(None, &mut (), |_| {}).unwrap();
        });

        // Receive `Self` created in the event loop thread
        let this = this_recv.recv().unwrap();
        *this.join_handle.lock().unwrap() = Some(join_handle);

        this
    }

    // TODO: Methods for requesting `Shm`s
}

impl Drop for Dpy {
    fn drop(&mut self) {
        // Stop the event loop
        self.req_send
            .get_mut()
            .unwrap()
            .send(DpyReq::Close)
            .unwrap();

        self.join_handle
            // Get the contents of `Mutex` via `&mut`
            // using normal poisoning handling
            .get_mut()
            .unwrap()
            // There must be a `JoinHandle` unless something went very wrong
            // during the creation of `Dpy`
            .take()
            .unwrap()
            // Wait for the event loop thread to exit, propagating
            // a panic (if any)
            .join()
            .unwrap();
    }
}
