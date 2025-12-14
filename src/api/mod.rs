use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::DbPool;
use crate::importers::{LastFmImporter, ListenBrainzImporter};
use crate::reports;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct ImportParams {
    source: String,
    username: String,
    api_key: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResponse {
    success: bool,
    count: usize,
    message: String,
}

pub fn create_router(pool: DbPool) -> Router {
    let state = AppState { pool };

    Router::new()
        .route("/", get(root_handler))
        .route("/api/scrobbles", get(get_scrobbles_handler))
        .route("/api/stats", get(get_stats_handler))
        .route("/api/import", post(import_handler))
        .route("/api/reports/:type", get(get_report_handler))
        .route("/api/timeline", get(get_timeline_handler))
        .with_state(Arc::new(state))
}

async fn root_handler() -> Html<String> {
    Html(include_str!("../../templates/index.html").to_string())
}

async fn get_scrobbles_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<crate::models::Scrobble>>, StatusCode> {
    match crate::db::get_scrobbles(&state.pool, params.limit, params.offset) {
        Ok(scrobbles) => Ok(Json(scrobbles)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_stats_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match (
        crate::db::get_scrobbles_count(&state.pool),
        crate::db::get_top_artists(&state.pool, 10, None, None),
        crate::db::get_top_tracks(&state.pool, 10, None, None),
    ) {
        (Ok(count), Ok(artists), Ok(tracks)) => {
            let stats = serde_json::json!({
                "total_scrobbles": count,
                "top_artists": artists,
                "top_tracks": tracks,
            });
            Ok(Json(stats))
        }
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn import_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<ImportParams>,
) -> Result<Json<ImportResponse>, StatusCode> {
    let count = match params.source.as_str() {
        "lastfm" => {
            if let Some(api_key) = params.api_key {
                let importer = LastFmImporter::new(api_key, params.username);
                importer.import_all(&state.pool).await
            } else {
                return Ok(Json(ImportResponse {
                    success: false,
                    count: 0,
                    message: "API key required for Last.fm".to_string(),
                }));
            }
        }
        "listenbrainz" => {
            let importer = ListenBrainzImporter::new(params.username, params.token);
            importer.import_all(&state.pool).await
        }
        _ => {
            return Ok(Json(ImportResponse {
                success: false,
                count: 0,
                message: format!("Unknown source: {}", params.source),
            }));
        }
    };

    match count {
        Ok(n) => Ok(Json(ImportResponse {
            success: true,
            count: n,
            message: format!("Successfully imported {} scrobbles", n),
        })),
        Err(e) => Ok(Json(ImportResponse {
            success: false,
            count: 0,
            message: format!("Import failed: {}", e),
        })),
    }
}

async fn get_report_handler(
    State(state): State<Arc<AppState>>,
    Path(report_type): Path<String>,
) -> Result<Json<reports::Report>, StatusCode> {
    let report = match report_type.as_str() {
        "alltime" => reports::generate_all_time_report(&state.pool),
        "lastmonth" => reports::generate_last_month_report(&state.pool),
        year if year.len() == 4 => {
            if let Ok(y) = year.parse::<i32>() {
                reports::generate_yearly_report(&state.pool, y)
            } else {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match report {
        Ok(r) => Ok(Json(r)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_timeline_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<crate::models::Scrobble>>, StatusCode> {
    match crate::db::get_scrobbles(&state.pool, params.limit, params.offset) {
        Ok(scrobbles) => Ok(Json(scrobbles)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
