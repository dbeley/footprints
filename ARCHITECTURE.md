# Architecture

## Overview

Footprints is a self-hosted music history manager built with Rust and SQLite. The architecture follows a layered approach with clear separation of concerns.

## Components

### 1. Database Layer (`src/db/`)

The database layer handles all SQLite operations:

- **Schema**: Single `scrobbles` table with indices for performance
- **Deduplication**: UNIQUE constraint on `(artist, track, timestamp, source)` prevents duplicates
- **Connection Pooling**: r2d2 connection pool for concurrent access
- **Operations**: Insert, query, aggregations for stats and reports

### 2. Models (`src/models/`)

Data structures representing application entities:

- **Scrobble**: Represents a single music listening event
  - Fields: artist, album, track, timestamp, source, source_id
  - Builder pattern for optional fields

### 3. Importers (`src/importers/`)

Modules for importing data from external services:

- **Last.fm Importer**: 
  - Uses Last.fm API v2.0
  - Paginated fetching with rate limiting consideration
  - Skips currently playing tracks
  
- **ListenBrainz Importer**:
  - Uses ListenBrainz API v1
  - Token-based authentication (optional)
  - Max timestamp pagination

Both importers:
- Use async/await with tokio runtime
- Generate unique source_id for deduplication
- Handle API errors gracefully
- Support incremental sync via `import_since()` method

### 4. Sync Module (`src/sync/`)

Background synchronization system for automatic imports:

- **SyncScheduler**: 
  - Runs in background with tokio async runtime
  - Checks enabled sync configs every minute
  - Triggers sync based on configured intervals
  - Updates last sync timestamp after successful sync
  
- **SyncConfig Model**:
  - Stores sync settings per user/source
  - Configurable sync intervals (in minutes)
  - Can be enabled/disabled
  - Stores API credentials securely in database

Key features:
- Incremental sync: Only fetches scrobbles since last sync
- No duplicates: Leverages existing database constraints
- Manual trigger: API endpoint to force immediate sync
- Coexists with one-time import functionality

### 5. Reports (`src/reports/`)

Report generation for different time periods:

- All-time statistics
- Yearly reports (e.g., 2024, 2023)
- Monthly reports
- Last month report
- Top artists, tracks, and albums

### 6. API Layer (`src/api/`)

REST API built with Axum:

**Scrobbles & Stats:**
- **GET /**: Main HTML interface
- **GET /api/scrobbles**: Paginated scrobbles list
- **GET /api/stats**: Overall statistics
- **GET /api/timeline**: Timeline view
- **GET /api/reports/:type**: Generated reports
- **POST /api/import**: Import data from Last.fm or ListenBrainz (one-time)

**Sync Configuration:**
- **POST /api/sync/config**: Create or update sync configuration
- **GET /api/sync/config**: Get all sync configurations
- **GET /api/sync/config/:id**: Get specific sync configuration
- **POST /api/sync/config/:id**: Update sync configuration
- **DELETE /api/sync/config/:id**: Delete sync configuration
- **POST /api/sync/config/:id/trigger**: Manually trigger a sync

### 7. Frontend (`templates/`)

Minimalist HTML/CSS/JavaScript interface:

- No external JavaScript dependencies
- Vanilla JavaScript for API calls
- CSS Grid for responsive layout
- Tab-based navigation
- Real-time import progress

## Data Flow

### One-Time Import Flow

```
User Request → API Handler → Importer
                                ↓
                          External API
                                ↓
                        Scrobble Objects
                                ↓
                        Database Layer
                                ↓
                          SQLite DB
```

### Automatic Sync Flow

```
SyncScheduler (background) → Check enabled configs
                                    ↓
                          Check last sync time
                                    ↓
                          Importer.import_since()
                                    ↓
                              External API
                                    ↓
                          New Scrobble Objects
                                    ↓
                            Database Layer
                                    ↓
                              SQLite DB
                                    ↓
                          Update last_sync_timestamp
```

### Query Flow

```
User Request → API Handler → Database Layer → SQLite DB
                                ↓
                           JSON Response
                                ↓
                          Frontend Update
```

## Database Schema

```sql
CREATE TABLE scrobbles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artist TEXT NOT NULL,
    album TEXT,
    track TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    source TEXT NOT NULL,
    source_id TEXT,
    UNIQUE(artist, track, timestamp, source)
);

CREATE INDEX idx_timestamp ON scrobbles(timestamp DESC);
CREATE INDEX idx_artist ON scrobbles(artist);
CREATE INDEX idx_source_id ON scrobbles(source_id);

CREATE TABLE sync_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    username TEXT NOT NULL,
    api_key TEXT,
    token TEXT,
    sync_interval_minutes INTEGER NOT NULL DEFAULT 60,
    last_sync_timestamp INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(source, username)
);

CREATE INDEX idx_sync_configs_enabled ON sync_configs(enabled, source);
```

## Deployment

### Docker

The application is containerized with a multi-stage Dockerfile:

1. **Builder stage**: Compiles Rust code
2. **Runtime stage**: Minimal Debian image with just the binary

### docker-compose

Simple deployment with:
- Volume mount for persistent database
- Environment variable configuration
- Port mapping

## Performance Considerations

1. **Connection Pooling**: r2d2 manages database connections efficiently
2. **Indices**: Strategic indices on timestamp, artist, source_id, and sync configs
3. **Pagination**: API endpoints support limit/offset pagination
4. **Deduplication**: Database-level UNIQUE constraint prevents duplicates at insert time
5. **Incremental Sync**: Only fetches new scrobbles since last sync, reducing API calls and database operations
6. **Background Processing**: Sync scheduler runs asynchronously without blocking the main application

## Security

1. **No secrets in code**: API keys provided by user at runtime
2. **Input validation**: SQL injection prevented by parameterized queries
3. **HTTPS**: Should be used with reverse proxy (nginx, Caddy) in production
4. **CORS**: Can be configured if needed
5. **Credential Storage**: Sync credentials stored in database (consider encryption for production)

## Automatic Sync

The automatic sync feature allows users to keep their scrobble history up-to-date without manual intervention:

- **Configuration**: Create sync configs via API with source, username, and credentials
- **Scheduling**: Background scheduler checks for due syncs every minute
- **Incremental Updates**: Only fetches scrobbles since the last successful sync
- **Deduplication**: Existing database constraints prevent duplicate entries
- **Manual Trigger**: API endpoint available to force immediate sync
- **Coexistence**: Works alongside one-time import functionality

### Sync Interval Recommendations

- **High activity users**: 15-30 minutes
- **Normal users**: 60 minutes (default)
- **Light users**: 120-240 minutes

## Future Enhancements

Potential areas for extension:

- User authentication and multi-user support
- More visualizations (charts, graphs)
- Export functionality (CSV, JSON)
- Integration with more services (Spotify, Apple Music)
- Advanced statistics (listening patterns, discovery rate)
- Search and filtering
- Tags and playlists
- Sync status history and error logging
- Web UI for sync configuration management
- Notification system for sync failures
