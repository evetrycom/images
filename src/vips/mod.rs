//! Custom libvips 8.x bindings for the Evetry image processor.
//!
//! Module layout:
//!   ffi    — raw `extern "C"` declarations
//!   error  — internal error helper (`take_error`)
//!   enums  — public enums (`Interesting`, `HeifCompression`)
//!   image  — `VipsImage` RAII wrapper + `VipsApp` lifecycle guard
//!   ops    — safe Rust wrappers for all vips operations
mod error;
pub mod ffi;
mod enums;
mod image;
mod ops;

// Re-export the public API surface so callers only need `use crate::vips::*`.
pub use enums::{HeifCompression, Interesting};
pub use image::{VipsApp, VipsImage};
pub use ops::{
    arrayjoin, bandjoin2, composite2, extract_area, extract_band, gaussblur, gifsave_buffer,
    heifsave_buffer, image_from_buffer, image_new_from_image1, jpegsave_buffer, jxlsave_buffer,
    pngsave_buffer, resize, set_page_height, sharpen, smartcrop, webpsave_buffer,
};
