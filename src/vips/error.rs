use std::ffi::CStr;
use super::ffi;

/// Pops the last error string from libvips and clears the buffer.
pub(super) fn take_error() -> String {
    unsafe {
        let msg = ffi::vips_error_buffer();
        let s = if msg.is_null() {
            "unknown vips error".to_string()
        } else {
            CStr::from_ptr(msg).to_string_lossy().into_owned()
        };
        ffi::vips_error_clear();
        s
    }
}
