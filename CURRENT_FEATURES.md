# Current Features - Footprints

## Overview
Footprints is a self-hosted music history manager that imports scrobbles from Last.fm and ListenBrainz, stores them locally in SQLite, and provides statistics, reports, and charts.

## Existing Features

### 1. Data Import & Sync
**Location**: `src/importers/`, `src/sync/`

#### One-Time Import
- **Last.fm Import**: Full history import via `user.getrecenttracks` API
  - Pagination support (200 tracks per page)
  - Retry logic for transient failures
  - Batch insertion (1000 tracks per batch)
  - Resume capability from specific page
- **ListenBrainz Import**: Full history import via `/user/{username}/listens` API
  - Timestamp-based pagination (100 listens per request)
  - Retry logic for rate limiting
  - Resume capability from timestamp

#### Automatic Sync
- **Sync Scheduler**: Background task that periodically fetches new scrobbles
- **Configurable Intervals**: Default 60 minutes, customizable per source
- **Multiple Sources**: Can sync from both Last.fm and ListenBrainz simultaneously
- **Incremental Sync**: Only fetches new listens since last sync
- **Manual Triggers**: API endpoint to trigger sync on-demand

### 2. Data Storage & Deduplication
**Location**: `src/db/mod.rs`, `src/models/`

#### Database Schema
- **scrobbles table**: Stores all listening events
  - Fields: id, artist, album, track, timestamp, source, source_id
  - UNIQUE constraint on (artist, track, timestamp, source) prevents duplicates
- **image_cache table**: Caches Last.fm/Deezer album art URLs
- **sync_configs table**: Stores automatic sync configurations

#### Indexes
- `idx_timestamp`: On timestamp DESC (for timeline queries)
- `idx_artist`: On artist (for artist aggregations)
- `idx_source_id`: On source_id (for deduplication)

### 3. Statistics & Reports
**Location**: `src/reports/mod.rs`, `src/api/mod.rs`

#### Basic Statistics
- **Total Scrobbles Count**: All-time and period-based
- **Top Artists**: Most listened artists (limit: 10-50)
- **Top Tracks**: Most played tracks (limit: 10-50)
- **Top Albums**: Most played albums (limit: 10-50)
- **Scrobbles Per Day**: Daily listening counts

#### Time-Based Reports
- **All-Time Report**: Complete listening history (2000-present)
- **Yearly Report**: Year-specific statistics (1970-2100 range)
- **Monthly Report**: Month-specific statistics
- **Last Month Report**: Previous calendar month

#### Period Filters
- Today
- Week (last 7 days)
- Month (current calendar month)
- Year (current calendar year)
- Custom date range (RFC3339 format)
- All-time

### 4. Web UI
**Location**: `templates/index.html`

#### Pages/Views
- **Overview**: Top artists, tracks, albums with images
- **Timeline**: Chronological listening history with pagination
- **Reports**: Pre-generated period reports
- **Import**: Manual import interface
- **Sync Config**: Automatic sync management

#### Features
- Period selector (today/week/month/year/custom/all-time)
- Artist/album images via Last.fm API (with gradient fallbacks)
- "Pulse" chart showing daily scrobble counts
- Pagination for large datasets (50-100 items per page)

### 5. API Endpoints
**Location**: `src/api/mod.rs`

#### Scrobbles & Stats
- `GET /api/scrobbles?limit=100&offset=0` - Paginated scrobble list
- `GET /api/stats` - Overall statistics (count, top artists, top tracks)
- `GET /api/stats/ui?period={period}` - UI-ready stats with images
- `GET /api/pulse?period={period}` - Daily scrobble counts for charting
- `GET /api/timeline?limit=50&offset=0` - Timeline data

#### Reports
- `GET /api/reports/alltime` - All-time report
- `GET /api/reports/lastmonth` - Last month report
- `GET /api/reports/{year}` - Yearly report (e.g., /api/reports/2024)
- `GET /api/reports/monthly?year=2024&month=12` - Monthly report

#### Import & Sync
- `POST /api/import` - One-time import from Last.fm or ListenBrainz
- `POST /api/sync/config` - Create/update sync configuration
- `GET /api/sync/config` - List all sync configurations
- `GET /api/sync/config/:id` - Get specific sync configuration
- `DELETE /api/sync/config/:id` - Delete sync configuration
- `POST /api/sync/config/:id/trigger` - Manually trigger sync

#### Export
- `GET /api/export?format={json|csv}` - Export all scrobbles

### 6. Image Service
**Location**: `src/images/`

#### Providers
- **Last.fm**: Primary image source (requires API key)
- **Deezer**: Fallback image source (no API key required)

#### Caching
- LRU cache in database (image_cache table)
- Fetched_at and last_accessed timestamps for cache management
- Supports artists, albums, and tracks

#### Image Types
- Artist images
- Album cover art
- Track artwork (with fallback to artist/album)

### 7. Data Quality & Handling

#### Deduplication
- Database-level UNIQUE constraint prevents duplicate scrobbles
- Source-specific IDs (source_id) for additional safety
- "INSERT OR IGNORE" strategy for batch imports

#### Timezone
- All timestamps stored as Unix epoch (UTC)
- No user timezone configuration (uses UTC)
- Client-side display could handle timezone conversion

#### Missing Data
- Album field is optional (can be NULL)
- source_id is optional
- Track and artist are required

## Current Limitations

### Data Model
- No separate artist/album/track tables (everything in scrobbles)
- No genre/tag information
- No track duration data
- No MusicBrainz IDs (MBIDs)
- No play count tracking (just scrobble events)

### Statistics
- No session detection or analysis
- No novelty/discovery metrics
- No diversity or entropy calculations
- No time-of-day or weekday patterns
- No artist-to-artist transition analysis
- No repeat vs. new listen tracking

### Performance
- No materialized views or pre-computed aggregates
- All aggregations run on-demand
- No server-side caching beyond images
- Large reports (all-time) may be slow with 100k+ scrobbles

### UI/UX
- No drilldown capabilities (click artist → see all tracks)
- No filtering by source, date range in UI
- No charts/visualizations beyond "Pulse" line chart
- No session views or detailed analytics

### Configuration
- No user timezone setting
- No configurable session gap
- No "minimum track duration" filter
- No artist/tag exclusions
