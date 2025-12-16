# Data Model Notes - Footprints

## Core Entities

### Scrobble (Listen Event)
**Table:** `scrobbles`  
**Source:** `src/models/scrobble.rs`

Represents a single listening event from any source (Last.fm, ListenBrainz).

#### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | INTEGER | Yes (Auto) | Primary key, auto-increment |
| `artist` | TEXT | Yes | Artist name (not normalized) |
| `album` | TEXT | No | Album/release name (can be NULL) |
| `track` | TEXT | Yes | Track/song name (not normalized) |
| `timestamp` | INTEGER | Yes | Unix epoch timestamp (UTC) when listened |
| `source` | TEXT | Yes | Import source: "lastfm" or "listenbrainz" |
| `source_id` | TEXT | No | External API unique ID (for deduplication) |

#### Constraints
- **Primary Key:** `id`
- **Unique:** `(artist, track, timestamp, source)` - Prevents exact duplicates

#### Indexes
- `idx_timestamp` ON `timestamp DESC` - Fast timeline/recent queries
- `idx_artist` ON `artist` - Fast artist-based aggregations
- `idx_source_id` ON `source_id` - Fast external ID lookups

#### Notes
- **No normalization:** Artist/album/track names stored as-is from source APIs
- **Case sensitivity:** SQLite default (case-insensitive for ASCII, sensitive for Unicode)
- **Duplicates:** Same artist+track at different timestamps = multiple scrobbles
- **Missing albums:** Common for singles, podcasts, or incomplete metadata (~20-30% NULL)

#### Example Data
```json
{
  "id": 12345,
  "artist": "Radiohead",
  "album": "OK Computer",
  "track": "Paranoid Android",
  "timestamp": 1702934400,
  "source": "lastfm",
  "source_id": "https://www.last.fm/music/Radiohead/_/Paranoid+Android"
}
```

### Sync Configuration
**Table:** `sync_configs`  
**Source:** `src/models/sync_config.rs`

Stores automatic sync configurations for background imports.

#### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | INTEGER | Yes (Auto) | Primary key, auto-increment |
| `source` | TEXT | Yes | "lastfm" or "listenbrainz" |
| `username` | TEXT | Yes | Username on the source platform |
| `api_key` | TEXT | No | Last.fm API key (required for Last.fm) |
| `token` | TEXT | No | ListenBrainz user token (optional) |
| `sync_interval_minutes` | INTEGER | Yes | How often to sync (default: 60) |
| `last_sync_timestamp` | INTEGER | No | Unix epoch of last successful sync |
| `enabled` | INTEGER | Yes | Boolean: 1=enabled, 0=disabled |
| `created_at` | INTEGER | Yes | Unix epoch when created |
| `updated_at` | INTEGER | Yes | Unix epoch when last updated |

#### Constraints
- **Primary Key:** `id`
- **Unique:** `(source, username)` - One config per user per source

#### Indexes
- `idx_sync_configs_enabled` ON `(enabled, source)` - Scheduler queries

#### Notes
- **Upsert behavior:** INSERT with ON CONFLICT DO UPDATE
- **Last sync:** NULL means never synced (will import all history on first run)
- **Disabled configs:** Not processed by scheduler but remain in database

### Image Cache
**Table:** `image_cache`  
**Source:** `src/images/cache.rs`

LRU cache for artist/album/track images from external APIs.

#### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | INTEGER | Yes (Auto) | Primary key, auto-increment |
| `entity_type` | TEXT | Yes | "artist", "album", or "track" |
| `entity_name` | TEXT | Yes | Artist name (or track name for tracks) |
| `entity_album` | TEXT | No | Album name (NULL for artists) |
| `image_url` | TEXT | No | Fetched image URL (NULL if not found) |
| `image_size` | TEXT | Yes | Size variant: "small", "medium", "large", etc. |
| `fetched_at` | INTEGER | Yes | Unix epoch when first fetched |
| `last_accessed` | INTEGER | Yes | Unix epoch when last accessed (LRU) |

#### Constraints
- **Primary Key:** `id`
- **Unique:** `(entity_type, entity_name, entity_album, image_size)`

#### Indexes
- `idx_image_cache_lookup` ON `(entity_type, entity_name, entity_album)` - Fast cache lookups
- `idx_image_cache_lru` ON `last_accessed` - LRU eviction

#### Notes
- **LRU eviction:** Periodically remove oldest `last_accessed` entries
- **NULL image_url:** Cached "not found" to avoid repeated API calls
- **Multiple sizes:** Same entity can have multiple cached sizes

## Data Relationships

### Logical Relationships (Not Enforced by Foreign Keys)

