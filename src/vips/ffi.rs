//! Raw FFI bindings to libvips 8.x.
//!
//! Only the functions required by this image processor are declared here.
//! All types are opaque C pointers; safe wrappers live in the parent module.
#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_double, c_int, c_void};

// ── Primitive type aliases ────────────────────────────────────────────────────

/// Opaque libvips VipsImage struct pointer.
pub type VipsImage = c_void;
pub type gboolean = c_int;
pub type gsize = usize;

// ── Enum constants ────────────────────────────────────────────────────────────

// VipsInteresting
pub const VIPS_INTERESTING_NONE: c_int = 0;
pub const VIPS_INTERESTING_CENTRE: c_int = 1;
pub const VIPS_INTERESTING_ENTROPY: c_int = 2;
pub const VIPS_INTERESTING_ATTENTION: c_int = 3;
pub const VIPS_INTERESTING_LOW: c_int = 4;
pub const VIPS_INTERESTING_HIGH: c_int = 5;
pub const VIPS_INTERESTING_ALL: c_int = 6;

// VipsBlendMode — only OVER is used here.
pub const VIPS_BLEND_MODE_OVER: c_int = 2;

// VipsForeignHeifCompression
pub const VIPS_FOREIGN_HEIF_COMPRESSION_HEVC: c_int = 1;
pub const VIPS_FOREIGN_HEIF_COMPRESSION_AVC: c_int = 2;
pub const VIPS_FOREIGN_HEIF_COMPRESSION_JPEG: c_int = 3;
pub const VIPS_FOREIGN_HEIF_COMPRESSION_AV1: c_int = 4;

// ── Extern "C" declarations ───────────────────────────────────────────────────

extern "C" {
    // Error handling
    /// Returns the last error message from libvips (thread-local).
    pub fn vips_error_buffer() -> *const c_char;
    /// Clears the libvips error buffer.
    pub fn vips_error_clear();

    // Lifecycle
    /// Initialises libvips — call once at program startup.
    pub fn vips_init(argv0: *const c_char) -> c_int;
    /// Sets the number of concurrent worker threads.
    pub fn vips_concurrency_set(concurrency: c_int);
    /// Sets the maximum number of operations to keep in the vips operation cache.
    pub fn vips_cache_set_max(max: c_int);
    /// Sets the maximum amount of memory (in bytes) used by the vips operation cache.
    pub fn vips_cache_set_max_mem(max_mem: gsize);
    /// Sets the maximum number of open files kept in the vips operation cache.
    pub fn vips_cache_set_max_files(max_files: c_int);
    /// Decrements a GObject reference count (used to free VipsImage).
    pub fn g_object_unref(obj: *mut c_void);

    // Image construction
    /// Loads an image from a memory buffer with a loader option string (e.g. `"n=-1,page=0"`).
    pub fn vips_image_new_from_buffer(
        buf: *const c_void,
        len: gsize,
        option_string: *const c_char,
        // NULL-terminated varargs — we always pass a single NULL sentinel.
        ...
    ) -> *mut VipsImage;

    /// Creates a new image with the same size/format as `image` filled with constant `c`.
    pub fn vips_image_new_from_image1(image: *mut VipsImage, c: c_double) -> *mut VipsImage;

    // Image metadata
    pub fn vips_image_get_width(image: *const VipsImage) -> c_int;
    pub fn vips_image_get_height(image: *const VipsImage) -> c_int;
    pub fn vips_image_get_bands(image: *const VipsImage) -> c_int;

    /// Reads an integer metadata property (e.g. `"n-pages"`).
    pub fn vips_image_get_int(
        image: *const VipsImage,
        name: *const c_char,
        out: *mut c_int,
    ) -> c_int;

    /// Writes an integer metadata property (e.g. `"page-height"`).
    pub fn vips_image_set_int(
        image: *mut VipsImage,
        name: *const c_char,
        value: c_int,
    );
    
    /// Reads a string metadata property (e.g. `"vips-loader"`).
    pub fn vips_image_get_string(
        image: *const VipsImage,
        name: *const c_char,
        out: *mut *const c_char,
    ) -> c_int;

    /// Reads a double metadata property (e.g. `"xres"`).
    pub fn vips_image_get_double(
        image: *const VipsImage,
        name: *const c_char,
        out: *mut c_double,
    ) -> c_int;

    /// Returns the VipsInterpretation enum value for the image's colorspace.
    pub fn vips_image_get_interpretation(image: *const VipsImage) -> c_int;

    // Save to buffer
    /// Encodes to JPEG buffer. Varargs: `"Q", quality, NULL`.
    pub fn vips_jpegsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    /// Encodes to WebP buffer. Varargs: `"Q", quality, NULL`.
    pub fn vips_webpsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    /// Encodes to PNG buffer (lossless).
    pub fn vips_pngsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    /// Encodes to HEIF/AVIF buffer.
    pub fn vips_heifsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    /// Encodes to GIF buffer.
    pub fn vips_gifsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    /// Encodes to JPEG XL buffer.
    pub fn vips_jxlsave_buffer(
        in_: *mut VipsImage,
        buf: *mut *mut c_void,
        len: *mut gsize,
        ...
    ) -> c_int;

    // Transformations
    /// Resizes by a scale factor. Varargs: `"vscale", factor, NULL` or just `NULL`.
    pub fn vips_resize(
        in_: *mut VipsImage,
        out: *mut *mut VipsImage,
        scale: c_double,
        ...
    ) -> c_int;

    /// Smart-crops to the target size using the given interest strategy.
    pub fn vips_smartcrop(
        in_: *mut VipsImage,
        out: *mut *mut VipsImage,
        width: c_int,
        height: c_int,
        ...
    ) -> c_int;

    /// Extracts a rectangular region from an image.
    pub fn vips_extract_area(
        in_: *mut VipsImage,
        out: *mut *mut VipsImage,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        ...
    ) -> c_int;

    /// Gaussian blur.
    pub fn vips_gaussblur(
        in_: *mut VipsImage,
        out: *mut *mut VipsImage,
        sigma: c_double,
        ...
    ) -> c_int;

    /// Unsharp-mask sharpen.
    pub fn vips_sharpen(in_: *mut VipsImage, out: *mut *mut VipsImage, ...) -> c_int;

    /// Composites two images with a blend mode.
    pub fn vips_composite2(
        base: *mut VipsImage,
        overlay: *mut VipsImage,
        out: *mut *mut VipsImage,
        mode: c_int, // VipsBlendMode
        ...
    ) -> c_int;

    /// Joins an array of images vertically (used for assembling animation frames).
    pub fn vips_arrayjoin(
        in_: *mut *mut VipsImage,
        out: *mut *mut VipsImage,
        n: c_int,
        ...
    ) -> c_int;

    /// Joins two images band-by-band.
    pub fn vips_bandjoin2(
        in1: *mut VipsImage,
        in2: *mut VipsImage,
        out: *mut *mut VipsImage,
        ...
    ) -> c_int;

    /// Extracts `n` bands starting at `band`.
    pub fn vips_extract_band(
        in_: *mut VipsImage,
        out: *mut *mut VipsImage,
        band: c_int,
        ...
    ) -> c_int;

    // Memory management
    /// Frees a buffer allocated by libvips.
    pub fn g_free(mem: *mut c_void);
}
