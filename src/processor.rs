use crate::{AppState, QueryParams};
use anyhow::{Result, anyhow};
use libvips::{ops, Image, Interesting};
use std::sync::Arc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;
use serde_json::{json, Value};

#[derive(Debug)]
pub enum ImageSource {
    Url(String),
    S3(String),
}

pub enum ProcessedResult {
    Image(Vec<u8>, String),
    Json(Value),
}

pub async fn process_image(
    state: &Arc<AppState>,
    source: ImageSource,
    params: QueryParams,
    accept_header: &str,
) -> Result<ProcessedResult> {
    // 1. Signature Validation (if secret is set)
    if let Some(secret) = &state.secret {
        validate_signature(secret, &source, &params)?;
    }

    // 2. Fetch image bytes
    let bytes = fetch_bytes(state, &source).await?;

    // 3. Smart Animation Detection
    // By default, we load all pages (n=-1) to detect and preserve animations.
    // This is more robust for GIF/WebP/AVIF.
    let n_pages = params.n.unwrap_or(-1);
    
    let mut loader_options = format!("n={}", n_pages);
    if let Some(page) = params.page {
        loader_options.push_str(&format!(",page={}", page));
    }

    let img = Image::new_from_buffer(&bytes, &loader_options)
        .map_err(|e| anyhow!("Failed to load image: {:?}", e))?;

    // 4. Early return for JSON metadata
    if params.output.as_deref() == Some("json") {
        return Ok(ProcessedResult::Json(extract_metadata(&img)));
    }

    // 5. Transformation logic
    let mut processed = img;

    // Smart Crop / Resize
    if params.w.is_some() || params.h.is_some() {
        let n_pages = processed.get_n_pages();
        let current_w = processed.get_width();
        let current_h = processed.get_height() / n_pages; // Real height per frame

        let target_w = params.w.unwrap_or(0);
        let target_h = params.h.unwrap_or(0);

        if let Some(align_mode) = &params.a {
            let interesting = match align_mode.as_str() {
                "entropy" => Some(Interesting::Entropy),
                "attention" => Some(Interesting::Attention),
                _ => None,
            };

            if let Some(interesting) = interesting {
                 processed = processed.smartcrop(target_w, target_h, &ops::SmartcropOptions {
                    interesting,
                    ..Default::default()
                }).map_err(|e| anyhow!("Smartcrop failed: {:?}", e))?;
            } else {
                let scale_w = if target_w > 0 { target_w as f64 / current_w as f64 } else { 1.0 };
                let scale_h = if target_h > 0 { target_h as f64 / current_h as f64 } else { 1.0 };
                let final_scale = scale_w.min(scale_h);
                processed = ops::resize(&processed, final_scale).map_err(|e| anyhow!("Resize failed: {:?}", e))?;
            }
        } else {
             let scale_w = if target_w > 0 { target_w as f64 / current_w as f64 } else { 1.0 };
             let scale_h = if target_h > 0 { target_h as f64 / current_h as f64 } else { 1.0 };
             let mut final_scale = scale_w.min(scale_h);

             if params.we.unwrap_or(false) && final_scale > 1.0 {
                 final_scale = 1.0;
             }
             processed = ops::resize(&processed, final_scale).map_err(|e| anyhow!("Resize failed: {:?}", e))?;
        }
    }

    // Sharpen / Blur
    if let Some(sigma) = params.sharp {
        processed = ops::sharpen(&processed, &ops::SharpenOptions { sigma, ..Default::default() })?;
    }
    if let Some(sigma) = params.blur {
        processed = ops::gaussblur(&processed, sigma, &ops::GaussblurOptions { ..Default::default() })?;
    }

    // 6. Overlay (Branding)
    if let Some(overlay_src) = &params.overlay {
        let overlay_source = if overlay_src.starts_with("http") {
             ImageSource::Url(overlay_src.clone())
        } else {
             ImageSource::S3(overlay_src.clone())
        };
        
        let overlay_bytes = fetch_bytes(state, &overlay_source).await?;
        let overlay_img = Image::new_from_buffer(&overlay_bytes, "")?;
        
        // Composite overlay onto base at ox, oy
        let x = params.ox.unwrap_or(0);
        let y = params.oy.unwrap_or(0);
        
        // Stacking images: [base, overlay]
        let images = vec![processed, overlay_img];
        let modes = vec![libvips::VipsBlendMode::Over];
        let xs = vec![x];
        let ys = vec![y];
        
        processed = ops::composite(&images, &modes, &xs, &ys)?;
    }

    // 7. Masking (Circle/Ellipse/Path)
    if let Some(mask_type) = &params.mask {
        processed = apply_mask(processed, mask_type, params.d.as_deref())?;
    }

    // 8. Determine output format and encode
    let target_format = params.output.as_deref().unwrap_or_else(|| {
        if accept_header.contains("image/avif") { "avif" }
        else if accept_header.contains("image/webp") { "webp" }
        else { "jpeg" }
    });

    let quality = params.q.unwrap_or(80);

    let (buffer, mime) = match target_format {
        "avif" => {
            let buf = ops::heifsave_buffer(&processed, &ops::HeifsaveOptions {
                q: quality,
                compression: libvips::ForeignHeifCompression::Av1,
                ..Default::default()
            })?;
            (buf, "image/avif".to_string())
        }
        "webp" => {
            let buf = ops::webpsave_buffer(&processed, &ops::WebpsaveOptions { q: quality, ..Default::default() })?;
            (buf, "image/webp".to_string())
        }
        "png" => {
            let buf = ops::pngsave_buffer(&processed, &ops::PngsaveOptions { ..Default::default() })?;
            (buf, "image/png".to_string())
        }
        "gif" => {
            let buf = ops::gifsave_buffer(&processed, &ops::GifsaveOptions { ..Default::default() })?;
            (buf, "image/gif".to_string())
        }
        "jxl" => {
             // JXL support depends on libvips build, usually jxl_save_buffer or heifsave with jxl
             let buf = ops::heifsave_buffer(&processed, &ops::HeifsaveOptions {
                q: quality,
                compression: libvips::ForeignHeifCompression::Hevc, // Placeholder mapping
                ..Default::default()
            })?;
            (buf, "image/jxl".to_string())
        }
        _ => {
            let buf = ops::jpegsave_buffer(&processed, &ops::JpegsaveOptions { q: quality, ..Default::default() })?;
            (buf, "image/jpeg".to_string())
        }
    };

    Ok(ProcessedResult::Image(buffer, mime))
}

