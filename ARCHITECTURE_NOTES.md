# Architecture Notes - Footprints

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Web Browser                          │
│            (templates/index.html + vanilla JS)              │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTP/JSON
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Axum Web Server                           │
│                   (src/api/mod.rs)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Routes     │  │  Handlers    │  │  State/Context  │  │
│  │   (GET/POST) │─→│  (async fns) │─→│  (Pool, Svc)    │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Database   │  │   Reports    │  │   Importers  │
│  (src/db/)   │  │(src/reports/)│  │(src/importers/)│
└──────┬───────┘  └──────────────┘  └──────┬───────┘
       │                                     │
       ▼                                     ▼
┌──────────────┐                    ┌──────────────┐
│   SQLite     │                    │  External    │
│  footprints  │                    │  APIs        │
│     .db      │                    │ (Last.fm, LB)│
└──────────────┘                    └──────────────┘
```

## Module Breakdown

### `src/main.rs` (Entry Point)
**Responsibilities:**
- Initialize tracing/logging
- Load environment variables (.env)
- Create database connection pool
- Initialize database schema
- Create ImageService with Last.fm API key
- Start sync scheduler background task
- Create Axum router
- Bind to TCP socket and serve

**Key Dependencies:**
- `dotenvy` for .env loading
- `tracing-subscriber` for structured logging
- Database pool shared via Arc across handlers

### `src/db/mod.rs` (Database Layer)
**Responsibilities:**
- Connection pool management (r2d2)
- Schema initialization (CREATE TABLE IF NOT EXISTS)
- Index creation for performance
- All CRUD operations for scrobbles, sync_configs
- Query functions for statistics (top artists, tracks, albums)
- Aggregation functions (counts, per-day breakdown)

**Key Functions:**
- `create_pool()` - Creates r2d2 pool with SQLite
- `init_database()` - Creates tables and indexes
- `insert_scrobble()` / `insert_scrobbles_batch()` - Insert with deduplication
- `get_scrobbles()` - Paginated retrieval
- `get_top_artists/tracks/albums()` - Aggregation queries with optional date range
- `get_scrobbles_per_day()` - Daily counts
- Sync config CRUD operations

**Database Type:** `DbPool = Pool<SqliteConnectionManager>`

**Testing:** `src/db/tests.rs` - Uses tempfile for isolated test databases

### `src/models/` (Data Models)
**Location:** `scrobble.rs`, `sync_config.rs`

#### Scrobble Model
```rust
pub struct Scrobble {
    pub id: Option<i64>,           // Auto-increment primary key
    pub artist: String,            // Required
    pub album: Option<String>,     // Optional
    pub track: String,             // Required
    pub timestamp: DateTime<Utc>,  // When scrobbled
    pub source: String,            // "lastfm" or "listenbrainz"
    pub source_id: Option<String>, // External API ID for dedup
}
```

#### SyncConfig Model
```rust
pub struct SyncConfig {
    pub id: Option<i64>,
    pub source: String,                      // "lastfm" or "listenbrainz"
    pub username: String,
    pub api_key: Option<String>,             // For Last.fm
    pub token: Option<String>,               // For ListenBrainz
    pub sync_interval_minutes: i32,          // Default: 60
    pub last_sync_timestamp: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### `src/api/mod.rs` (Web API Layer)
**Responsibilities:**
- Define all HTTP routes
- Request/response serialization (JSON)
- Parameter parsing and validation
- Call database functions
- Fetch images via ImageService
- Trigger imports and syncs

**State Management:**
```rust
pub struct AppState {
    pub pool: DbPool,                        // Database pool
    pub image_service: Arc<ImageService>,    // Image fetching/caching
    pub sync_scheduler: SyncScheduler,       // Background sync manager
}
```

**Router Structure:**
- `/` - Serves `templates/index.html`
- `/api/scrobbles` - GET scrobbles with pagination
- `/api/stats` - GET basic statistics
- `/api/stats/ui` - GET statistics with images for UI
- `/api/pulse` - GET daily scrobble counts
- `/api/reports/{type}` - GET various report types
- `/api/reports/monthly` - GET monthly report
- `/api/timeline` - GET timeline data
- `/api/import` - POST to trigger one-time import
- `/api/sync/config` - CRUD for sync configurations
- `/api/sync/config/:id/trigger` - POST to manually trigger sync
- `/api/export` - GET to export data (JSON/CSV)
- `/static/*` - Static file serving

**Date Range Helpers:**
- `get_today_range()` - Current day 00:00:00 to now
- `get_week_range()` - Last 7 days
- `get_month_range()` - Current calendar month
- `get_year_range()` - Current calendar year
- `parse_custom_range()` - Parse RFC3339 dates

### `src/reports/mod.rs` (Report Generation)
**Responsibilities:**
- Generate pre-defined reports with date ranges
- Aggregate top artists, tracks, albums
- Count total scrobbles in period

**Report Types:**
- `generate_yearly_report()` - Full calendar year (Jan 1 - Dec 31)
- `generate_monthly_report()` - Full calendar month
- `generate_last_month_report()` - Previous calendar month
- `generate_all_time_report()` - 2000-01-01 to now

**Report Structure:**
```rust
pub struct Report {
    pub period: String,                      // "2024", "2024-12", "All Time"
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_scrobbles: i64,
    pub top_artists: Vec<(String, i64)>,     // (name, count)
    pub top_tracks: Vec<(String, String, i64)>, // (artist, track, count)
    pub top_albums: Vec<(String, String, i64)>, // (artist, album, count)
}
```

**Testing:** `src/reports/tests.rs` - Validates date ranges and report generation

### `src/importers/` (Data Import)
**Location:** `lastfm.rs`, `listenbrainz.rs`

#### Last.fm Importer (`lastfm.rs`)
**API Endpoint:** `https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks`

**Strategy:**
- Pagination: 200 tracks per page
- Starts from page 1, continues until no more tracks
- Batch insertion: Accumulates 1000 tracks, then bulk insert
- Retry logic: 3 attempts for transient failures
- Resume capability: Can restart from specific page

**Deduplication:**
- Uses timestamp + artist + track as unique key
- Stores Last.fm track URL as source_id
- "INSERT OR IGNORE" prevents duplicates

**Fields Mapped:**
- artist.#text → artist
- album.#text → album (optional)
- name → track
- date.uts → timestamp (Unix timestamp string)

#### ListenBrainz Importer (`listenbrainz.rs`)
**API Endpoint:** `https://api.listenbrainz.org/1/user/{username}/listens`

**Strategy:**
- Pagination: 100 listens per request
- Uses `max_ts` parameter to fetch older listens
- Continues until count < 100 (reached the end)
- Retry logic: 3 attempts, handles rate limiting
- Resume capability: Can restart from timestamp

**Deduplication:**
- Uses listened_at + artist + track as unique key
- Stores recording_msid as source_id
- "INSERT OR IGNORE" prevents duplicates

**Fields Mapped:**
- track_metadata.artist_name → artist
- track_metadata.release_name → album (optional)
- track_metadata.track_name → track
- listened_at → timestamp (Unix timestamp integer)

### `src/sync/` (Automatic Sync)
**Location:** `mod.rs`, `scheduler.rs`

#### SyncScheduler
**Responsibilities:**
- Background task that runs every 60 seconds
- Checks all enabled sync configurations
- Determines if sync is due based on interval
- Triggers incremental import for each due source
- Updates last_sync_timestamp after successful sync

**Logic:**
- For each enabled config:
  - Check if `now - last_sync > interval`
  - If yes, call importer with `since` parameter
  - Update `last_sync_timestamp` in database

**Concurrency:**
- Each sync runs sequentially (no parallel syncs)
- Uses tokio spawn for background execution
- Shared state via Arc<Mutex<>> for scheduler control

### `src/images/` (Image Fetching & Caching)
**Location:** `mod.rs`, `types.rs`, `lastfm.rs`, `deezer.rs`, `cache.rs`

#### ImageService
**Responsibilities:**
- Fetch artist/album/track images from external APIs
- Cache results in database (image_cache table)
- LRU eviction based on last_accessed
- Fallback from Last.fm to Deezer

**Image Request Types:**
```rust
pub enum ImageRequest {
    Artist { name: String },
    Album { artist: String, album: String },
    Track { artist: String, track: String },
}
```

**Caching Strategy:**
- Check database cache first
- If miss, fetch from Last.fm (if API key available)
- If Last.fm fails, try Deezer
- Store result with fetched_at and last_accessed
- Update last_accessed on cache hit
- LRU eviction: Delete oldest last_accessed when cache grows too large

**Last.fm Image Sizes:**
- small, medium, large, extralarge, mega
- Default: "large"

## Database Schema Details

### scrobbles Table
```sql
CREATE TABLE scrobbles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artist TEXT NOT NULL,
    album TEXT,
    track TEXT NOT NULL,
    timestamp INTEGER NOT NULL,           -- Unix epoch (UTC)
    source TEXT NOT NULL,                 -- "lastfm" or "listenbrainz"
    source_id TEXT,                       -- External ID for dedup
    UNIQUE(artist, track, timestamp, source)
)
```

**Indexes:**
- `idx_timestamp ON scrobbles(timestamp DESC)` - Timeline queries
- `idx_artist ON scrobbles(artist)` - Artist aggregations
- `idx_source_id ON scrobbles(source_id)` - Dedup lookups

### image_cache Table
```sql
CREATE TABLE image_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,            -- "artist", "album", "track"
    entity_name TEXT NOT NULL,            -- Artist name
    entity_album TEXT,                    -- Album name (for albums/tracks)
    image_url TEXT,                       -- Fetched URL (NULL if not found)
    image_size TEXT NOT NULL,             -- "small", "medium", "large", etc.
    fetched_at INTEGER NOT NULL,          -- When first fetched
    last_accessed INTEGER NOT NULL,       -- For LRU eviction
    UNIQUE(entity_type, entity_name, entity_album, image_size)
)
```

**Indexes:**
- `idx_image_cache_lookup ON image_cache(entity_type, entity_name, entity_album)` - Fast lookups
- `idx_image_cache_lru ON image_cache(last_accessed)` - LRU eviction

### sync_configs Table
```sql
CREATE TABLE sync_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    username TEXT NOT NULL,
    api_key TEXT,
    token TEXT,
    sync_interval_minutes INTEGER NOT NULL DEFAULT 60,
    last_sync_timestamp INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,   -- Boolean (1=true, 0=false)
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(source, username)
)
```

**Indexes:**
- `idx_sync_configs_enabled ON sync_configs(enabled, source)` - Scheduler queries

## Query Patterns

### Common Aggregation Pattern
Most stats queries follow this pattern:
```sql
SELECT artist, COUNT(*) as count 
FROM scrobbles
WHERE timestamp >= ?1 AND timestamp <= ?2
GROUP BY artist 
ORDER BY count DESC 
LIMIT ?3
```

### Date Range Filtering
Optional date range parameters:
- If `Some(start)` and `Some(end)`: Filter with WHERE clause
- If `None`: Query all data

### Pagination
Standard pagination via LIMIT/OFFSET:
```sql
SELECT * FROM scrobbles 
ORDER BY timestamp DESC 
LIMIT ?1 OFFSET ?2
```

## Performance Considerations

### Current Optimizations
- Indexes on timestamp (DESC) for timeline queries
- Index on artist for aggregations
- Batch insertion (1000 records at a time)
- Image caching to avoid repeated API calls
- r2d2 connection pooling

### Potential Bottlenecks
- Large aggregations (top artists over 100k+ scrobbles) run on-demand
- No materialized views or pre-computed stats
- All-time reports query entire table
- No query result caching (beyond images)
- No pagination limits enforced (could fetch 1M records)

### Scalability Limits
- SQLite single-writer limitation (not an issue for read-heavy workload)
- File-based database (single machine only)
- No sharding or distribution
- Expected scale: 10k-1M scrobbles (typical user has ~100k-500k)

## Configuration

### Environment Variables
- `DATABASE_PATH` - Path to SQLite database file (default: "footprints.db")
- `PORT` - HTTP server port (default: 3000)
- `RUST_LOG` - Logging level (default: "footprints=info")
- `LASTFM_API_KEY` - Last.fm API key for images (optional)

### Compile-Time Configuration
- Batch size: 1000 (in importers)
- Per-page fetch: 200 (Last.fm), 100 (ListenBrainz)
- Max retries: 3
- Default sync interval: 60 minutes
- Report limit: 50 results

## Error Handling

### Strategy
- `anyhow::Result<T>` for propagating errors
- `StatusCode` enums for HTTP responses
- Logging via `tracing::error!` / `tracing::warn!`

### Fallback Behavior
- Image fetching: Gradient placeholder if unavailable
- Database errors: Return 500 Internal Server Error
- Invalid parameters: Return 400 Bad Request
- Missing resources: Return 404 Not Found

## Testing Strategy

### Unit Tests
- `src/db/tests.rs` - Database operations with tempfile
- `src/reports/tests.rs` - Report generation validation

### Test Fixtures
- `tempfile::NamedTempFile` for isolated test databases
- Minimal scrobble data created in-test
- No external API calls in tests (all mocked or skipped)

### Coverage
- Basic CRUD operations tested
- Deduplication logic tested
- Sync config lifecycle tested
- Report date range validation tested
- **Missing**: API endpoint integration tests, importer tests, image service tests