```
User (external)
  ├─> SyncConfig (1:N) - Multiple sync configs per user
  └─> Scrobbles (1:N)  - Many scrobbles per user

Artist (string)
  ├─> Scrobbles (1:N)  - Many scrobbles per artist
  ├─> Albums (1:N)     - Many albums per artist (implicit)
  └─> ImageCache (1:N) - Multiple size variants

Album (string)
  ├─> Scrobbles (1:N)  - Many scrobbles per album
  └─> ImageCache (1:N) - Multiple size variants

Track (string)
  └─> Scrobbles (1:N)  - Many scrobbles per track
```

**Note:** No foreign keys or normalization. Artist/album/track are denormalized strings in the scrobbles table.

## Data Quality Issues

### Inconsistencies Across Sources
- **Artist name variations:** "Björk" vs "Bjork", "The Beatles" vs "Beatles"
- **Album name differences:** "OK Computer" vs "OK Computer [Collector's Edition]"
- **Missing metadata:** ListenBrainz often has NULL albums
- **Different capitalization:** "OK Computer" vs "OK computer"

### Deduplication Challenges
- **Unique constraint:** Only prevents exact duplicates (same artist, track, timestamp, source)
- **Near-duplicates:** Same listen imported from both Last.fm and ListenBrainz with slightly different timestamps
- **Re-scrobbles:** Legitimately listening to same song twice in short period

### Missing Data
- **Albums:** ~20-30% of scrobbles have NULL album
- **Source IDs:** Some older scrobbles may have NULL source_id
- **Track duration:** Not stored (would be useful for session detection)
- **MusicBrainz IDs:** No MBID fields (prevents accurate linking)

## Aggregation Patterns

### Current Query Patterns

#### Top Artists
```sql
SELECT artist, COUNT(*) as count 
FROM scrobbles 
WHERE timestamp >= ?1 AND timestamp <= ?2 
GROUP BY artist 
ORDER BY count DESC 
LIMIT ?3
```

**Complexity:** O(n) full table scan with WHERE filter + GROUP BY
**Scale:** ~100ms for 100k scrobbles, ~1s for 1M scrobbles

#### Top Tracks
```sql
SELECT artist, track, COUNT(*) as count 
FROM scrobbles 
WHERE timestamp >= ?1 AND timestamp <= ?2 
GROUP BY artist, track 
ORDER BY count DESC 
LIMIT ?3
```

**Complexity:** O(n) + GROUP BY on two columns
**Scale:** Similar to top artists but higher cardinality

#### Top Albums
```sql
SELECT artist, album, COUNT(*) as count 
FROM scrobbles 
WHERE album IS NOT NULL 
  AND timestamp >= ?1 AND timestamp <= ?2 
GROUP BY artist, album 
ORDER BY count DESC 
LIMIT ?3
```

**Complexity:** O(n) with NULL filter
**Scale:** Slightly faster due to NULL filter reducing working set

#### Scrobbles Per Day
```sql
SELECT strftime('%Y-%m-%d', datetime(timestamp, 'unixepoch')) as day, 
       COUNT(*) as count 
FROM scrobbles 
WHERE timestamp >= ?1 AND timestamp <= ?2 
GROUP BY day 
ORDER BY day ASC
```

**Complexity:** O(n) with date function + GROUP BY
**Scale:** Bottleneck is `strftime` function on every row

### Performance Characteristics

| Query Type | Typical Data Size | Current Performance | Bottleneck |
|------------|-------------------|---------------------|------------|
| Top Artists (all-time) | 100k scrobbles | ~100ms | Full table scan |
| Top Tracks (all-time) | 100k scrobbles | ~150ms | High cardinality GROUP BY |
| Top Albums (all-time) | 100k scrobbles | ~80ms | NULL filter helps |
| Scrobbles Per Day (all-time) | 100k scrobbles | ~200ms | Date function overhead |
| Timeline (paginated) | 100 scrobbles | ~5ms | Indexed timestamp |

## Data Volume Estimates

### Typical User (Active Listener)
- **Scrobbles/day:** 50-150 (3-10 hours of music)
- **Scrobbles/year:** 18,000-55,000
- **Years of history:** 5-15 years
- **Total scrobbles:** 90,000-825,000
- **Unique artists:** 1,000-5,000
- **Unique tracks:** 10,000-50,000
- **Unique albums:** 2,000-10,000

### Database Size Estimates
- **Per scrobble:** ~200 bytes (including indexes)
- **100k scrobbles:** ~20 MB
- **500k scrobbles:** ~100 MB
- **1M scrobbles:** ~200 MB

### Image Cache
- **Per entry:** ~150 bytes (just URLs, not images)
- **1000 artists:** ~150 KB
- **10000 tracks:** ~1.5 MB

## Timezone Handling

### Current Implementation
- **Storage:** All timestamps stored as Unix epoch (UTC)
- **Display:** No timezone conversion (assumes UTC everywhere)
- **Import:** Last.fm and ListenBrainz provide UTC timestamps
- **Reports:** Date ranges computed in UTC

