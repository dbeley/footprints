mod api;
mod db;
mod importers;
mod models;
mod reports;

use anyhow::Result;
use std::net::SocketAddr;
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

    // Create router
    let app = api::create_router(pool)
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
