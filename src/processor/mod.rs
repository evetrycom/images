mod encode;
mod fetch;
mod mask;
mod signature;
mod source;
mod transform;

fn is_likely_multi_page(data: &[u8]) -> bool {
    // 1. GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return true;
    }
    // 2. WebP: RIFFxxxxWEBP
    if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return true;
    }
    // 3. HEIF/AVIF: ....ftypavif or ....ftypheic
    if data.len() > 12 && &data[4..8] == b"ftyp" {
        let ftyp = &data[8..12];
        if ftyp == b"avif" || ftyp == b"heic" || ftyp == b"hevc" {
            return true;
        }
    }
    // 4. TIFF: II* (little) or MM.* (big)
    if data.starts_with(b"II\x2a\x00") || data.starts_with(b"MM\x00\x2a") {
        return true;
    }
    false
}

fn parse_format_name(loader: &str) -> &str {
    // Vips loader names look like "jpegload", "pngload_buffer", etc.
    let base = loader.strip_suffix("_buffer").unwrap_or(loader);
    let base = base.strip_suffix("load").unwrap_or(base);
    
    // Normalize some common names
    match base {
        "heif" | "avif" => "avif",
        "webp" => "webp",
        "jpeg" | "jpg" => "jpeg",
        "png" => "png",
        "gif" => "gif",
        "svg" => "svg",
        "jxl" => "jxl",
        _ => base,
    }
}

pub use source::ImageSource;

use anyhow::{anyhow, Result};
use std::sync::Arc;
use serde_json::{json, Value};
use bytes::Bytes;

use crate::vips::{self, image_from_buffer};
use crate::{AppState, QueryParams};

// ── Public result type ────────────────────────────────────────────────────────

/// The output of a successful `process_image` call.
pub enum ProcessedResult {
    /// Encoded image bytes together with the MIME type string.
    Image(Bytes, String),
    /// JSON metadata response (when `output=json`).
    Json(Value),
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Orchestrates the full image-processing pipeline:
///
/// 1. Validates the optional HMAC signature.
/// 2. Fetches the source bytes (URL or S3) in parallel.
/// 3. Moves CPU-bound processing to tokio::task::spawn_blocking.
/// 4. Returns the result as Bytes or Value.
pub async fn process_image(
    state: &Arc<AppState>,
    source: ImageSource,
    params: QueryParams,
    accept_header: &str,
) -> Result<ProcessedResult> {
    // 1. Signature validation.
    if let Some(secret) = &state.secret {
        signature::validate_signature(secret, &source, &params)?;
    }

    // 2. Fetch raw bytes for main image and overlay in parallel.
    let (bytes, overlay_bytes) = {
        let state = state.clone();
        let main_source = source.clone();
        
        let overlay_source = params.overlay.as_ref().map(|src| {
            if src.starts_with("http") {
                ImageSource::Url(src.to_string())
            } else {
                ImageSource::S3(src.to_string())
            }
        });

        match overlay_source {
            Some(os) => {
                let (b1, b2) = tokio::try_join!(
                    fetch::fetch_bytes(state.clone(), main_source),
                    fetch::fetch_bytes(state, os)
                )?;
                (b1, Some(b2))
            }
            None => (fetch::fetch_bytes(state, main_source).await?, None),
        }
    };

    // 3. Move CPU-bound libvips processing to a blocking thread pool.
    let accept = accept_header.to_string();
    let result = tokio::task::spawn_blocking(move || {
        process_sync(bytes, overlay_bytes, params, &accept)
    }).await.map_err(|e| anyhow!("Blocking task failed: {}", e))??;

    Ok(result)
}

/// Synchronous part of the processing pipeline, runs in spawn_blocking.
fn process_sync(
    bytes: Bytes,
    overlay_bytes: Option<Bytes>,
    params: QueryParams,
    accept: &str,
) -> Result<ProcessedResult> {
    // A. Loader options.
    let mut opts = Vec::new();
    if let Some(n) = params.n {
        opts.push(format!("n={}", n));
    } else if is_likely_multi_page(&bytes) {
        opts.push("n=-1".to_string());
    }
    if let Some(page) = params.page {
        opts.push(format!("page={}", page));
    }
    let loader_opts = opts.join(",");

    // B. Load image.
    let img = image_from_buffer(&bytes, &loader_opts)
        .map_err(|e| anyhow!("Failed to load image: {}", e))?;

    // C. Early return for JSON.
    if params.output.as_deref() == Some("json") {
        return Ok(ProcessedResult::Json(extract_metadata(img, bytes.len())));
    }

    // D. Transformations.
    // E.encode.
    let loader = img.loader(); // Capture loader name before loop/moves
    let original_fmt = parse_format_name(&loader);

    let mut processed = img;

    if params.w.is_some() || params.h.is_some() {
        processed = transform::apply_resize(processed, &params)?;
    }

    processed = transform::apply_filters(processed, &params)?;

    // Overlay.
    if let Some(ob) = overlay_bytes {
        processed = apply_overlay(processed, &ob, &params)?;
    }

    // Mask.
    if let Some(mask_type) = &params.mask {
        processed = mask::apply_mask(processed, mask_type, params.d.as_deref())
            .map_err(|e| anyhow!("Mask failed: {}", e))?;
    }

    let fmt = encode::negotiate_format(params.output.as_deref(), accept, original_fmt);
    let quality = params.q.unwrap_or(80);
    let (buffer, mime) = encode::encode(&processed, fmt, quality)?;

    Ok(ProcessedResult::Image(Bytes::from(buffer), mime))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn apply_overlay(
    base: crate::vips::VipsImage,
    overlay_bytes: &[u8],
    params: &QueryParams,
) -> Result<crate::vips::VipsImage> {
    let overlay_img = image_from_buffer(overlay_bytes, "")
        .map_err(|e| anyhow!("Failed to load overlay: {}", e))?;

    let x = params.ox.unwrap_or(0);
    let y = params.oy.unwrap_or(0);

    vips::composite2(&base, &overlay_img, x, y)
        .map_err(|e| anyhow!("Composite failed: {}", e))
}

fn extract_metadata(img: crate::vips::VipsImage, original_size: usize) -> Value {
    let n_pages = img.n_pages();
    let loader = img.loader();
    let format = parse_format_name(&loader);

    json!({
        "status": "success",
        "data": {
            "format": format,
            "width": img.width(),
            "height": img.height() / n_pages,
            "isAnimated": n_pages > 1,
            "frameCount": n_pages,
            "hasAlpha": img.bands() == 4 || img.bands() == 2,
            "space": img.interpretation(),
            "channels": img.bands(),
            "fileSize": original_size,
        }
    })
}
