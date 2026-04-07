use crate::vips::{self, VipsImage};

/// Applies a shape mask to `img`, making areas outside the shape transparent.
///
/// Supported mask types: `"circle"`, `"ellipse"`, `"path"`.
/// Returns the original image unchanged for unrecognised mask types.
pub fn apply_mask(img: VipsImage, mask_type: &str, path_data: Option<&str>) -> Result<VipsImage, String> {
    let width = img.width();
    let n_pages = img.n_pages();
    let height = img.height() / n_pages; // real frame height

    let svg = build_mask_svg(mask_type, width, height, path_data);
    let svg = match svg {
        Some(s) => s,
        None => return Ok(img), // unknown mask type — pass through
    };

    // Load the SVG mask as a single-band image.
    let mask = vips::image_from_buffer(svg.as_bytes(), "")
        .map_err(|e| format!("Failed to load SVG mask: {}", e))?;

    let mask = if mask.bands() > 1 {
        vips::extract_band(&mask, 0, 1)
            .map_err(|e| format!("extract_band failed: {}", e))?
    } else {
        mask
    };

    // For animated images, replicate the mask for every frame.
    let full_mask = if n_pages > 1 {
        let mut frames: Vec<VipsImage> = (0..n_pages).map(|_| mask.add_ref()).collect();
        vips::arrayjoin(&mut frames).map_err(|e| format!("arrayjoin failed: {}", e))?
    } else {
        mask
    };

    // Ensure the image has an alpha channel.
    let bands = img.bands();
    let img_with_alpha = if bands == 3 || bands == 1 {
        let white_alpha = vips::image_new_from_image1(&img, 255.0)
            .map_err(|e| format!("image_new_from_image1 failed: {}", e))?;
        vips::bandjoin2(&img, &white_alpha)
            .map_err(|e| format!("bandjoin2 failed: {}", e))?
    } else {
        img
    };

    // Replace the alpha channel with our mask.
    let current_bands = img_with_alpha.bands();
    let color_bands = vips::extract_band(&img_with_alpha, 0, current_bands - 1)
        .map_err(|e| format!("extract_band color failed: {}", e))?;

    vips::bandjoin2(&color_bands, &full_mask)
        .map_err(|e| format!("bandjoin2 mask failed: {}", e))
}

/// Builds an SVG string for the requested mask shape, or `None` for unknown types.
fn build_mask_svg(mask_type: &str, width: i32, height: i32, path_data: Option<&str>) -> Option<String> {
    let svg = match mask_type {
        "circle" => {
            let r = (width.min(height) as f64) / 2.0;
            format!(
                r#"<svg width="{w}" height="{h}"><circle cx="{cx}" cy="{cy}" r="{r}" fill="white"/></svg>"#,
                w = width,
                h = height,
                cx = width as f64 / 2.0,
                cy = height as f64 / 2.0,
                r = r,
            )
        }
        "ellipse" => {
            let rx = width as f64 / 2.0;
            let ry = height as f64 / 2.0;
            format!(
                r#"<svg width="{w}" height="{h}"><ellipse cx="{rx}" cy="{ry}" rx="{rx}" ry="{ry}" fill="white"/></svg>"#,
                w = width,
                h = height,
                rx = rx,
                ry = ry,
            )
        }
        "path" => {
            let d = path_data.unwrap_or("");
            format!(
                r#"<svg width="{w}" height="{h}"><path d="{d}" fill="white"/></svg>"#,
                w = width,
                h = height,
                d = d,
            )
        }
        _ => return None,
    };
    Some(svg)
}
