mod handlers;
mod processor;
mod vips;

use crate::vips::VipsApp;
use aws_sdk_s3::Client as S3Client;
use axum::{routing::get, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Clone)]
pub struct AppState {
    pub s3_client: S3Client,
    pub bucket: String,
    pub allow_external: bool,
    pub secret: Option<String>,
    pub allowed_origins: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct QueryParams {
    pub w: Option<i32>,         // Width
    pub h: Option<i32>,         // Height
    pub fit: Option<String>,    // Fit (cover, contain, etc.)
    pub we: Option<bool>,       // Without enlargement
    pub a: Option<String>,      // Alignment/Smart Crop (entropy, attention)
    pub n: Option<i32>,         // Number of pages (animation)
    pub page: Option<i32>,      // Specific page
    pub blur: Option<f64>,      // Blur
    pub sharp: Option<f64>,     // Sharpen
    pub q: Option<i32>,         // Quality
    pub output: Option<String>, // Output format (webp, avif, json...)
    pub sig: Option<String>,    // HMAC Signature
    // Overlay / Watermark
    pub overlay: Option<String>, // Overlay image source (S3 key or URL)
    pub ox: Option<i32>,         // Overlay X offset
    pub oy: Option<i32>,         // Overlay Y offset
    pub og: Option<i32>,         // Overlay gravity (not implemented yet, using offsets for now)
    // Masking
    pub mask: Option<String>, // Mask shape (circle, ellipse)
    pub d: Option<String>,    // Custom SVG path data
}

#[tokio::main]
async fn main() {
    // 1. Initialize environment and logging
    dotenvy::dotenv().ok();

    // Configurable logging (defaults to info)
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "images=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Initialize Libvips with environment-driven tuning
    let vips_app = VipsApp::new("evetry-images").expect("Could not start libvips");

    // Read performance/memory limits from env
    let concurrency = std::env::var("VIPS_CONCURRENCY")
        .map(|s| s.parse().unwrap_or(num_cpus::get() as i32))
        .unwrap_or(num_cpus::get() as i32);

    let cache_ops = std::env::var("VIPS_MAX_CACHE_OPS")
        .map(|s| s.parse().unwrap_or(10))
        .unwrap_or(10);

    let cache_mem = std::env::var("VIPS_MAX_CACHE_MEM")
        .map(|s| parse_size_bytes(&s).unwrap_or(64 * 1024 * 1024))
        .unwrap_or(64 * 1024 * 1024); // Default 64MB

    let cache_files = std::env::var("VIPS_MAX_CACHE_FILES")
        .map(|s| s.parse().unwrap_or(20))
        .unwrap_or(20);

    vips_app.set_concurrency(concurrency);
    vips_app.set_cache_max(cache_ops);
    vips_app.set_cache_max_mem(cache_mem);
    vips_app.set_cache_max_files(cache_files);

    tracing::info!(
        "VIPS initialized: concurrency={}, cache_ops={}, cache_mem={}MB, cache_files={}",
        concurrency,
        cache_ops,
        cache_mem / (1024 * 1024),
        cache_files
    );

    // 3. Setup S3 Client (R2 Compatible)
    let s3_endpoint = std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
    let bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");
    let access_key = std::env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
    let secret_key = std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
    let allow_external =
        std::env::var("ALLOW_EXTERNAL_URL").unwrap_or_else(|_| "true".to_string()) == "true";
    let app_secret = std::env::var("APP_SECRET").ok();

    let credentials =
        aws_sdk_s3::config::Credentials::new(access_key, secret_key, None, None, "R2");

    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new("auto"))
        .credentials_provider(credentials)
        .endpoint_url(s3_endpoint)
        .load()
        .await;

    let s3_client = S3Client::new(&s3_config);

    // 4. Setup CORS
    let allowed_origins = std::env::var("ALLOWED_ORIGINS").ok();

    let state = Arc::new(AppState {
        s3_client,
        bucket,
        allow_external,
        secret: app_secret,
        allowed_origins: allowed_origins.clone(),
    });

    let cors = if let Some(origins) = &allowed_origins {
        if origins == "*" {
            CorsLayer::new().allow_origin(Any)
        } else {
            let list = origins
                .split(',')
                .map(|o| o.parse().expect("Invalid origin"))
                .collect::<Vec<_>>();
            CorsLayer::new().allow_origin(list)
        }
    } else {
        CorsLayer::new().allow_origin(Any)
    };

    let cors = cors.allow_methods(Any).allow_headers(Any);

    // 5. Setup router
    let app = Router::new()
        .route(
            "/health",
            get(handlers::health_check)
                .post(handlers::health_check)
                .head(handlers::health_check),
        )
        .route("/assets/{*path}", get(handlers::handle_s3_image))
        .route("/url/{*remote_url}", get(handlers::handle_external_image))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Invalid address");

    tracing::info!("🚀 Evetry Images started at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Helper to parse sizes like "64MB", "1GB" or just bytes.
fn parse_size_bytes(s: &str) -> Option<usize> {
    let s = s.trim().to_uppercase();
    if s.ends_with("MB") {
        s.strip_suffix("MB")?
            .parse::<usize>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else if s.ends_with("GB") {
        s.strip_suffix("GB")?
            .parse::<usize>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else {
        s.parse::<usize>().ok()
    }
}
