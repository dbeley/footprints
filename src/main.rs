mod api;
mod db;
mod images;
mod importers;
mod models;
mod reports;
mod sync;

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "footprints=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables if .env exists
    let _ = dotenvy::dotenv();

    // Get database path from environment or use default
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "footprints.db".to_string());

    tracing::info!("Initializing database at {}", db_path);

    // Create database pool
    let pool = db::create_pool(&db_path)?;

    // Initialize database schema
    db::init_database(&pool)?;

    tracing::info!("Database initialized successfully");

    // Get Last.fm API key from environment
    let lastfm_api_key = std::env::var("LASTFM_API_KEY").unwrap_or_else(|_| {
        tracing::warn!("LASTFM_API_KEY not set; artist/album images will not be fetched");
        String::new()
    });

    // Create image service
    let image_service = Arc::new(images::ImageService::new(pool.clone(), lastfm_api_key));
    tracing::info!("Image service initialized");

    // Start sync scheduler
    let sync_scheduler = sync::SyncScheduler::new(pool.clone());
    sync_scheduler.start().await;
    tracing::info!("Sync scheduler started");

    // Create router with sync scheduler
    let app = api::create_router(pool, image_service, sync_scheduler)
        .nest_service("/static", ServeDir::new("static"));

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
