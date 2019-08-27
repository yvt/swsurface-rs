use log::debug;
use owning_ref::OwningRefMut;
use std::{
    cell::{Cell, RefCell},
    fmt,
    ops::{Deref, DerefMut},
    os::raw::{c_ulong, c_void},
};
use winit::window::WindowId;
use x11_dl::xlib;

use super::super::{align::Align, buffer::Buffer, Config, Format, ImageInfo};

// TODO: Non-opaque window

lazy_static::lazy_static! {
    static ref XLIB: xlib::Xlib = xlib::Xlib::open().unwrap();
}

pub struct SurfaceImpl {
    xlib: &'static xlib::Xlib,
    x_dpy: *mut xlib::Display,
    x_wnd: c_ulong,
    x_scrn: *mut xlib::Screen,
    image_info: Cell<ImageInfo>,
    image: RefCell<Buffer>,
    scanline_align: Align,
}

impl fmt::Debug for SurfaceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceImpl").finish()
    }
}

impl SurfaceImpl {
    pub unsafe fn new(
        x_dpy: *mut c_void,
        x_wnd: c_ulong,
        _wnd_id: WindowId,
        config: &Config,
        scanline_align: Align,
    ) -> Self {
        let xlib = &*XLIB;
        let x_dpy = x_dpy as *mut xlib::Display;

        // Get the window attributs
        let mut x_wnd_attrs: xlib::XWindowAttributes = std::mem::zeroed();
        (xlib.XGetWindowAttributes)(x_dpy, x_wnd, &mut x_wnd_attrs);
        let x_scrn = x_wnd_attrs.screen;
        assert!(!x_scrn.is_null());

        Self {
            xlib,
            x_dpy,
            x_wnd,
            x_scrn,
            image_info: Cell::new(ImageInfo::default()),
            image: RefCell::new(Buffer::from_size_align(1, config.align).unwrap()),
            scanline_align,
        }
    }

    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        assert_ne!(extent[0], 0);
        assert_ne!(extent[1], 0);
        assert!(extent[0] <= <i32>::max_value() as u32);
        assert!(extent[1] <= <i32>::max_value() as u32);

        use std::convert::TryInto;
        let extent_usize: [usize; 2] = [
            extent[0].try_into().expect("overflow"),
            extent[1].try_into().expect("overflow"),
        ];

        let stride = extent_usize[0]
            .checked_mul(4)
            .and_then(|x| self.scanline_align.align_up(x))
            .expect("overflow");

        // `stride` must fit in `XImage::bytes_per_line`
        let _bytes_per_line: i32 = stride.try_into().unwrap();

        let size = stride.checked_mul(extent_usize[1]).expect("overflow");

        let depth = unsafe { (self.xlib.XDefaultDepthOfScreen)(self.x_scrn) };
        debug!("DefaultDepthOfScreen = {}", depth);
        assert_ne!(depth, 0);

        // TODO: Probably we need this sometime
        let _ = depth;

        let mut image = self.image.borrow_mut();
        image.resize(size);

        self.image_info.set(ImageInfo {
            extent,
            stride: extent[0] as usize * 4,
            format,
        });
    }

    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        [Format::Argb8888, Format::Xrgb8888].iter().cloned()
    }

    pub fn image_info(&self) -> ImageInfo {
        self.image_info.get()
    }

    pub fn num_images(&self) -> usize {
        1
    }

    pub fn does_preserve_image(&self) -> bool {
        true
    }

    pub fn poll_next_image(&self) -> Option<usize> {
        Some(0)
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        assert_eq!(i, 0);
        OwningRefMut::new(self.image.borrow_mut()).map_mut(|p| &mut **p)
    }

    pub fn present_image(&self, i: usize) {
        assert_eq!(i, 0);

        let image_info = self.image_info.get();
        let image = self
            .image
            .try_borrow()
            .expect("the image is currently locked");

        // TODO: Use XShape to set the window shape based on alpha channel
        //       <https://www.x.org/releases/X11R7.7/doc/xextproto/shape.html>

        // TODO: See if this works on uncommon visuals

        unsafe {
            let mut x_image = xlib::XImage {
                width: image_info.extent[0] as _,
                height: image_info.extent[1] as _,
                xoffset: 0,
                format: xlib::ZPixmap,
                data: image.as_ptr() as *mut _,
                byte_order: if cfg!(target_endian = "little") {
                    xlib::LSBFirst
                } else {
                    xlib::MSBFirst
                },
                bitmap_unit: 32,
                bitmap_bit_order: xlib::LSBFirst,
                bitmap_pad: 32,
                depth: 24,
                bytes_per_line: image_info.stride as _,
                bits_per_pixel: 32,
                red_mask: 0xff0000,
                green_mask: 0xff00,
                blue_mask: 0xff,
                ..std::mem::zeroed()
            };

            (self.xlib.XInitImage)(&mut x_image);

            let x_gc = (self.xlib.XDefaultGCOfScreen)(self.x_scrn);

            (self.xlib.XPutImage)(
                self.x_dpy,
                self.x_wnd,
                x_gc,
                &mut x_image,
                0,
                0,
                0,
                0,
                image_info.extent[0] as _,
                image_info.extent[1] as _,
            );
        }
    }
}
