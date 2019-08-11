//! This crate provides a software-rendered surface for `winit`.
//!
//! The goal of this crate is to provide a minimal drawing functionality
//! for every platform supported by `winit` even if the drawing APIs that we
//! usually assume are available, such as OpenGL¹, aren't available in the
//! target environment. This crate is also useful as a fallback when they are
//! available, but failed due to an unrecoverable error.
//!
//! ¹ [“Servo on Windows in VirtualBox gets 'NoAvailablePixelFormat'” servo/servo #9468](https://github.com/servo/servo/issues/9468)
//!
//! To this end, this crate is designed to panic only when preconditions are not
//! met or under very pathologic circumstances that would cause winit to panic.
//!
//! # Unimplemented features
//!
//!  - Almost everything!
//!  - Support for platforms other than: macOS
//!  - Multi-threaded rendering (`Send`-able `Surface`)
//!  - Color management - we'll try to stick to sRGB for now
//!
use std::ops::{Deref, DerefMut};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowId},
};

/// Configuration for a [`Surface`].
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub vsync: bool,
    /// The preferred number of swapchain images.
    pub image_count: usize,
    /// Specifies whether the surface is opaque or not.
    ///
    /// If `false` is specified, the content of the surface is blended over
    /// the content below the window. The alpha values are interpreted as
    /// pre-multiplied alpha. You also have to specify an appropriate window
    /// creation option such as `WindowBuilder::with_transparent(true)` and use
    /// a [pixel format](Format) having an alpha channel for this option to
    /// work.
    ///
    /// Defaults to `true`.
    pub opaque: bool,
}

impl Config {
    /// Construct a default `Config`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vsync: true,
            image_count: 2,
            opaque: true,
        }
    }
}

/// Specifies a pixel format.
///
/// A backend may support only a subset of these formats. For each platform,
/// formats marked with **mandatory** are always supporterd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// 32-bit ARGB format.
    ///
    ///  - Wayland `argb8888` (`0`) (**mandatory**)
    ///  - Windows (**mandatory**)
    ///
    Argb8888,

    /// 32-bit RGB format.
    ///
    ///  - Wayland `xrgb8888` (`1`) (**mandatory**)
    ///
    Xrgb8888,
}

/// Describes the format of a swapchain image.
///
/// A swapchain image is a row-major top-down bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageInfo {
    /// The image size (`[width, height]`), measured in bytes.
    pub extent: [u32; 2],
    /// The offset between rows, measured in bytes.
    pub stride: usize,
    /// The pixel format.
    pub format: Format,
}

impl Default for ImageInfo {
    fn default() -> Self {
        Self {
            extent: [0, 0],
            stride: 0,
            format: Format::Argb8888,
        }
    }
}

/// A software-rendered window.
///
/// This is a safe wrapper around [`Surface`] and [`winit::window::Window`].
/// For each method, only a synopsis is provided here. See `Surface`'s
/// documentation for a full documentation.
#[derive(Debug)]
pub struct SwWindow {
    surface: Option<Surface>,
    window: Option<Window>,
}

impl SwWindow {
    /// Construct a `SwWindow` by wrapping an existing `Window`.
    pub fn new(window: Window, context: &Context, config: &Config) -> Self {
        Self {
            surface: Some(unsafe { Surface::new(&window, context, config) }),
            window: Some(window),
        }
    }

    /// Detach the surface and get the wrapped [`winit::window::Window`].
    pub fn into_window(mut self) -> Window {
        // Deconstruct the surface first
        drop(self.surface.take());

        self.window.take().unwrap()
    }

    /// Split the `Window` apart from the `Surface`.
    ///
    /// **Unsafety:** The `Surface` must be dropped before the `Window`.
    pub unsafe fn split(mut self) -> (Surface, Window) {
        (self.surface.take().unwrap(), self.window.take().unwrap())
    }

    /// Get a reference to the wrapped [`winit::window::Window`].
    pub fn window(&self) -> &Window {
        self.window.as_ref().unwrap()
    }

