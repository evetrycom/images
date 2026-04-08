use crate::{processor, AppState, QueryParams};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hex;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub async fn health_check() -> &'static str {
    "OK"
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
    process_and_respond(
        state,
        processor::ImageSource::Url(remote_url),
        params,
        headers,
    )
    .await
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
    mut params: QueryParams,
    headers: HeaderMap,
) -> axum::response::Response {
    // 1. Dynamic output format negotiation based on Accept header
    if params.output.is_none() {
        if let Some(accept) = headers.get(axum::http::header::ACCEPT).and_then(|v| v.to_str().ok()) {
            if accept.contains("image/avif") {
                params.output = Some("avif".to_string());
            } else if accept.contains("image/webp") {
                params.output = Some("webp".to_string());
            }
        }
    }

    // 2. Generate ETag based on source and parameters
    let etag = generate_etag(&source, &params);

    // 2. Check If-None-Match header for cache hits
    if let Some(if_none_match) = headers.get("if-none-match").and_then(|v| v.to_str().ok()) {
        if if_none_match == etag || if_none_match == format!("W/\"{}\"", etag) {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    match processor::process_image(&state, source, params).await {
        Ok(processor::ProcessedResult::Image(buffer, mime_type)) => {
            let mut res_headers = HeaderMap::new();
            res_headers.insert("Content-Type", mime_type.parse().unwrap());

            // 3. Set Cache-Control with configurable max-age
            let max_age = std::env::var("CACHE_MAX_AGE").unwrap_or_else(|_| "31536000".to_string());
            res_headers.insert(
                "Cache-Control",
                format!("public, max-age={}, immutable", max_age)
                    .parse()
                    .unwrap(),
            );

            // 4. Set ETag
            res_headers.insert("ETag", format!("\"{}\"", etag).parse().unwrap());

            // 5. Build Vary header
            let vary_value = match &state.allowed_origins {
                Some(origins) if origins == "*" => "Accept",
                None => "Accept",
                _ => "Origin, Accept",
            };
            res_headers.insert("Vary", vary_value.parse().unwrap());

            (StatusCode::OK, res_headers, buffer).into_response()
        }
        Ok(processor::ProcessedResult::Json(json_val)) => (
            StatusCode::OK,
            [("Content-Type", "application/json".to_string())],
            serde_json::to_string_pretty(&json_val).unwrap(),
        )
            .into_response(),
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
