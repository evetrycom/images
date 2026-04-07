use anyhow::{anyhow, Result};
use std::sync::Arc;

use crate::AppState;

use super::source::ImageSource;

use bytes::Bytes;

/// Fetches raw image bytes from either a remote URL or an S3 object.
pub async fn fetch_bytes(state: Arc<AppState>, source: ImageSource) -> Result<Bytes> {
    match &source {
        ImageSource::Url(url) => {
            let resp = reqwest::get(url).await?;
            if !resp.status().is_success() {
                return Err(anyhow!("Failed to fetch URL: {}", resp.status()));
            }
            Ok(resp.bytes().await?)
        }
        ImageSource::S3(path) => {
            let output = state
                .s3_client
                .get_object()
                .bucket(&state.bucket)
                .key(path)
                .send()
                .await?;
            Ok(output.body.collect().await?.into_bytes())
        }
    }
}