    /// Update the properties of the surface.
    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        self.surface
            .as_ref()
            .unwrap()
            .update_surface(extent, format);
    }

    /// Update the properties of the surface. The surface size is automatically
    /// derived based on the window size.
    pub fn update_surface_to_fit(&self, format: Format) {
        self.surface
            .as_ref()
            .unwrap()
            .update_surface_to_fit(self.window.as_ref().unwrap(), format);
    }

    /// Enumerate supported pixel formats.
    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        self.surface.as_ref().unwrap().supported_formats()
    }

    /// Get the `ImageInfo` describing the current swapchain images.
    pub fn image_info(&self) -> ImageInfo {
        self.surface.as_ref().unwrap().image_info()
    }

    /// Get the number of swapchain images.
    pub fn num_images(&self) -> usize {
        self.surface.as_ref().unwrap().num_images()
    }

    /// Get a flag indicating whether swapchain images preserve their contents
    /// when their indices are used again.
    pub fn does_preserve_image(&self) -> bool {
        self.surface.as_ref().unwrap().does_preserve_image()
    }

    /// Get the index of the next available swapchain image. Blocks the current
    /// thread.
    pub fn poll_next_image(&self) -> Option<usize> {
        self.surface.as_ref().unwrap().poll_next_image()
    }

    /// Lock a swapchain image at index `i` to access its contents.
    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        self.surface.as_ref().unwrap().lock_image(i)
    }

    /// Enqueue the presentation of a swapchain image at index `i`.
    pub fn present_image(&self, i: usize) {
        self.surface.as_ref().unwrap().present_image(i)
    }
}

impl Drop for SwWindow {
    fn drop(&mut self) {
        // Deconstruct the surface first
        drop(self.surface.take());
    }
}

// --------------------------------------------------------------------------
// Backend implementations

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::SurfaceImpl;
#[cfg(target_os = "windows")]
type ContextImpl = NullContextImpl;

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod cglffi;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod objcutils;

#[cfg(target_os = "macos")]
mod cgl;
#[cfg(target_os = "macos")]
use self::cgl::SurfaceImpl;
#[cfg(target_os = "macos")]
type ContextImpl = NullContextImpl;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod unix;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use self::unix::{ContextImpl, SurfaceImpl};

// --------------------------------------------------------------------------

#[allow(dead_code)]
pub struct ContextBuilder<'a, T: 'static> {
    event_loop: &'a EventLoop<T>,
    ready_cb: ReadyCb,
}

type ReadyCb = Box<dyn Fn(WindowId)>;

impl<'a, T: 'static> ContextBuilder<'a, T> {
    /// Construct a `ContextBuilder`.
    pub fn new(event_loop: &'a EventLoop<T>) -> Self {
        Self {
            event_loop,
            ready_cb: Box::new(|_| {}),
        }
    }

    /// Specify the function to be called when a swapchain image becomes
    /// available.
    pub fn with_ready_cb(self, cb: impl Fn(WindowId) + 'static) -> Self {
        if ContextImpl::TAKES_READY_CB {
            Self {
                ready_cb: Box::new(cb),
                ..self
            }
        } else {
            self
        }
    }

    /// Build a `Context`.
    pub fn build(self) -> Context {
        Context {
            inner: ContextImpl::new(self),
        }
    }
}

/// The global data for [`Surface`], constructed using [`ContextBuilder`].
#[derive(Debug)]
pub struct Context {
    inner: ContextImpl,
}

/// For backends that don't require `ContextImpl`, this type is aliased as
/// `ContextImpl`.
#[allow(dead_code)]
#[derive(Debug)]
struct NullContextImpl;

#[allow(dead_code)]
impl NullContextImpl {
    const TAKES_READY_CB: bool = false;

    fn new<T: 'static>(_: ContextBuilder<'_, T>) -> Self {
        Self {}
    }
}

// --------------------------------------------------------------------------

/// A software-rendered surface that is implicitly associated with the
/// underlying window (like `glutin::RawContext`).
#[derive(Debug)]
pub struct Surface {
    inner: SurfaceImpl,
}

