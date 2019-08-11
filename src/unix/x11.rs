use std::{
    ops::{Deref, DerefMut},
    os::raw::{c_ulong, c_void},
};
use winit::window::WindowId;

use super::super::{Config, Format, ImageInfo};

#[derive(Debug)]
pub struct SurfaceImpl {}

impl SurfaceImpl {
    pub unsafe fn new(
        x_dpy: *mut c_void,
        x_wnd: c_ulong,
        wnd_id: WindowId,
        config: &Config,
    ) -> Self {
        unimplemented!()
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        unimplemented!()
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        [Format::Argb8888].iter().cloned()
    }

    pub fn image_info(&self) -> ImageInfo {
        unimplemented!()
    }

    pub fn num_images(&self) -> usize {
        unimplemented!()
    }

    pub fn does_preserve_image(&self) -> bool {
        true
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
