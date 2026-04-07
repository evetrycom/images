mod processor;
mod vips;
mod handlers;

use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::vips::VipsApp;
use aws_sdk_s3::Client as S3Client;
use serde::Deserialize;

#[derive(Clone)]
pub struct AppState {
    pub s3_client: S3Client,
    pub bucket: String,
    pub allow_external: bool,
    pub secret: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct QueryParams {
    pub w: Option<i32>,       // Width
    pub h: Option<i32>,       // Height
    pub fit: Option<String>,  // Fit (cover, contain, etc.)
    pub we: Option<bool>,     // Without enlargement
    pub a: Option<String>,    // Alignment/Smart Crop (entropy, attention)
    pub n: Option<i32>,       // Number of pages (animation)
    pub page: Option<i32>,    // Specific page
    pub blur: Option<f64>,    // Blur
    pub sharp: Option<f64>,   // Sharpen
    pub q: Option<i32>,       // Quality
    pub output: Option<String>, // Output format (webp, avif, json...)
    pub sig: Option<String>,  // HMAC Signature
    // Overlay / Watermark
    pub overlay: Option<String>, // Overlay image source (S3 key or URL)
    pub ox: Option<i32>,      // Overlay X offset
    pub oy: Option<i32>,      // Overlay Y offset
    pub og: Option<i32>,      // Overlay gravity (not implemented yet, using offsets for now)
    // Masking
    pub mask: Option<String>, // Mask shape (circle, ellipse)
    pub d: Option<String>,    // Custom SVG path data
}

#[tokio::main]
async fn main() {
    // 1. Initialize environment and logging
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Initialize Libvips
    let vips_app = VipsApp::new("evetry-images").expect("Could not start libvips");
    
    // Performance tuning:
    vips_app.set_concurrency(num_cpus::get() as i32);
    vips_app.set_cache_max(100); // Cache up to 100 operations

    // 3. Setup S3 Client (R2 Compatible)
    let s3_endpoint = std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
    let bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");
    let access_key = std::env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
    let secret_key = std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
    let allow_external = std::env::var("ALLOW_EXTERNAL_URL").unwrap_or_else(|_| "true".to_string()) == "true";
    let app_secret = std::env::var("APP_SECRET").ok();

    let credentials = aws_sdk_s3::config::Credentials::new(
        access_key,
        secret_key,
        None,
        None,
        "R2",
    );

    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new("auto"))
        .credentials_provider(credentials)
        .endpoint_url(s3_endpoint)
        .load()
        .await;

    let s3_client = S3Client::new(&s3_config);

    let state = Arc::new(AppState {
        s3_client,
        bucket,
        allow_external,
        secret: app_secret,
    });

    // 4. Setup router
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/url/{*remote_url}", get(handlers::handle_external_image))
        .route("/{*path}", get(handlers::handle_s3_image))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("Invalid address");
    
    tracing::info!("🚀 Evetry Images started at http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
