use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::DbPool;
use crate::images::{ImageRequest, ImageService};
use crate::importers::{LastFmImporter, ListenBrainzImporter};
use crate::models::SyncConfig;
use crate::reports;
use crate::sync::SyncScheduler;

type DateRange = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub image_service: Arc<ImageService>,
    pub sync_scheduler: SyncScheduler,
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

pub fn create_router(
    pool: DbPool,
    image_service: Arc<ImageService>,
    sync_scheduler: SyncScheduler,
) -> Router {
    let state = AppState {
        pool,
        image_service,
        sync_scheduler,
    };

    Router::new()
        .route("/", get(root_handler))
        .route("/styles.css", get(styles_handler))
        .route("/scripts.js", get(scripts_handler))
        .route("/api/scrobbles", get(get_scrobbles_handler))
        .route("/api/stats", get(get_stats_handler))
        .route("/api/stats/ui", get(get_stats_ui_handler))
        .route("/api/years", get(get_available_years_handler))
        .route("/api/pulse", get(get_pulse_handler))
        .route("/api/import", post(import_handler))
        .route("/api/sync/config", post(create_sync_config_handler))
        .route("/api/sync/config", get(get_sync_configs_handler))
        .route(
            "/api/sync/config/:id",
            get(get_sync_config_handler)
                .post(update_sync_config_handler)
                .delete(delete_sync_config_handler),
        )
        .route("/api/sync/config/:id/trigger", post(trigger_sync_handler))
        .route("/api/export", get(export_handler))
        .route("/api/reports/:type", get(get_report_handler))
        .route("/api/reports/monthly", get(get_monthly_report_handler))
        .route("/api/reports/heatmap", get(get_heatmap_handler))
        .route("/api/reports/novelty", get(get_novelty_handler))
        .route("/api/reports/transitions", get(get_transitions_handler))
        .route("/api/reports/diversity", get(get_diversity_handler))
        .route("/api/reports/yearly/:year", get(get_yearly_handler))
        .route("/api/timeline", get(get_timeline_handler))
        .route("/api/artist/:artist", get(get_artist_handler))
        .route("/api/album/:artist/:album", get(get_album_handler))
        .route("/api/track/:artist/:track", get(get_track_handler))
        .with_state(Arc::new(state))
}

async fn root_handler() -> Html<String> {
    Html(include_str!("../../templates/index.html").to_string())
}

async fn styles_handler() -> axum::response::Response {
    use axum::body::Body;
    use axum::http::header;
    use axum::response::Response;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(include_str!("../../templates/styles.css")))
        .unwrap()
}

async fn scripts_handler() -> axum::response::Response {
    use axum::body::Body;
    use axum::http::header;
    use axum::response::Response;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(include_str!("../../templates/scripts.js")))
        .unwrap()
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

