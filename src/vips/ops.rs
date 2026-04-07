/// Zero-cost C-string literal from a string literal (stack-allocated).
///
/// Appends a `\0` byte and casts the pointer — only valid for `'static` literals.
macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const std::ffi::c_char
    };
}

use std::ffi::CString;
use std::ptr;

use super::enums::{HeifCompression, Interesting};
use super::error::take_error;
use super::ffi;
use super::image::VipsImage;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Copies a libvips-allocated buffer into a `Vec<u8>`, then frees the original.
///
/// # Safety
/// `buf` must be a pointer returned by a vips save function and not yet freed.
unsafe fn vips_buf_to_vec(buf: *mut std::ffi::c_void, len: usize) -> Vec<u8> {
    if buf.is_null() {
        return Vec::new();
    }
    let slice = std::slice::from_raw_parts(buf as *const u8, len);
    let vec = slice.to_vec();
    ffi::g_free(buf);
    vec
}

// ── Load ──────────────────────────────────────────────────────────────────────

/// Loads an image from a byte slice with a loader option string (e.g. `"n=-1"`).
pub fn image_from_buffer(data: &[u8], options: &str) -> Result<VipsImage, String> {
    let opts = CString::new(options).map_err(|e| e.to_string())?;
    let ptr = unsafe {
        ffi::vips_image_new_from_buffer(
            data.as_ptr() as *const std::ffi::c_void,
            data.len(),
            opts.as_ptr(),
            ptr::null::<std::ffi::c_char>(), // varargs sentinel
        )
    };
    VipsImage::from_raw(ptr).ok_or_else(take_error)
}

/// Creates a new image matching the layout of `img`, filled with constant `c`.
///
/// Useful for creating a solid alpha channel before a `bandjoin2` call.
pub fn image_new_from_image1(img: &VipsImage, c: f64) -> Result<VipsImage, String> {
    let ptr = unsafe { ffi::vips_image_new_from_image1(img.as_ptr(), c) };
    VipsImage::from_raw(ptr).ok_or_else(take_error)
}

// ── Resize / Crop ─────────────────────────────────────────────────────────────

/// Resizes the image by a uniform scale factor (`1.0` = no change).
pub fn resize(img: &VipsImage, scale: f64) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_resize(img.as_ptr(), &mut out, scale, ptr::null::<std::ffi::c_char>())
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Smart-crops the image to the target size using the given interest strategy.
pub fn smartcrop(
    img: &VipsImage,
    width: i32,
    height: i32,
    interest: Interesting,
) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_smartcrop(
            img.as_ptr(),
            &mut out,
            width,
            height,
            cstr!("interesting"),
            interest.as_c_int(),
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

// ── Filters ───────────────────────────────────────────────────────────────────

/// Applies a Gaussian blur with the given sigma (in pixels).
pub fn gaussblur(img: &VipsImage, sigma: f64) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_gaussblur(img.as_ptr(), &mut out, sigma, ptr::null::<std::ffi::c_char>())
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Applies an unsharp-mask sharpen with the given sigma.
pub fn sharpen(img: &VipsImage, sigma: f64) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_sharpen(
            img.as_ptr(),
            &mut out,
            cstr!("sigma"),
            sigma,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

// ── Compositing ───────────────────────────────────────────────────────────────

/// Composites `overlay` on top of `base` using BLENDMODE_OVER at position `(x, y)`.
pub fn composite2(
    base: &VipsImage,
    overlay: &VipsImage,
    x: i32,
    y: i32,
) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_composite2(
            base.as_ptr(),
            overlay.as_ptr(),
            &mut out,
            ffi::VIPS_BLEND_MODE_OVER,
            cstr!("x"),
            x,
            cstr!("y"),
            y,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

// ── Band operations ───────────────────────────────────────────────────────────

/// Joins two images band-by-band (e.g. color image + alpha channel).
pub fn bandjoin2(img1: &VipsImage, img2: &VipsImage) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_bandjoin2(
            img1.as_ptr(),
            img2.as_ptr(),
            &mut out,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Extracts `n` bands starting at index `band`.
pub fn extract_band(img: &VipsImage, band: i32, n: i32) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_extract_band(
            img.as_ptr(),
            &mut out,
            band,
            cstr!("n"),
            n,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Joins an array of images vertically (used to assemble animation frames).
pub fn arrayjoin(images: &mut [VipsImage]) -> Result<VipsImage, String> {
    let mut ptrs: Vec<*mut ffi::VipsImage> = images.iter().map(|i| i.as_ptr()).collect();
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_arrayjoin(
            ptrs.as_mut_ptr(),
            &mut out,
            ptrs.len() as std::ffi::c_int,
            cstr!("across"),
            1i32,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Extracts a rectangular region from an image (used to slice individual animation frames).
pub fn extract_area(img: &VipsImage, left: i32, top: i32, width: i32, height: i32) -> Result<VipsImage, String> {
    let mut out: *mut ffi::VipsImage = ptr::null_mut();
    let ret = unsafe {
        ffi::vips_extract_area(
            img.as_ptr(),
            &mut out,
            left,
            top,
            width,
            height,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        VipsImage::from_raw(out).ok_or_else(take_error)
    }
}

/// Sets the `page-height` metadata on a multi-frame image so encoders treat it as animated.
pub fn set_page_height(img: &VipsImage, height: i32) {
    let name = std::ffi::CString::new("page-height").unwrap();
    unsafe { ffi::vips_image_set_int(img.as_ptr(), name.as_ptr(), height) };
}

// ── Save to buffer ────────────────────────────────────────────────────────────

/// Encodes the image to a JPEG buffer at the given quality (1–100).
pub fn jpegsave_buffer(img: &VipsImage, quality: i32) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_jpegsave_buffer(
            img.as_ptr(),
            &mut buf,
            &mut len,
            cstr!("Q"),
            quality,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}

/// Encodes the image to a WebP buffer at the given quality (1–100).
pub fn webpsave_buffer(img: &VipsImage, quality: i32) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_webpsave_buffer(
            img.as_ptr(),
            &mut buf,
            &mut len,
            cstr!("Q"),
            quality,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}

/// Encodes the image to a PNG buffer (lossless).
pub fn pngsave_buffer(img: &VipsImage) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_pngsave_buffer(img.as_ptr(), &mut buf, &mut len, ptr::null::<std::ffi::c_char>())
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}

/// Encodes the image to a HEIF/AVIF buffer with the specified compression codec.
pub fn heifsave_buffer(
    img: &VipsImage,
    quality: i32,
    compression: HeifCompression,
) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_heifsave_buffer(
            img.as_ptr(),
            &mut buf,
            &mut len,
            cstr!("Q"),
            quality,
            cstr!("compression"),
            compression.as_c_int(),
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}

/// Encodes the image to a GIF buffer.
pub fn gifsave_buffer(img: &VipsImage) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_gifsave_buffer(img.as_ptr(), &mut buf, &mut len, ptr::null::<std::ffi::c_char>())
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}

/// Encodes the image to a JPEG XL buffer at the given quality.
pub fn jxlsave_buffer(img: &VipsImage, quality: i32) -> Result<Vec<u8>, String> {
    let mut buf: *mut std::ffi::c_void = ptr::null_mut();
    let mut len: ffi::gsize = 0;
    let ret = unsafe {
        ffi::vips_jxlsave_buffer(
            img.as_ptr(),
            &mut buf,
            &mut len,
            cstr!("Q"),
            quality,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    if ret != 0 {
        Err(take_error())
    } else {
        Ok(unsafe { vips_buf_to_vec(buf, len) })
    }
}
