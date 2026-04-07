#![allow(dead_code)]
use super::ffi;

// ── Interesting ───────────────────────────────────────────────────────────────

/// Crop interest strategy passed to `vips_smartcrop`.
#[derive(Debug, Clone, Copy)]
pub enum Interesting {
    None,
    Centre,
    Entropy,
    Attention,
    Low,
    High,
    All,
}

impl Interesting {
    pub(super) fn as_c_int(self) -> std::ffi::c_int {
        match self {
            Self::None => ffi::VIPS_INTERESTING_NONE,
            Self::Centre => ffi::VIPS_INTERESTING_CENTRE,
            Self::Entropy => ffi::VIPS_INTERESTING_ENTROPY,
            Self::Attention => ffi::VIPS_INTERESTING_ATTENTION,
            Self::Low => ffi::VIPS_INTERESTING_LOW,
            Self::High => ffi::VIPS_INTERESTING_HIGH,
            Self::All => ffi::VIPS_INTERESTING_ALL,
        }
    }
}

// ── HeifCompression ───────────────────────────────────────────────────────────

/// Compression codec for HEIF/AVIF output.
#[derive(Debug, Clone, Copy)]
pub enum HeifCompression {
    Hevc,
    Avc,
    Jpeg,
    Av1,
}

impl HeifCompression {
    pub(super) fn as_c_int(self) -> std::ffi::c_int {
        match self {
            Self::Hevc => ffi::VIPS_FOREIGN_HEIF_COMPRESSION_HEVC,
            Self::Avc => ffi::VIPS_FOREIGN_HEIF_COMPRESSION_AVC,
            Self::Jpeg => ffi::VIPS_FOREIGN_HEIF_COMPRESSION_JPEG,
            Self::Av1 => ffi::VIPS_FOREIGN_HEIF_COMPRESSION_AV1,
        }
    }
}