async fn fetch_bytes(state: &Arc<AppState>, source: &ImageSource) -> Result<Vec<u8>> {
    match source {
        ImageSource::Url(url) => {
            let resp = reqwest::get(url).await?;
            if !resp.status().is_success() {
                return Err(anyhow!("Failed to fetch: {}", resp.status()));
            }
            Ok(resp.bytes().await?.to_vec())
        }
        ImageSource::S3(path) => {
            let output = state.s3_client.get_object().bucket(&state.bucket).key(path).send().await?;
            Ok(output.body.collect().await?.to_vec())
        }
    }
}

fn extract_metadata(img: &Image) -> Value {
    let n_pages = img.get_n_pages();
    json!({
        "status": "success",
        "data": {
            "format": "vips-internal",
            "width": img.get_width(),
            "height": img.get_height() / n_pages, // Height per frame
            "isAnimated": n_pages > 1,
            "frameCount": n_pages,
            "hasAlpha": img.get_bands() == 4 || img.get_bands() == 2,
            "space": format!("{:?}", img.get_interpretation()),
            "channels": img.get_bands(),
        }
    })
}

fn validate_signature(secret: &str, source: &ImageSource, params: &QueryParams) -> Result<()> {
    let sig = params.sig.as_deref().ok_or_else(|| anyhow!("Missing signature"))?;
    let path = match source {
        ImageSource::Url(url) => format!("/url/{}", url),
        ImageSource::S3(p) => format!("/{}", p),
    };

    let mut query_parts = Vec::new();
    if let Some(w) = params.w { query_parts.push(format!("w={}", w)); }
    if let Some(h) = params.h { query_parts.push(format!("h={}", h)); }
    // ... add all other params for full coverage if needed
    query_parts.sort();
    let data = format!("{}?{}", path, query_parts.join("&"));
    
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(data.as_bytes());
    if sig != hex::encode(mac.finalize().into_bytes()) {
        return Err(anyhow!("Invalid signature"));
    }
    Ok(())
}

fn apply_mask(img: Image, mask_type: &str, path_data: Option<&str>) -> Result<Image> {
    let width = img.get_width();
    let n_pages = img.get_n_pages();
    let height = img.get_height() / n_pages; // Height per frame

    let svg = match mask_type {
        "circle" => {
            let r = (width.min(height) as f64) / 2.0;
            format!(
                r#"<svg width="{}" height="{}"><circle cx="{}" cy="{}" r="{}" fill="white"/></svg>"#,
                width, height, (width as f64) / 2.0, (height as f64) / 2.0, r
            )
        }
        "ellipse" => {
            let rx = (width as f64) / 2.0;
            let ry = (height as f64) / 2.0;
            format!(
                r#"<svg width="{}" height="{}"><ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="white"/></svg>"#,
                width, height, rx, ry, rx, ry
            )
        }
        "path" => {
            let d = path_data.unwrap_or("");
            format!(
                r#"<svg width="{}" height="{}"><path d="{}" fill="white"/></svg>"#,
                width, height, d
            )
        }
        _ => return Ok(img),
    };

    let mask = Image::new_from_buffer(svg.as_bytes(), "")
        .map_err(|e| anyhow!("Failed to load SVG mask: {:?}", e))?;
    
    // Ensure mask is 1-band
    let mask = if mask.get_bands() > 1 {
        ops::extract_band(&mask, 0, &ops::ExtractBandOptions { n: 1 })?
    } else {
        mask
    };
    
    // If animated, we need to replicate the mask for all frames
    let full_mask = if n_pages > 1 {
        let mut frames = Vec::new();
        for _ in 0..n_pages {
            frames.push(mask.clone());
        }
        ops::arrayjoin(&frames, &ops::ArrayjoinOptions { across: 1, ..Default::default() })?
    } else {
        mask
    };

    // If base doesn't have alpha, add it
    let bands = img.get_bands();
    let mut img_with_alpha = if bands == 3 || bands == 1 {
        // Add opaque alpha
        let white_alpha = (ops::black(img.get_width(), img.get_height())? + 255.0);
        ops::bandjoin(&vec![img, white_alpha])?
    } else {
        img
    };
    
    // Replace alpha channel with our mask
    let current_bands = img_with_alpha.get_bands();
    let color_bands = ops::extract_band(&img_with_alpha, 0, &ops::ExtractBandOptions { n: (current_bands - 1) as i32 })?;
    
    ops::bandjoin(&vec![color_bands, full_mask]).map_err(|e| anyhow!("Failed to apply mask: {:?}", e))
}
