//! OpenGL backend for macOS - This might sound weird, but every macOS system
//! (starting from the very first “Mac OS X”) reliably supports OpenGL. The
//! implementation seems to be quite resilient and automatically recovers from a
//! device reset event without major interruption. Even without a suitable
//! device driver (like in the recovery mode and during the operating
//! system installation), it keeps working with a resonably fast, feature-rich
//! software renderer.
use cocoa::{
    appkit::{self, NSOpenGLContext, NSOpenGLPixelFormat},
    base::{id, nil},
};
use owning_ref::OwningRefMut;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
};
use winit::{platform::macos::WindowExtMacOS, window::Window};

use super::{
    align::Align, buffer::Buffer, cglffi as gl, objcutils::IdRef, Config, Format, ImageInfo,
    NullContextImpl,
};

#[derive(Debug)]
pub struct SurfaceImpl {
    gl_context: IdRef,
    gl_tex: gl::GLuint,
    image: RefCell<Buffer>,
    image_info: Cell<ImageInfo>,
    scanline_align: Align,
}

impl SurfaceImpl {
    pub(crate) unsafe fn new(window: &Window, _: &NullContextImpl, config: &Config) -> Self {
        let scanline_align = Align::new(config.scanline_align).unwrap();

        // Create `NSOpenGLPixelFormat`
        let attrs = [
            appkit::NSOpenGLPFAOpenGLProfile as u32,
            appkit::NSOpenGLPFAOpenGLProfiles::NSOpenGLProfileVersionLegacy as u32,
            appkit::NSOpenGLPFAColorSize as u32,
            24,
            appkit::NSOpenGLPFAAlphaSize as u32,
            8,
            appkit::NSOpenGLPFADoubleBuffer as u32,
            // null termination
            0,
        ];
        let pixel_format = IdRef::new(NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attrs))
            .non_nil()
            .expect("no available pixel format");

        // Create `NSOpenGLContext`.
        let gl_context = IdRef::new(
            NSOpenGLContext::alloc(nil).initWithFormat_shareContext_(*pixel_format, nil),
        )
        .non_nil()
        .expect("could not create a OpenGL context");

        gl_context.setView_(window.ns_view() as id);

        gl_context.setValues_forParameter_(
            &(config.vsync as i32),
            appkit::NSOpenGLContextParameter::NSOpenGLCPSwapInterval,
        );

        if !config.opaque {
            cgl::CGLSetParameter(
                gl_context.CGLContextObj() as *mut _,
                cgl::kCGLCPSurfaceOpacity,
                &mut 0, // false (not opaque)
            );
        }

        // Create a texture name
        gl_context.makeCurrentContext();
        let mut gl_tex: gl::GLuint = 0;
        gl::glGenTextures(1, &mut gl_tex);

        Self {
            gl_context,
            gl_tex,
            image: RefCell::new(Buffer::from_size_align(1, config.align).unwrap()),
            image_info: Cell::new(ImageInfo::default()),
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

        let size = stride.checked_mul(extent_usize[1]).expect("overflow");

        let (ifmt, fmt, ty) = translate_format(format);

        let mut image = self.image.borrow_mut();
        let gl_context = &self.gl_context;
        unsafe {
            // Because the window was resized...
            gl_context.update();

            // Update the texture. We assume that NPOT textures are supported.
            // (This is true even for the first Intel Mac (with GMA950), IIRC)
            // TODO: Check maximum texture size
            gl_context.makeCurrentContext();
            gl::glBindTexture(gl::GL_TEXTURE_2D, self.gl_tex);
            gl::glTexImage2D(
                gl::GL_TEXTURE_2D,
                0,
                ifmt,
                extent[0] as i32,
                extent[1] as i32,
                0,
                fmt,
                ty,
                std::ptr::null(),
            );

            gl::glTexParameteri(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR);
            gl::glTexParameteri(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR);

            image.resize(size);
        }

        self.image_info.set(ImageInfo {
            extent,
            stride,
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
        // `present_image` will block instead, unfortunately.
        Some(0)
    }

    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        assert_eq!(i, 0);
        OwningRefMut::new(self.image.borrow_mut()).map_mut(|p| &mut **p)
    }

    pub fn present_image(&self, i: usize) {
        assert_eq!(i, 0);

        let gl_context = &self.gl_context;
        let image_info = self.image_info.get();
        let image = self
            .image
            .try_borrow()
            .expect("the image is currently locked");
        let (_ifmt, fmt, ty) = translate_format(image_info.format);

        unsafe {
            gl_context.makeCurrentContext();
            gl::glBindTexture(gl::GL_TEXTURE_2D, self.gl_tex);

            gl::glPixelStorei(gl::GL_UNPACK_ROW_LENGTH, (image_info.stride / 4) as _);
            gl::glTexSubImage2D(
                gl::GL_TEXTURE_2D,
                0,
                0,
                0,
                image_info.extent[0] as _,
                image_info.extent[1] as _,
                fmt,
                ty,
                image.as_ptr() as *const _,
            );
            gl::glPixelStorei(gl::GL_UNPACK_ROW_LENGTH, 0);

            gl::glClearColor(0.0, 0.0, 0.0, 0.0);
            gl::glClear(gl::GL_COLOR_BUFFER_BIT);
            gl::glEnable(gl::GL_TEXTURE_2D);

            gl::glBegin(gl::GL_TRIANGLE_STRIP);
            gl::glTexCoord2f(0.0, 0.0);
            gl::glVertex2f(-1.0, 1.0);
            gl::glTexCoord2f(2.0, 0.0);
            gl::glVertex2f(3.0, 1.0);
            gl::glTexCoord2f(0.0, 2.0);
            gl::glVertex2f(-1.0, -3.0);
            gl::glEnd();

            // According to my past observation, the following call is where
            // actual blocking occurs
            gl_context.flushBuffer();
        }
    }
}

fn translate_format(format: Format) -> (gl::GLenum, gl::GLenum, gl::GLenum) {
    match format {
        Format::Argb8888 => (gl::GL_RGBA, gl::GL_BGRA, gl::GL_UNSIGNED_BYTE),
        Format::Xrgb8888 => (gl::GL_RGB, gl::GL_BGRA, gl::GL_UNSIGNED_INT_8_8_8_8_REV),
    }
}
