use anyhow::{anyhow, Result};

use crate::vips::{self, Interesting, VipsImage};
use crate::QueryParams;

/// Applies resize and/or smart-crop according to `w`, `h`, `a`, and `we` params.
pub fn apply_resize(img: VipsImage, params: &QueryParams) -> Result<VipsImage> {
    let n_pages = img.n_pages();
    let current_w = img.width();
    // Height is stored as total-height for all frames; divide by page count for real height.
    let current_h = img.height() / n_pages;

    let target_w = params.w.unwrap_or(0);
    let target_h = params.h.unwrap_or(0);

    if let Some(align_mode) = &params.a {
        let interesting = match align_mode.as_str() {
            "entropy" => Some(Interesting::Entropy),
            "attention" => Some(Interesting::Attention),
            _ => None,
        };

        if let Some(interest) = interesting {
            if n_pages > 1 {
                // For animated images, we must smartcrop each frame individually
                let mut frames = Vec::with_capacity(n_pages as usize);
                for i in 0..n_pages {
                    let frame = vips::extract_area(&img, 0, i * current_h, current_w, current_h)
                        .map_err(|e| anyhow!("Failed to extract frame {}: {}", i, e))?;
                    let cropped = vips::smartcrop(&frame, target_w, target_h, interest)
                        .map_err(|e| anyhow!("Smartcrop failed on frame {}: {}", i, e))?;
                    frames.push(cropped);
                }
                let joined = vips::arrayjoin(&mut frames)
                    .map_err(|e| anyhow!("Failed to join frames: {}", e))?;
                vips::set_page_height(&joined, target_h);
                return Ok(joined);
            } else {
                return vips::smartcrop(&img, target_w, target_h, interest)
                    .map_err(|e| anyhow!("Smartcrop failed: {}", e));
            }
        }
    }

    // Default: uniform scale-to-fit
    let scale_w = if target_w > 0 { target_w as f64 / current_w as f64 } else { 1.0 };
    let scale_h = if target_h > 0 { target_h as f64 / current_h as f64 } else { 1.0 };
    let mut scale = scale_w.min(scale_h);

    // Without-enlargement: clamp scale to 1.0
    if params.we.unwrap_or(false) && scale > 1.0 {
        scale = 1.0;
    }

    let resized = vips::resize(&img, scale).map_err(|e| anyhow!("Resize failed: {}", e))?;
    
    // For animated images, we must update the page-height metadata
    if n_pages > 1 {
        let new_page_height = (current_h as f64 * scale).round() as i32;
        vips::set_page_height(&resized, new_page_height);
    }
    
    Ok(resized)
}

/// Applies sharpen and/or blur filters when present in params.
pub fn apply_filters(mut img: VipsImage, params: &QueryParams) -> Result<VipsImage> {
    if let Some(sigma) = params.sharp {
        img = vips::sharpen(&img, sigma).map_err(|e| anyhow!("Sharpen failed: {}", e))?;
    }
    if let Some(sigma) = params.blur {
        img = vips::gaussblur(&img, sigma).map_err(|e| anyhow!("Gaussblur failed: {}", e))?;
    }
    Ok(img)
}
