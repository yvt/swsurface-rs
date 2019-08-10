//! Wayland/X11 backend
use std::ops::{Deref, DerefMut};
use winit::{platform::unix::WindowExtUnix, window::Window};

use super::{Config, Format, ImageInfo};

mod wayland;

#[derive(Debug)]
pub enum SurfaceImpl {
    Wayland(wayland::SurfaceImpl), // TODO: X11
}

impl SurfaceImpl {
    pub unsafe fn new(window: &Window, _config: &Config) -> Self {
        match (
            window.wayland_display(),
            window.wayland_surface(),
            window.xlib_display(),
            window.xlib_window(),
        ) {
            (Some(wl_dpy), Some(wl_srf), _, _) => {
                SurfaceImpl::Wayland(wayland::SurfaceImpl::new(wl_dpy, wl_srf))
            }
            (None, None, Some(x_dpy), Some(x_wnd)) => unimplemented!(),
            _ => unreachable!(),
        }
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        match self {
            SurfaceImpl::Wayland(imp) => imp.update_surface(extent, format),
        }
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        match self {
            SurfaceImpl::Wayland(imp) => imp.supported_formats(),
        }
    }

    pub fn image_info(&self) -> ImageInfo {
        match self {
            SurfaceImpl::Wayland(imp) => imp.image_info(),
        }
    }

    pub fn num_images(&self) -> usize {
        match self {
            SurfaceImpl::Wayland(imp) => imp.num_images(),
        }
    }

    pub fn does_preserve_image(&self) -> bool {
        match self {
            SurfaceImpl::Wayland(imp) => imp.does_preserve_image(),
        }
    }

    pub fn wait_next_image(&self) -> Option<usize> {
        match self {
            SurfaceImpl::Wayland(imp) => imp.wait_next_image(),
        }
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        match self {
            SurfaceImpl::Wayland(imp) => imp.lock_image(i),
        }
    }

    pub fn present_image(&self, i: usize) {
        match self {
            SurfaceImpl::Wayland(imp) => imp.present_image(i),
        }
    }
}
