use anyhow::{anyhow, Result};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::QueryParams;

use super::source::ImageSource;

type HmacSha256 = Hmac<Sha256>;

/// Validates the HMAC-SHA256 request signature.
///
/// The signed payload is `"{path}?{sorted_query_parts}"`.
pub fn validate_signature(secret: &str, source: &ImageSource, params: &QueryParams) -> Result<()> {
    let sig = params
        .sig
        .as_deref()
        .ok_or_else(|| anyhow!("Missing signature"))?;

    let path = match source {
        ImageSource::Url(url) => format!("/url/{}", url),
        ImageSource::S3(p) => format!("/{}", p),
    };

    let mut query_parts: Vec<String> = Vec::new();
    if let Some(w) = params.w {
        query_parts.push(format!("w={}", w));
    }
    if let Some(h) = params.h {
        query_parts.push(format!("h={}", h));
    }
    query_parts.sort();

    let data = format!("{}?{}", path, query_parts.join("&"));

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow!("Invalid HMAC key"))?;
    mac.update(data.as_bytes());

    if sig != hex::encode(mac.finalize().into_bytes()) {
        return Err(anyhow!("Invalid signature"));
    }
    Ok(())
}