### Limitations
- **No user timezone:** Cannot show "listens by hour of my day"
- **Date boundaries:** "Today" is UTC today, not user's local today
- **Time-of-day analysis:** Would show UTC hours, not user's local hours

### Timezone Data Available
- None currently stored
- Could be added to sync_config or as global setting
- Would need to store timezone offset or IANA timezone name

## Future Data Model Extensions

### Potential Additions

#### Separate Entity Tables
```sql
CREATE TABLE artists (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    musicbrainz_id TEXT,
    image_url TEXT,
    tags TEXT  -- JSON array
);

CREATE TABLE albums (
    id INTEGER PRIMARY KEY,
    artist_id INTEGER REFERENCES artists(id),
    name TEXT NOT NULL,
    musicbrainz_id TEXT,
    release_year INTEGER,
    image_url TEXT
);

CREATE TABLE tracks (
    id INTEGER PRIMARY KEY,
    artist_id INTEGER REFERENCES artists(id),
    album_id INTEGER REFERENCES albums(id),
    name TEXT NOT NULL,
    duration_ms INTEGER,
    musicbrainz_id TEXT
);

-- Update scrobbles to reference normalized entities
ALTER TABLE scrobbles ADD COLUMN artist_id INTEGER REFERENCES artists(id);
ALTER TABLE scrobbles ADD COLUMN album_id INTEGER REFERENCES albums(id);
ALTER TABLE scrobbles ADD COLUMN track_id INTEGER REFERENCES tracks(id);
```

**Benefits:**
- Consistent artist/album/track names
- MusicBrainz linking for accurate metadata
- Genre/tag information
- Track durations for session detection

**Migration Challenges:**
- Deduplicating existing denormalized data
- Handling name variations and typos
- Mapping to MusicBrainz IDs (would need MB API calls)

#### Metadata Enrichment
```sql
CREATE TABLE track_metadata (
    track_id INTEGER PRIMARY KEY REFERENCES tracks(id),
    duration_ms INTEGER,
    musicbrainz_recording_id TEXT,
    spotify_id TEXT,
    acousticness REAL,
    danceability REAL,
    energy REAL,
    valence REAL
);
```

**Use Cases:**
- Session detection (need track durations)
- Mood/energy analysis
- Skip detection (if scrobble duration < track duration)

#### Computed Aggregates (Materialized Views)
```sql
CREATE TABLE artist_stats (
    artist TEXT PRIMARY KEY,
    total_scrobbles INTEGER,
    first_listen INTEGER,
    last_listen INTEGER,
    unique_tracks INTEGER,
    updated_at INTEGER
);

CREATE INDEX idx_artist_stats_scrobbles ON artist_stats(total_scrobbles DESC);
```

**Benefits:**
- Pre-computed stats for instant queries
- Incremental updates on new scrobbles
- Dramatically faster all-time reports

**Maintenance:**
- Triggers to update on INSERT
- Periodic full recomputation for accuracy

## Data Export Format

### Current Export (JSON)
```json
[
  {
    "id": 1,
    "artist": "Radiohead",
    "album": "OK Computer",
    "track": "Paranoid Android",
    "timestamp": "2024-12-15T18:30:00Z",
    "source": "lastfm",
    "source_id": "https://www.last.fm/music/Radiohead/_/Paranoid+Android"
  }
]
```

### Current Export (CSV)
```csv
timestamp,artist,album,track,source
2024-12-15T18:30:00Z,Radiohead,OK Computer,Paranoid Android,lastfm
```

### Potential Export Enhancements
- **Last.fm CSV format:** Compatible with Last.fm Backup tools
- **Maloja format:** Compatible with Maloja imports
- **JSON-LD:** Linked data format with MusicBrainz URIs
- **Parquet:** Columnar format for big data analysis

## Important Fields for New Features

### Session Detection
- **Required:** `timestamp` (already have)
- **Useful:** Track duration (NOT stored) - would need to fetch from external APIs
- **Workaround:** Use fixed gap (30-45 min) between scrobbles

### Novelty Tracking
- **Required:** `artist`, `track`, `timestamp` (already have)
- **Method:** Window functions to detect "first time ever listened"

### Time-of-Day Analysis
- **Required:** `timestamp` (already have)
- **Missing:** User timezone setting (currently assumes UTC)
- **Workaround:** Extract hour from UTC timestamp (less useful)

### Artist Transitions
- **Required:** `artist`, `timestamp` (already have), session boundaries
- **Method:** Order by timestamp, detect transitions within sessions

### Diversity Metrics
- **Required:** `artist`, `track`, `timestamp` (already have)
- **Method:** Rolling windows with unique artist count, Shannon entropy

### Repeat vs. New Listen Ratio
- **Required:** `artist`, `track`, `timestamp` (already have)
- **Method:** Window function to mark first occurrence, then count by period
