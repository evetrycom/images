use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    http::{StatusCode, HeaderMap},
};
use std::sync::Arc;
use crate::{AppState, QueryParams, processor};
use sha2::{Sha256, Digest};
use hex;

pub async fn health_check() -> &'static str {
    "Evetry Images is running OK"
}

// Handler for external images: /url/https://example.com/img.png?tr=...
pub async fn handle_external_image(
    State(state): State<Arc<AppState>>,
    Path(remote_url): Path<String>,
    Query(params): Query<QueryParams>,
    headers: HeaderMap,
) -> axum::response::Response {
    if !state.allow_external {
        return (StatusCode::FORBIDDEN, "External URLs are disabled").into_response();
    }

    tracing::info!("Proxying external image: {}", remote_url);
    
    // Process image
    process_and_respond(state, processor::ImageSource::Url(remote_url), params, headers).await
}

// Handler for S3 images: /assets/photo.jpg?tr=...
pub async fn handle_s3_image(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Query(params): Query<QueryParams>,
    headers: HeaderMap,
) -> axum::response::Response {
    tracing::info!("Fetching image from S3: {}", path);
    
    process_and_respond(state, processor::ImageSource::S3(path), params, headers).await
}

async fn process_and_respond(
    state: Arc<AppState>,
    source: processor::ImageSource,
    params: QueryParams,
    headers: HeaderMap,
) -> axum::response::Response {
    // 1. Generate ETag based on source and parameters
    let etag = generate_etag(&source, &params);

    // 2. Check If-None-Match header for cache hits
    if let Some(if_none_match) = headers.get("if-none-match").and_then(|v| v.to_str().ok()) {
        if if_none_match == etag || if_none_match == format!("W/\"{}\"", etag) {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }
    
    match processor::process_image(&state, source, params).await {
        Ok(processor::ProcessedResult::Image(buffer, mime_type)) => {
            (
                StatusCode::OK,
                [
                    ("Content-Type", mime_type), 
                    ("Cache-Control", "public, max-age=31536000, immutable".to_string()),
                    ("ETag", format!("\"{}\"", etag)),
                ],
                buffer,
            ).into_response()
        }
        Ok(processor::ProcessedResult::Json(json_val)) => {
            (
                StatusCode::OK,
                [("Content-Type", "application/json".to_string())],
                serde_json::to_string_pretty(&json_val).unwrap(),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Error processing image: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

fn generate_etag(source: &processor::ImageSource, params: &QueryParams) -> String {
    let mut hasher = Sha256::new();
    
    // Hash the source identifier
    match source {
        processor::ImageSource::Url(url) => hasher.update(url.as_bytes()),
        processor::ImageSource::S3(key) => hasher.update(key.as_bytes()),
    }
    
    // Hash the transformation parameters
    // We use debug print as a simple way to serialize for hashing
    hasher.update(format!("{:?}", params).as_bytes());
    
    // Constant salt to allow force-purging global cache by changing this string
    hasher.update(b"v1.0.0"); 

    hex::encode(hasher.finalize())
}