async fn get_available_years_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<i32>>, StatusCode> {
    match crate::db::get_available_years(&state.pool) {
        Ok(years) => Ok(Json(years)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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
                if (1970..=2100).contains(&y) {
                    reports::generate_yearly_report(&state.pool, y)
                } else {
                    return Err(StatusCode::BAD_REQUEST);
                }
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

#[derive(Deserialize)]
struct MonthlyReportParams {
    year: i32,
    month: u32,
}

async fn get_monthly_report_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MonthlyReportParams>,
) -> Result<Json<reports::Report>, StatusCode> {
    if !(1..=12).contains(&params.month) {
        return Err(StatusCode::BAD_REQUEST);
    }

    match reports::generate_monthly_report(&state.pool, params.year, params.month) {
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

#[derive(Deserialize)]
struct HeatmapParams {
    start: Option<String>,
    end: Option<String>,
    #[serde(default = "default_timezone")]
    timezone: String,
    #[serde(default)]
    normalize: bool,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

async fn get_heatmap_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HeatmapParams>,
) -> Result<Json<reports::heatmap::HeatmapReport>, StatusCode> {
    // Parse timezone
    let timezone = params
        .timezone
        .parse::<chrono_tz::Tz>()
        .unwrap_or(chrono_tz::UTC);

    // Parse date strings
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    match reports::heatmap::generate_heatmap(&state.pool, start, end, timezone, params.normalize) {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct NoveltyParams {
    start: Option<String>,
    end: Option<String>,
    #[serde(default = "default_granularity")]
    granularity: String,
}

fn default_granularity() -> String {
    "week".to_string()
}

async fn get_novelty_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<NoveltyParams>,
) -> Result<Json<reports::novelty::NoveltyReport>, StatusCode> {
    // Parse granularity
    let granularity = match params.granularity.to_lowercase().as_str() {
        "day" => reports::novelty::Granularity::Day,
        "week" => reports::novelty::Granularity::Week,
        "month" => reports::novelty::Granularity::Month,
        "year" => reports::novelty::Granularity::Year,
        _ => reports::novelty::Granularity::Week,
    };

    // Parse date strings
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    match reports::novelty::generate_novelty_report(&state.pool, start, end, granularity) {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct TransitionsParams {
    start: Option<String>,
    end: Option<String>,
    #[serde(default = "default_gap_minutes")]
    gap_minutes: i64,
    #[serde(default = "default_min_count")]
    min_count: i64,
    #[serde(default)]
    include_self_transitions: bool,
}

fn default_gap_minutes() -> i64 {
    45
}

fn default_min_count() -> i64 {
    2
}

async fn get_transitions_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TransitionsParams>,
) -> Result<Json<reports::transitions::TransitionsReport>, StatusCode> {
    // Parse date strings
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    match reports::transitions::generate_transitions_report(
        &state.pool,
        start,
        end,
        params.gap_minutes,
        params.min_count,
        params.include_self_transitions,
    ) {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct DiversityParams {
    #[serde(default = "default_granularity")]
    granularity: String,
    start: Option<String>,
    end: Option<String>,
}

async fn get_diversity_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiversityParams>,
) -> Result<Json<reports::diversity::DiversityReport>, StatusCode> {
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let granularity = match params.granularity.as_str() {
        "day" => reports::diversity::Granularity::Day,
        "week" => reports::diversity::Granularity::Week,
        "month" => reports::diversity::Granularity::Month,
        _ => reports::diversity::Granularity::Week,
    };

    match reports::diversity::generate_diversity_report(&state.pool, start, end, granularity) {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_yearly_handler(
    State(state): State<Arc<AppState>>,
    Path(year): Path<i32>,
) -> Result<Json<reports::yearly::YearlyReport>, StatusCode> {
    match reports::yearly::generate_yearly_report(&state.pool, year) {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct StatsUiParams {
    #[serde(default = "default_period")]
    period: String,
    start: Option<String>,
    end: Option<String>,
}

fn default_period() -> String {
    "alltime".to_string()
}

#[derive(Serialize)]
struct ArtistWithImage {
    name: String,
    count: i64,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct TrackWithImage {
    artist: String,
    track: String,
    count: i64,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct AlbumWithImage {
    artist: String,
    album: String,
    count: i64,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct PulsePoint {
    day: String,
    count: i64,
}

async fn get_stats_ui_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<StatsUiParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Calculate date range based on period
    let (start_date, end_date) = match params.period.as_str() {
        "today" => get_today_range(),
        "week" => get_week_range(),
        "month" => get_month_range(),
        "year" => get_year_range(),
        "custom" => parse_custom_range(params.start.as_deref(), params.end.as_deref())
            .ok_or(StatusCode::BAD_REQUEST)?,
        "alltime" => (None, None),
        _ => (None, None),
    };

    // Fetch stats from database
    let top_artists = crate::db::get_top_artists(&state.pool, 15, start_date, end_date)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let top_tracks = crate::db::get_top_tracks(&state.pool, 15, start_date, end_date)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let top_albums = crate::db::get_top_albums(&state.pool, 15, start_date, end_date)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let period_count = crate::db::get_scrobbles_count_in_range(&state.pool, start_date, end_date)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch images for artists
    let mut artists_with_images = Vec::new();
    for (name, count) in top_artists {
        let mut image_url: Option<String> = state
            .image_service
            .get_image_url(ImageRequest::artist(name.clone()))
            .await
            .ok()
            .flatten();

        // fallback: use top album cover for this artist
        if image_url.is_none()
            && let Ok(Some(album)) = crate::db::get_top_album_for_artist(&state.pool, &name)
        {
            image_url = state
                .image_service
                .get_image_url(ImageRequest::album(name.clone(), album))
                .await
                .ok()
                .flatten();
        }
        artists_with_images.push(ArtistWithImage {
            name,
            count,
            image_url,
        });
    }

    // Fetch images for tracks (try track image first, then artist, then album)
    let mut tracks_with_images = Vec::new();
    for (artist, track, count) in top_tracks {
        let mut image_url: Option<String> = state
            .image_service
            .get_image_url(ImageRequest::track(artist.clone(), track.clone()))
            .await
            .ok()
            .flatten();

        // fallback 1: try artist image
        if image_url.is_none() {
            image_url = state
                .image_service
                .get_image_url(ImageRequest::artist(artist.clone()))
                .await
                .ok()
                .flatten();
        }

        // fallback 2: try the most common album for this track
        if image_url.is_none()
            && let Ok(Some(album)) = crate::db::get_album_for_track(&state.pool, &artist, &track)
        {
            image_url = state
                .image_service
                .get_image_url(ImageRequest::album(artist.clone(), album))
                .await
                .ok()
                .flatten();
        }
        tracks_with_images.push(TrackWithImage {
            artist,
            track,
            count,
            image_url,
        });
    }

    // Fetch images for albums
    let mut albums_with_images = Vec::new();
    for (artist, album, count) in top_albums {
        let image_url: Option<String> = state
            .image_service
            .get_image_url(ImageRequest::album(artist.clone(), album.clone()))
            .await
            .ok()
            .flatten();
        albums_with_images.push(AlbumWithImage {
            artist,
            album,
            count,
            image_url,
        });
    }

    Ok(Json(serde_json::json!({
        "period": params.period,
        "period_scrobbles": period_count,
        "top_artists": artists_with_images,
        "top_tracks": tracks_with_images,
        "top_albums": albums_with_images,
    })))
}

#[derive(Deserialize)]
struct PulseParams {
    #[serde(default = "default_period")]
    period: String,
    start: Option<String>,
    end: Option<String>,
}

async fn get_pulse_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PulseParams>,
) -> Result<Json<Vec<PulsePoint>>, StatusCode> {
    let (start_date, end_date) = match params.period.as_str() {
        "today" => get_today_range(),
        "week" => get_week_range(),
        "month" => get_month_range(),
        "year" => get_year_range(),
        "custom" => parse_custom_range(params.start.as_deref(), params.end.as_deref())
            .ok_or(StatusCode::BAD_REQUEST)?,
        "alltime" => (None, None),
        _ => (None, None),
    };

    let data = crate::db::get_scrobbles_per_day(&state.pool, start_date, end_date)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        data.into_iter()
            .map(|(day, count)| PulsePoint { day, count })
            .collect(),
    ))
}

fn get_today_range() -> DateRange {
    let now = Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(Utc).single());
    (today_start, Some(now))
}

fn get_week_range() -> DateRange {
    let now = Utc::now();
    let week_ago = now - Duration::days(7);
    (Some(week_ago), Some(now))
}

fn get_month_range() -> DateRange {
    let now = Utc::now();
    let month_start = now
        .date_naive()
        .with_day(1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| dt.and_local_timezone(Utc).single());
    (month_start, Some(now))
}

fn get_year_range() -> DateRange {
    let now = Utc::now();
    let year_start = now
        .date_naive()
        .with_month(1)
        .and_then(|d| d.with_day(1))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| dt.and_local_timezone(Utc).single());
    (year_start, Some(now))
}

fn parse_custom_range(start: Option<&str>, end: Option<&str>) -> Option<DateRange> {
    let start_dt = start
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let end_dt = end
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    match (start_dt, end_dt) {
        (Some(s), Some(e)) => Some((Some(s), Some(e))),
        _ => None,
    }
}

// Sync configuration handlers
#[derive(Deserialize)]
pub struct CreateSyncConfigParams {
    source: String,
    username: String,
    api_key: Option<String>,
    token: Option<String>,
    #[serde(default = "default_sync_interval")]
    sync_interval_minutes: i32,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_sync_interval() -> i32 {
    60 // Default to 60 minutes
}

fn default_enabled() -> bool {
    true
}

#[derive(Serialize)]
pub struct SyncConfigResponse {
    success: bool,
    message: String,
    config: Option<SyncConfig>,
}

#[derive(Serialize)]
pub struct SyncTriggerResponse {
    success: bool,
    count: usize,
    message: String,
}

async fn create_sync_config_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<CreateSyncConfigParams>,
) -> Result<Json<SyncConfigResponse>, StatusCode> {
    let mut config = SyncConfig::new(
        params.source.clone(),
        params.username.clone(),
        params.sync_interval_minutes,
    )
    .with_enabled(params.enabled);

    if let Some(api_key) = params.api_key {
        config = config.with_api_key(api_key);
    }

    if let Some(token) = params.token {
        config = config.with_token(token);
    }

    match crate::db::insert_sync_config(&state.pool, &config) {
        Ok(_) => Ok(Json(SyncConfigResponse {
            success: true,
            message: format!(
                "Sync configuration created for {} user {}",
                params.source, params.username
            ),
            config: Some(config),
        })),
        Err(e) => {
            tracing::error!("Failed to create sync config: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_sync_configs_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SyncConfig>>, StatusCode> {
    match crate::db::get_all_sync_configs(&state.pool) {
        Ok(configs) => Ok(Json(configs)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_sync_config_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<SyncConfig>, StatusCode> {
    match crate::db::get_sync_config(&state.pool, id) {
        Ok(Some(config)) => Ok(Json(config)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn update_sync_config_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(params): Json<CreateSyncConfigParams>,
) -> Result<Json<SyncConfigResponse>, StatusCode> {
    // Verify the config exists
    match crate::db::get_sync_config(&state.pool, id) {
        Ok(Some(_)) => {
            let mut config = SyncConfig::new(
                params.source.clone(),
                params.username.clone(),
                params.sync_interval_minutes,
            )
            .with_enabled(params.enabled);

            if let Some(api_key) = params.api_key {
                config = config.with_api_key(api_key);
            }

            if let Some(token) = params.token {
                config = config.with_token(token);
            }

            match crate::db::insert_sync_config(&state.pool, &config) {
                Ok(_) => Ok(Json(SyncConfigResponse {
                    success: true,
                    message: format!(
                        "Sync configuration updated for {} user {}",
                        params.source, params.username
                    ),
                    config: Some(config),
                })),
                Err(e) => {
                    tracing::error!("Failed to update sync config: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_sync_config_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<SyncConfigResponse>, StatusCode> {
    match crate::db::delete_sync_config(&state.pool, id) {
        Ok(_) => Ok(Json(SyncConfigResponse {
            success: true,
            message: "Sync configuration deleted".to_string(),
            config: None,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn trigger_sync_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<SyncTriggerResponse>, StatusCode> {
    match state.sync_scheduler.trigger_sync(id).await {
        Ok(count) => Ok(Json(SyncTriggerResponse {
            success: true,
            count,
            message: format!("Successfully synced {} new scrobbles", count),
        })),
        Err(e) => Ok(Json(SyncTriggerResponse {
            success: false,
            count: 0,
            message: format!("Sync failed: {}", e),
        })),
    }
}

#[derive(Deserialize)]
struct ExportParams {
    #[serde(default = "default_export_format")]
    format: String,
}

fn default_export_format() -> String {
    "json".to_string()
}

async fn export_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ExportParams>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::body::Body;
    use axum::http::header;
    use axum::response::Response;

    match crate::db::get_scrobbles(&state.pool, Some(1000000), Some(0)) {
        Ok(scrobbles) => {
            let (content_type, body) = match params.format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&scrobbles)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    ("application/json", json)
                }
                "csv" => {
                    let mut csv = String::from("timestamp,artist,album,track,source\n");
                    for scrobble in scrobbles {
                        let album = scrobble.album.unwrap_or_default();
                        csv.push_str(&format!(
                            "{},{},{},{},{}\n",
                            scrobble.timestamp.to_rfc3339(),
                            escape_csv(&scrobble.artist),
                            escape_csv(&album),
                            escape_csv(&scrobble.track),
                            scrobble.source
                        ));
                    }
                    ("text/csv", csv)
                }
                _ => return Err(StatusCode::BAD_REQUEST),
            };

            let filename = format!(
                "footprints_export_{}.{}",
                Utc::now().format("%Y-%m-%d"),
                params.format
            );

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(body))
                .unwrap())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// Entity detail handlers
#[derive(Deserialize)]
struct EntityParams {
    start: Option<String>,
    end: Option<String>,
}

#[derive(Serialize)]
struct ArtistDetail {
    stats: serde_json::Value,
    top_tracks: Vec<TrackItem>,
    top_albums: Vec<AlbumItem>,
    scrobbles_over_time: Vec<TimePoint>,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct TrackItem {
    name: String,
    count: i64,
}

#[derive(Serialize)]
struct AlbumItem {
    name: String,
    count: i64,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct TimePoint {
    date: String,
    count: i64,
}

#[derive(Serialize)]
struct AlbumDetail {
    stats: serde_json::Value,
    tracks: Vec<TrackItem>,
    scrobbles_over_time: Vec<TimePoint>,
    image_url: Option<String>,
}

#[derive(Serialize)]
struct TrackDetail {
    stats: serde_json::Value,
    scrobbles_over_time: Vec<TimePoint>,
    image_url: Option<String>,
}

async fn get_artist_handler(
    State(state): State<Arc<AppState>>,
    Path(artist): Path<String>,
    Query(params): Query<EntityParams>,
) -> Result<Json<ArtistDetail>, StatusCode> {
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let stats = crate::db::get_artist_stats(&state.pool, &artist, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let top_tracks = crate::db::get_artist_top_tracks(&state.pool, &artist, 20, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|(name, count)| TrackItem { name, count })
        .collect();

    let top_albums_data = crate::db::get_artist_top_albums(&state.pool, &artist, 20, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut top_albums = Vec::new();
    for (name, count) in top_albums_data {
        let image_url = state
            .image_service
            .get_image_url(ImageRequest::album(artist.clone(), name.clone()))
            .await
            .ok()
            .flatten();
        top_albums.push(AlbumItem {
            name,
            count,
            image_url,
        });
    }

    let scrobbles_over_time =
        crate::db::get_artist_scrobbles_over_time(&state.pool, &artist, start, end)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .map(|(date, count)| TimePoint { date, count })
            .collect();

    let mut image_url = state
        .image_service
        .get_image_url(ImageRequest::artist(artist.clone()))
        .await
        .ok()
        .flatten();

    if image_url.is_none()
        && let Ok(Some(album)) = crate::db::get_top_album_for_artist(&state.pool, &artist)
    {
        image_url = state
            .image_service
            .get_image_url(ImageRequest::album(artist.clone(), album))
            .await
            .ok()
            .flatten();
    }

    Ok(Json(ArtistDetail {
        stats,
        top_tracks,
        top_albums,
        scrobbles_over_time,
        image_url,
    }))
}

async fn get_album_handler(
    State(state): State<Arc<AppState>>,
    Path((artist, album)): Path<(String, String)>,
    Query(params): Query<EntityParams>,
) -> Result<Json<AlbumDetail>, StatusCode> {
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let stats = crate::db::get_album_stats(&state.pool, &artist, &album, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tracks = crate::db::get_album_tracks(&state.pool, &artist, &album, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|(name, count)| TrackItem { name, count })
        .collect();

    let scrobbles_over_time =
        crate::db::get_album_scrobbles_over_time(&state.pool, &artist, &album, start, end)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .map(|(date, count)| TimePoint { date, count })
            .collect();

    let image_url = state
        .image_service
        .get_image_url(ImageRequest::album(artist.clone(), album.clone()))
        .await
        .ok()
        .flatten();

    Ok(Json(AlbumDetail {
        stats,
        tracks,
        scrobbles_over_time,
        image_url,
    }))
}

async fn get_track_handler(
    State(state): State<Arc<AppState>>,
    Path((artist, track)): Path<(String, String)>,
    Query(params): Query<EntityParams>,
) -> Result<Json<TrackDetail>, StatusCode> {
    let start = params
        .start
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end = params
        .end
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let stats = crate::db::get_track_stats(&state.pool, &artist, &track, start, end)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let scrobbles_over_time =
        crate::db::get_track_scrobbles_over_time(&state.pool, &artist, &track, start, end)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .map(|(date, count)| TimePoint { date, count })
            .collect();

    let mut image_url = state
        .image_service
        .get_image_url(ImageRequest::track(artist.clone(), track.clone()))
        .await
        .ok()
        .flatten();

    if image_url.is_none() {
        image_url = state
            .image_service
            .get_image_url(ImageRequest::artist(artist.clone()))
            .await
            .ok()
            .flatten();
    }

    if image_url.is_none()
        && let Ok(Some(album)) = crate::db::get_album_for_track(&state.pool, &artist, &track)
    {
        image_url = state
            .image_service
            .get_image_url(ImageRequest::album(artist.clone(), album))
            .await
            .ok()
            .flatten();
    }

    Ok(Json(TrackDetail {
        stats,
        scrobbles_over_time,
        image_url,
    }))
}
