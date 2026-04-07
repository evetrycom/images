use anyhow::{anyhow, Result};

use crate::vips::{self, HeifCompression, VipsImage};

/// Encodes `img` to the requested format and returns the raw bytes + MIME type.
///
/// `format` is a lowercase string such as `"webp"`, `"avif"`, `"png"`, etc.
/// Falls back to JPEG for unrecognised values.
pub fn encode(img: &VipsImage, format: &str, quality: i32) -> Result<(Vec<u8>, String)> {
    let (buf, mime) = match format {
        "avif" => {
            let buf = vips::heifsave_buffer(img, quality, HeifCompression::Av1)
                .map_err(|e| anyhow!("AVIF encode failed: {}", e))?;
            (buf, "image/avif")
        }
        "webp" => {
            let buf = vips::webpsave_buffer(img, quality)
                .map_err(|e| anyhow!("WebP encode failed: {}", e))?;
            (buf, "image/webp")
        }
        "png" => {
            let buf =
                vips::pngsave_buffer(img).map_err(|e| anyhow!("PNG encode failed: {}", e))?;
            (buf, "image/png")
        }
        "gif" => {
            let buf =
                vips::gifsave_buffer(img).map_err(|e| anyhow!("GIF encode failed: {}", e))?;
            (buf, "image/gif")
        }
        "jxl" => {
            let buf = vips::jxlsave_buffer(img, quality)
                .map_err(|e| anyhow!("JXL encode failed: {}", e))?;
            (buf, "image/jxl")
        }
        _ => {
            let buf = vips::jpegsave_buffer(img, quality)
                .map_err(|e| anyhow!("JPEG encode failed: {}", e))?;
            (buf, "image/jpeg")
        }
    };
    Ok((buf, mime.to_string()))
}

/// Picks the output format from the explicit `output` param or the `Accept` header.
pub fn negotiate_format<'a>(output_param: Option<&'a str>, accept_header: &'a str) -> &'a str {
    output_param.unwrap_or_else(|| {
        if accept_header.contains("image/avif") {
            "avif"
        } else if accept_header.contains("image/webp") {
            "webp"
        } else {
            "jpeg"
        }
    })
}