impl Surface {
    /// Construct and attach a surface to the specified window.
    ///
    /// **Unsafety:** The constructed `Surface` must be dropped before `window`.
    pub unsafe fn new(window: &Window, context: &Context, config: &Config) -> Self {
        Self {
            inner: SurfaceImpl::new(window, &context.inner, config),
        }
    }

    /// Update the properties of the surface.
    ///
    /// After resizing a window, you must call this method irregardless of
    /// whether you want to change the image size or not. Also, you must call
    /// this method at least once before calling other methods.
    ///
    /// The result of a mismatching image size is implementation-dependent.
    /// In general, you should use `update_surface_to_fit`.
    ///
    /// Panics if:
    ///  - `format` is not in `supported_formats()`.
    ///  - One of `extent`'s elements is zero.
    ///  - One or more swapchain images are locked.
    pub fn update_surface(&self, extent: [u32; 2], format: Format) {
        self.inner.update_surface(extent, format);
    }

    /// Update the properties of the surface. The surface size is automatically
    /// derived based on the window size.
    ///
    /// This internally calls `update_surface`.
    pub fn update_surface_to_fit(&self, window: &Window, format: Format) {
        let (size_w, size_h) = window
            .inner_size()
            .to_physical(window.hidpi_factor())
            .into();

        self.update_surface([size_w, size_h], format);
    }

    /// Enumerate supported pixel formats.
    pub fn supported_formats(&self) -> impl Iterator<Item = Format> + '_ {
        self.inner.supported_formats()
    }

    /// Get the `ImageInfo` describing the current swapchain images.
    pub fn image_info(&self) -> ImageInfo {
        self.inner.image_info()
    }

    /// Get the number of swapchain images.
    ///
    /// This value is automatically calculated when `update_surface` is called.
    ///
    /// This value does not reflect the actual number of buffers that stand
    /// between the application and the display hardware. It's only useful
    /// when `does_preserve_image() == true` and the application wants to
    /// track dirty regions in each swapchain image.
    pub fn num_images(&self) -> usize {
        self.inner.num_images()
    }

    /// Get a flag indicating whether swapchain images preserve their contents
    /// when their indices are used again.
    ///
    /// If this function returns `true`, the application can optimize rendering
    /// by only updating the dirty portions.
    pub fn does_preserve_image(&self) -> bool {
        self.inner.does_preserve_image()
    }

    /// Get the index of the next available swapchain image.
    ///
    /// Returns `None` if no image is available. In this case, the function
    /// specified via [`ContextBuilder::with_ready_cb`] will be called when one
    /// is ready. If you call `poll_next_image` for multiple times before the
    /// callback function is called for the next time, and all of the calls to
    /// `poll_next_image` returns `None`, then the callback function will be
    /// called only once.
    ///
    /// `update_surface` may or may not cancel the deferred call to the
    /// callback.
    ///
    /// If an image is returned, this method does not remove the image from
    /// the set of avilable images. For example, if the application calls
    /// `poll_next_image` repeatedly, it may return the same image index for
    /// all of the calls.
    pub fn poll_next_image(&self) -> Option<usize> {
        self.inner.poll_next_image()
    }

    /// Lock a swapchain image at index `i` to access its contents.
    ///
    /// `i` must be the index of a swapchain image acquired by `poll_next_image`.
    ///
    /// Panics if the image is currently locked or not ready to be accessed by
    /// the application.
    ///
    /// Given an `ImageInfo`, the length is calculated as:
    /// `extent[1] * stride * 4`.
    pub fn lock_image(&self, i: usize) -> impl Deref<Target = [u8]> + DerefMut + '_ {
        self.inner.lock_image(i)
    }

    /// Enqueue the presentation of a swapchain image at index `i`.
    ///
    /// This method removes the swapchain image at index `i` from the set of
    /// available images and enqueues it for presentation.
    ///
    /// `i` must be the index of a swapchain image acquired by `poll_next_image`.
    /// The image must not be locked by `lock_image`.
    pub fn present_image(&self, i: usize) {
        self.inner.present_image(i)
    }
}
