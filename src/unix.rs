//! Wayland/X11 backend
use std::ops::{Deref, DerefMut};
use winit::{platform::unix::WindowExtUnix, window::Window};

use super::{Config, Format, ImageInfo};

#[derive(Debug)]
pub struct SurfaceImpl {
    // TODO
}

impl SurfaceImpl {
    pub unsafe fn new(window: &Window, _config: &Config) -> Self {
        match (
            window.wayland_display(),
            window.wayland_surface(),
            window.xlib_display(),
            window.xlib_window(),
        ) {
            (Some(wl_dpy), Some(wl_srf), _, _) => unimplemented!(),
            (None, None, Some(x_dpy), Some(x_wnd)) => unimplemented!(),
            _ => unreachable!(),
        }
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
