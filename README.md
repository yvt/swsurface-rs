# swsurface

[<img src="https://docs.rs/swsurface/badge.svg" alt="docs.rs">](https://docs.rs/swsurface/)

This crate provides a software-rendered surface for `winit`.

The goal of this crate is to provide a minimal drawing functionality
for every platform supported by `winit` even if the drawing APIs that we
usually assume are available, such as OpenGL¹, aren't available in the
target environment. This crate is also useful as a fallback when they are
available, but failed due to an unrecoverable error.

¹ [“Servo on Windows in VirtualBox gets 'NoAvailablePixelFormat'” servo/servo #9468](https://github.com/servo/servo/issues/9468)

To this end, this crate is designed to panic only when preconditions are not
met or under very pathologic circumstances that would cause winit to panic.

## Unimplemented features

 - Partial update - The application can redraw a portion, but `swsurface`
   always sends entire the windowfor now
 - Support for platforms other than: macOS, Windows, X11, Wayland
 - X11: Support for color depths other than 24
 - X11: Transparency
 - Multi-threaded rendering (`Send`-able `Surface`)
 - Color management - we'll try to stick to sRGB for now


License: MIT/Apache-2.0
