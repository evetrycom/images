use std::ffi::CString;

use super::ffi;
use super::error::take_error;

// ── VipsImage (RAII smart pointer) ────────────────────────────────────────────

/// Safe RAII wrapper around `*mut ffi::VipsImage`.
///
/// Automatically calls `g_object_unref` on drop.
pub struct VipsImage {
    ptr: *mut ffi::VipsImage,
}

impl VipsImage {
    /// Takes ownership of `ptr`.
    ///
    /// # Safety
    /// `ptr` must be a valid, caller-owned pointer to a VipsImage.
    pub(crate) fn from_raw(ptr: *mut ffi::VipsImage) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::VipsImage {
        self.ptr
    }

    // ── Metadata ──────────────────────────────────────────────────────────────

    pub fn width(&self) -> i32 {
        unsafe { ffi::vips_image_get_width(self.ptr) }
    }

    pub fn height(&self) -> i32 {
        unsafe { ffi::vips_image_get_height(self.ptr) }
    }

    pub fn bands(&self) -> i32 {
        unsafe { ffi::vips_image_get_bands(self.ptr) }
    }

    /// Number of pages / animation frames. Returns at least 1.
    pub fn n_pages(&self) -> i32 {
        let name = CString::new("n-pages").unwrap();
        unsafe {
            // Check if the property exists first to avoid polluting the vips error buffer.
            if ffi::vips_image_get_typeof(self.ptr, name.as_ptr()) != 0 {
                let mut val: std::ffi::c_int = 1;
                if ffi::vips_image_get_int(self.ptr, name.as_ptr(), &mut val) == 0 {
                    return val.max(1);
                }
            }
        }
        1
    }

    /// Returns the name of the loader that created this image (e.g. "jpegload").
    pub fn loader(&self) -> String {
        let name = CString::new("vips-loader").unwrap();
        let mut ptr: *const std::ffi::c_char = std::ptr::null();
        unsafe {
            if ffi::vips_image_get_string(self.ptr, name.as_ptr(), &mut ptr) == 0 && !ptr.is_null() {
                return std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
            }
        }
        "unknown".to_string()
    }

    /// Human-readable name of the image's colorspace interpretation.
    pub fn interpretation(&self) -> String {
        let v = unsafe { ffi::vips_image_get_interpretation(self.ptr) };
        match v {
            0 => "ERROR",
            2 => "MULTIBAND",
            3 => "B_W",
            6 => "HISTOGRAM",
            8 => "XYZ",
            9 => "LAB",
            10 => "CMYK",
            11 => "LABQ",
            12 => "RGB",
            13 => "CMC",
            14 => "LCH",
            15 => "LABS",
            16 => "sRGB",
            17 => "YXY",
            18 => "FOURIER",
            19 => "RGB16",
            20 => "GREY16",
            21 => "MATRIX",
            22 => "scRGB",
            23 => "HSV",
            _ => return format!("Unknown({})", v),
        }
        .to_string()
    }

    /// Increments the GObject refcount and returns a second owner.
    ///
    /// Use sparingly — only needed when an array of raw pointers must outlive
    /// individual `VipsImage` values (e.g. `arrayjoin` for animated frames).
    pub fn add_ref(&self) -> VipsImage {
        extern "C" {
            fn g_object_ref(obj: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }
        unsafe {
            g_object_ref(self.ptr);
        }
        VipsImage { ptr: self.ptr }
    }
}

impl Drop for VipsImage {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::g_object_unref(self.ptr) }
        }
    }
}

// ── VipsApp (lifecycle guard) ─────────────────────────────────────────────────

/// Represents an active libvips session. Must live for the entire program.
pub struct VipsApp;

impl VipsApp {
    /// Initialises libvips. Call exactly once at the start of `main`.
    pub fn new(name: &str) -> Result<Self, String> {
        let name_c = CString::new(name).map_err(|e| e.to_string())?;
        let ret = unsafe { ffi::vips_init(name_c.as_ptr()) };
        if ret != 0 {
            Err(take_error())
        } else {
            Ok(VipsApp)
        }
    }

    /// Sets the number of concurrent vips worker threads.
    pub fn set_concurrency(&self, n: i32) {
        unsafe { ffi::vips_concurrency_set(n) }
    }

    /// Sets the vips operation cache size (number of operations).
    pub fn set_cache_max(&self, n: i32) {
        unsafe { ffi::vips_cache_set_max(n) }
    }

    /// Sets the maximum cumulative memory (in bytes) used by the operation cache.
    pub fn set_cache_max_mem(&self, n: usize) {
        unsafe { ffi::vips_cache_set_max_mem(n) }
    }

    /// Sets the maximum number of open files kept in the operation cache.
    pub fn set_cache_max_files(&self, n: i32) {
        unsafe { ffi::vips_cache_set_max_files(n) }
    }
}
