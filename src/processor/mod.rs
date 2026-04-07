mod encode;
mod fetch;
mod mask;
mod signature;
mod source;
mod transform;

pub use source::ImageSource;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::vips::{self, image_from_buffer};
use crate::{AppState, QueryParams};

// ── Public result type ────────────────────────────────────────────────────────

/// The output of a successful `process_image` call.
pub enum ProcessedResult {
    /// Encoded image bytes together with the MIME type string.
    Image(Vec<u8>, String),
    /// JSON metadata response (when `output=json`).
    Json(Value),
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Orchestrates the full image-processing pipeline:
///
/// 1. Validates the optional HMAC signature.
/// 2. Fetches the source bytes (URL or S3).
/// 3. Loads the image via libvips.
/// 4. Optionally returns JSON metadata early.
/// 5. Applies resize / crop → filters → overlay → mask transformations.
/// 6. Encodes to the negotiated output format.
pub async fn process_image(
    state: &Arc<AppState>,
    source: ImageSource,
    params: QueryParams,
    accept_header: &str,
) -> Result<ProcessedResult> {
    // 1. Signature validation (skipped when no secret is configured).
    if let Some(secret) = &state.secret {
        signature::validate_signature(secret, &source, &params)?;
    }

    // 2. Fetch raw bytes for main image and overlay.
    let bytes = fetch::fetch_bytes(state, &source).await?;
    
    let overlay_bytes = if let Some(overlay_src) = &params.overlay {
        let overlay_source = if overlay_src.starts_with("http") {
            ImageSource::Url(overlay_src.to_string())
        } else {
            ImageSource::S3(overlay_src.to_string())
        };
        Some(fetch::fetch_bytes(state, &overlay_source).await?)
    } else {
        None
    };

    // 3. Load image — request all pages by default to preserve animations.
    let n_pages_param = params.n.unwrap_or(-1);
    let mut loader_opts = format!("n={}", n_pages_param);
    if let Some(page) = params.page {
        loader_opts.push_str(&format!(",page={}", page));
    }

    let img = image_from_buffer(&bytes, &loader_opts)
        .map_err(|e| anyhow!("Failed to load image: {}", e))?;

    // 4. Early return for JSON metadata.
    if params.output.as_deref() == Some("json") {
        return Ok(ProcessedResult::Json(extract_metadata(img)));
    }

    // 5. Transformations.
    let mut processed = img;

    if params.w.is_some() || params.h.is_some() {
        processed = transform::apply_resize(processed, &params)?;
    }

    processed = transform::apply_filters(processed, &params)?;

    // Overlay / watermark.
    if let Some(ob) = overlay_bytes {
        processed = apply_overlay(processed, &ob, &params)?;
    }

    // Mask.
    if let Some(mask_type) = &params.mask {
        processed = mask::apply_mask(processed, mask_type, params.d.as_deref())
            .map_err(|e| anyhow!("Mask failed: {}", e))?;
    }

    // 6. Encode.
    let fmt = encode::negotiate_format(params.output.as_deref(), accept_header);
    let quality = params.q.unwrap_or(80);
    let (buffer, mime) = encode::encode(&processed, fmt, quality)?;

    Ok(ProcessedResult::Image(buffer, mime))
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

fn extract_metadata(img: crate::vips::VipsImage) -> Value {
    let n_pages = img.n_pages();
    json!({
        "status": "success",
        "data": {
            "format": "vips-internal",
            "width": img.width(),
            "height": img.height() / n_pages,
            "isAnimated": n_pages > 1,
            "frameCount": n_pages,
            "hasAlpha": img.bands() == 4 || img.bands() == 2,
            "space": img.interpretation(),
            "channels": img.bands(),
        }
    })
}
