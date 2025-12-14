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

### 4. Reports (`src/reports/`)

Report generation for different time periods:

- All-time statistics
- Yearly reports (e.g., 2024, 2023)
- Monthly reports
- Last month report
- Top artists, tracks, and albums

### 5. API Layer (`src/api/`)

REST API built with Axum:

- **GET /**: Main HTML interface
- **GET /api/scrobbles**: Paginated scrobbles list
- **GET /api/stats**: Overall statistics
- **GET /api/timeline**: Timeline view
- **GET /api/reports/:type**: Generated reports
- **POST /api/import**: Import data from Last.fm or ListenBrainz

### 6. Frontend (`templates/`)

Minimalist HTML/CSS/JavaScript interface:

- No external JavaScript dependencies
- Vanilla JavaScript for API calls
- CSS Grid for responsive layout
- Tab-based navigation
- Real-time import progress

## Data Flow

### Import Flow

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
2. **Indices**: Strategic indices on timestamp, artist, and source_id
3. **Pagination**: API endpoints support limit/offset pagination
4. **Deduplication**: Database-level UNIQUE constraint prevents duplicates at insert time

## Security

1. **No secrets in code**: API keys provided by user at runtime
2. **Input validation**: SQL injection prevented by parameterized queries
3. **HTTPS**: Should be used with reverse proxy (nginx, Caddy) in production
4. **CORS**: Can be configured if needed

## Future Enhancements

Potential areas for extension:

- User authentication and multi-user support
- More visualizations (charts, graphs)
- Export functionality (CSV, JSON)
- Scheduled automatic imports
- Integration with more services (Spotify, Apple Music)
- Advanced statistics (listening patterns, discovery rate)
- Search and filtering
- Tags and playlists
