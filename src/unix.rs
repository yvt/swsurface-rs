//! Wayland/X11 backend
use either::Either;
use std::ops::{Deref, DerefMut};
use winit::{platform::unix::*, window::Window};

use super::{align::Align, Config, ContextBuilder, Format, ImageInfo};

mod wayland;
mod x11;

#[derive(Debug)]
pub enum ContextImpl {
    Wayland(wayland::ContextImpl),
    X11,
}

impl ContextImpl {
    pub const TAKES_READY_CB: bool = true;

    pub fn new<T: 'static>(builder: ContextBuilder<'_, T>) -> Self {
        unsafe {
            match builder.event_loop.wayland_display() {
                Some(wl_dpy) => ContextImpl::Wayland(wayland::ContextImpl::new(wl_dpy, builder)),
                None => ContextImpl::X11,
            }
        }
    }
}

#[derive(Debug)]
pub enum SurfaceImpl {
    Wayland(wayland::SurfaceImpl),
    X11(x11::SurfaceImpl),
}

impl SurfaceImpl {
    pub(crate) unsafe fn new(window: &Window, context: &ContextImpl, config: &Config) -> Self {
        let scanline_align = Align::new(config.scanline_align).unwrap();

        match (
            window.wayland_display(),
            window.wayland_surface(),
            window.xlib_display(),
            window.xlib_window(),
        ) {
            (Some(wl_dpy), Some(wl_srf), _, _) => match context {
                ContextImpl::Wayland(context) => SurfaceImpl::Wayland(wayland::SurfaceImpl::new(
                    wl_dpy,
                    wl_srf,
                    window.id(),
                    context,
                    config,
                    scanline_align,
                )),
                ContextImpl::X11 => panic!("backend mismatch"),
            },
            (None, None, Some(x_dpy), Some(x_wnd)) => match context {
                ContextImpl::Wayland(_) => panic!("backend mismatch"),
                ContextImpl::X11 => SurfaceImpl::X11(x11::SurfaceImpl::new(
                    x_dpy,
                    x_wnd,
                    window.id(),
                    config,
                    scanline_align,
                )),
            },
            _ => unreachable!(),
        }
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        match self {
            SurfaceImpl::Wayland(imp) => imp.update_surface(extent, format),
            SurfaceImpl::X11(imp) => imp.update_surface(extent, format),
        }
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        match self {
            SurfaceImpl::Wayland(imp) => Either::Left(imp.supported_formats()),
            SurfaceImpl::X11(imp) => Either::Right(imp.supported_formats()),
        }
    }

    pub fn image_info(&self) -> ImageInfo {
        match self {
            SurfaceImpl::Wayland(imp) => imp.image_info(),
            SurfaceImpl::X11(imp) => imp.image_info(),
        }
    }

    pub fn num_images(&self) -> usize {
        match self {
            SurfaceImpl::Wayland(imp) => imp.num_images(),
            SurfaceImpl::X11(imp) => imp.num_images(),
        }
    }

    pub fn does_preserve_image(&self) -> bool {
        match self {
            SurfaceImpl::Wayland(imp) => imp.does_preserve_image(),
            SurfaceImpl::X11(imp) => imp.does_preserve_image(),
        }
    }

    pub fn poll_next_image(&self) -> Option<usize> {
        match self {
            SurfaceImpl::Wayland(imp) => imp.poll_next_image(),
            SurfaceImpl::X11(imp) => imp.poll_next_image(),
        }
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        match self {
            SurfaceImpl::Wayland(imp) => Either::Left(imp.lock_image(i)),
            SurfaceImpl::X11(imp) => Either::Right(imp.lock_image(i)),
        }
    }

    pub fn present_image(&self, i: usize) {
        match self {
            SurfaceImpl::Wayland(imp) => imp.present_image(i),
            SurfaceImpl::X11(imp) => imp.present_image(i),
        }
    }
}
