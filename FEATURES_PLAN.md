# Features Implementation Plan - Footprints

## Overview

This document provides detailed implementation plans for the top 5 selected features:
1. **Listening Sessions** (Phase 1 - This PR)
2. **Time-of-Day Heatmap** (Phase 1 - This PR)
3. Novelty vs. Re-listen (Phase 2)
4. Artist Transitions (Phase 2)
5. Diversity/Entropy Trend (Phase 3)

---

## Phase 1: Features 1 & 2 (This PR)

### Feature 1: Listening Sessions Report

#### User Story
As a music listener, I want to see how I listen to music (long focused sessions vs. short bursts) so I can understand my listening patterns and discover my longest listening sessions.

#### API Contract

##### Endpoint: `GET /api/reports/sessions`

**Query Parameters:**
- `start` (optional): RFC3339 date, start of range
- `end` (optional): RFC3339 date, end of range
- `gap_minutes` (optional, default: 45): Inactivity gap threshold
- `source` (optional): Filter by "lastfm" or "listenbrainz"
- `min_tracks` (optional, default: 2): Minimum tracks per session

**Response:**
```json
{
  "sessions": [
    {
      "id": "session_123",
      "start_time": "2024-12-15T18:00:00Z",
      "end_time": "2024-12-15T21:30:00Z",
      "duration_minutes": 210,
      "track_count": 42,
      "unique_artists": 8,
      "tracks": [
        {
          "artist": "Radiohead",
          "track": "Paranoid Android",
          "timestamp": "2024-12-15T18:00:00Z"
        }
      ]
    }
  ],
  "summary": {
    "total_sessions": 156,
    "avg_duration_minutes": 68.5,
    "avg_tracks_per_session": 15.2,
    "longest_session_minutes": 345,
    "total_listening_hours": 178.2
  },
  "distribution": {
    "by_duration": {
      "0-30": 45,
      "30-60": 52,
      "60-120": 38,
      "120-180": 15,
      "180+": 6
    },
    "by_track_count": {
      "2-10": 58,
      "10-20": 47,
      "20-30": 31,
      "30-50": 15,
      "50+": 5
    }
  },
  "sessions_per_day": [
    { "date": "2024-12-15", "count": 3 },
    { "date": "2024-12-16", "count": 2 }
  ]
}
```

##### Endpoint: `GET /api/reports/sessions/:session_id`

**Response:**
```json
{
  "session_id": "session_123",
  "start_time": "2024-12-15T18:00:00Z",
  "end_time": "2024-12-15T21:30:00Z",
  "duration_minutes": 210,
  "track_count": 42,
  "unique_artists": 8,
  "tracks": [
    {
      "artist": "Radiohead",
      "album": "OK Computer",
      "track": "Paranoid Android",
      "timestamp": "2024-12-15T18:00:00Z",
      "gap_after_minutes": 4
    }
  ]
}
```

#### Database Changes

**No new tables needed** - compute sessions on-demand from scrobbles.

**New Indexes:**
```sql
-- Already exists: idx_timestamp ON scrobbles(timestamp DESC)
-- Consider adding if performance issues:
CREATE INDEX IF NOT EXISTS idx_timestamp_source ON scrobbles(timestamp, source);
```

**Caching Strategy:**
- Server-side cache: Store computed sessions for common parameters (last 30 days, default gap)
- Cache key: `sessions:{start}:{end}:{gap}:{source}`
- TTL: 1 hour (invalidate on new scrobbles import)
- Implementation: In-memory HashMap with TTL, or extend image_cache table

#### Rust Implementation

**New Module:** `src/reports/sessions.rs`

```rust
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,  // Format: "session_{timestamp}"
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_minutes: i64,
    pub track_count: usize,
    pub unique_artists: usize,
    pub tracks: Vec<SessionTrack>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionTrack {
    pub artist: String,
    pub album: Option<String>,
    pub track: String,
    pub timestamp: DateTime<Utc>,
    pub gap_after_minutes: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsSummary {
    pub total_sessions: usize,
    pub avg_duration_minutes: f64,
    pub avg_tracks_per_session: f64,
    pub longest_session_minutes: i64,
    pub total_listening_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsReport {
    pub sessions: Vec<Session>,
    pub summary: SessionsSummary,
    pub distribution: SessionDistribution,
    pub sessions_per_day: Vec<DayCount>,
}

pub fn detect_sessions(
    scrobbles: Vec<Scrobble>, 
    gap_threshold_minutes: i64
) -> Vec<Session> {
    // Algorithm:
    // 1. Sort scrobbles by timestamp
    // 2. Iterate, compute gap between consecutive scrobbles
    // 3. Start new session when gap > threshold
    // 4. Build Session objects
}

pub fn generate_sessions_report(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    gap_minutes: i64,
    source: Option<String>,
    min_tracks: usize,
) -> Result<SessionsReport> {
    // 1. Fetch scrobbles in range (all, no limit)
    // 2. Filter by source if specified
    // 3. Detect sessions
    // 4. Filter by min_tracks
    // 5. Compute summary and distribution
    // 6. Return report
}
```

**Algorithm Details:**

```rust
pub fn detect_sessions(
    mut scrobbles: Vec<Scrobble>, 
    gap_threshold_minutes: i64
) -> Vec<Session> {
    if scrobbles.is_empty() {
        return Vec::new();
    }
    
    // Sort by timestamp ascending
    scrobbles.sort_by_key(|s| s.timestamp);
    
    let gap_threshold = Duration::minutes(gap_threshold_minutes);
    let mut sessions = Vec::new();
    let mut current_session_tracks = Vec::new();
    
    for (i, scrobble) in scrobbles.iter().enumerate() {
        if current_session_tracks.is_empty() {
            // Start first session
            current_session_tracks.push(scrobble.clone());
        } else {
            // Check gap from last track
            let last_track = current_session_tracks.last().unwrap();
            let gap = scrobble.timestamp.signed_duration_since(last_track.timestamp);
            
            if gap > gap_threshold {
                // End current session, start new one
                sessions.push(build_session(current_session_tracks));
                current_session_tracks = vec![scrobble.clone()];
            } else {
                // Continue current session
                current_session_tracks.push(scrobble.clone());
            }
        }
    }
    
    // Don't forget last session
    if !current_session_tracks.is_empty() {
        sessions.push(build_session(current_session_tracks));
    }
    
    sessions
}

fn build_session(tracks: Vec<Scrobble>) -> Session {
    let start_time = tracks.first().unwrap().timestamp;
    let end_time = tracks.last().unwrap().timestamp;
    let duration_minutes = end_time.signed_duration_since(start_time).num_minutes();
    
    let unique_artists: HashSet<String> = tracks.iter()
        .map(|t| t.artist.clone())
        .collect();
    
    let session_tracks: Vec<SessionTrack> = tracks.windows(2)
        .map(|window| {
            let gap = window[1].timestamp.signed_duration_since(window[0].timestamp).num_minutes();
            SessionTrack {
                artist: window[0].artist.clone(),
                album: window[0].album.clone(),
                track: window[0].track.clone(),
                timestamp: window[0].timestamp,
                gap_after_minutes: Some(gap),
            }
        })
        .collect();
    
    // Add last track (no gap after)
    let last = tracks.last().unwrap();
    session_tracks.push(SessionTrack {
        artist: last.artist.clone(),
        album: last.album.clone(),
        track: last.track.clone(),
        timestamp: last.timestamp,
        gap_after_minutes: None,
    });
    
    Session {
        id: format!("session_{}", start_time.timestamp()),
        start_time,
        end_time,
        duration_minutes,
        track_count: tracks.len(),
        unique_artists: unique_artists.len(),
        tracks: session_tracks,
    }
}
```

#### UI Components

**New Page:** `templates/sessions.html` (or add to index.html as section)

**Components:**
1. **Filters Panel:**
   - Date range picker
   - Gap threshold slider (15/30/45/60 min)
   - Source filter (all/lastfm/listenbrainz)
   - Min tracks filter

2. **Summary Cards:**
   - Total sessions
   - Average duration
   - Longest session
   - Total listening hours

3. **Charts:**
   - **Duration Distribution:** Histogram (x=duration buckets, y=session count)
   - **Sessions Per Day:** Line chart (x=date, y=count)
   - **Longest Sessions:** Top 10 table with drilldown

4. **Session List:**
   - Paginated table: date, duration, tracks, unique artists
   - Click row → expand to show full track list

**JavaScript (Vanilla):**
```javascript
async function loadSessions() {
    const start = document.getElementById('start-date').value;
    const end = document.getElementById('end-date').value;
    const gap = document.getElementById('gap-slider').value;
    
    const response = await fetch(
        `/api/reports/sessions?start=${start}&end=${end}&gap_minutes=${gap}`
    );
    const data = await response.json();
    
    renderSummary(data.summary);
    renderDistribution(data.distribution);
    renderSessionsList(data.sessions);
}

function renderDistribution(distribution) {
    // Use Chart.js or vanilla canvas
    const canvas = document.getElementById('duration-chart');
    // ... draw histogram
}
```

#### Tests

**Unit Tests:** `src/reports/sessions/tests.rs`

```rust
#[test]
fn test_session_detection_basic() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z"),
        test_scrobble("2024-01-01T10:05:00Z"),
        test_scrobble("2024-01-01T10:10:00Z"),
        // 60 min gap
        test_scrobble("2024-01-01T11:10:00Z"),
        test_scrobble("2024-01-01T11:15:00Z"),
    ];
    
    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].track_count, 3);
    assert_eq!(sessions[1].track_count, 2);
}

#[test]
fn test_session_detection_single_track() {
    let scrobbles = vec![test_scrobble("2024-01-01T10:00:00Z")];
    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].track_count, 1);
}

#[test]
fn test_session_duration_calculation() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z"),
        test_scrobble("2024-01-01T10:30:00Z"),
        test_scrobble("2024-01-01T11:00:00Z"),
    ];
    
    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions[0].duration_minutes, 60);
}

#[test]
fn test_session_unique_artists() {
    let scrobbles = vec![
        test_scrobble_with_artist("2024-01-01T10:00:00Z", "Artist A"),
        test_scrobble_with_artist("2024-01-01T10:05:00Z", "Artist B"),
        test_scrobble_with_artist("2024-01-01T10:10:00Z", "Artist A"),
    ];
    
    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions[0].unique_artists, 2);
}

#[test]
fn test_empty_scrobbles() {
    let sessions = detect_sessions(vec![], 45);
    assert_eq!(sessions.len(), 0);
}

#[test]
fn test_session_midnight_boundary() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T23:45:00Z"),
        test_scrobble("2024-01-02T00:15:00Z"), // 30 min gap
    ];
    
    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1); // Same session across midnight
}
```

**Integration Test:**
```rust
#[tokio::test]
async fn test_sessions_endpoint() {
    // Setup test server with test data
    // Call /api/reports/sessions
    // Assert response structure
}
```

#### Acceptance Criteria

✅ **Must Have:**
- [ ] Detect sessions using configurable gap threshold
- [ ] Return summary stats (total, avg duration, etc.)
- [ ] Return distribution histograms
- [ ] Handle edge cases (empty data, single track, midnight boundary)
- [ ] Unit tests with >90% coverage
- [ ] API endpoint responds in <500ms for 10k scrobbles
- [ ] UI displays sessions with filters

✅ **Nice to Have:**
- [ ] Server-side caching for common queries
- [ ] Drilldown to individual session
- [ ] Export sessions as CSV/JSON

#### Edge Cases Handled

1. **Empty scrobbles:** Return empty sessions array
2. **Single track:** Create session with 1 track, duration=0
3. **Sessions spanning midnight:** Treat as single session (no artificial date boundary)
4. **Mixed sources:** Allow filtering by source, or combine all
5. **Very long sessions:** No upper limit, but flag sessions >6 hours as potential errors
6. **Gap exactly at threshold:** Treat as new session (gap > threshold, not >=)

---

### Feature 2: Time-of-Day / Week Heatmap

#### User Story
As a music listener, I want to see when I listen to music throughout the week so I can understand my listening routines and compare different time periods.

#### API Contract

##### Endpoint: `GET /api/reports/heatmap`

**Query Parameters:**
- `start` (optional): RFC3339 date, start of range
- `end` (optional): RFC3339 date, end of range
- `timezone` (optional, default: "UTC"): IANA timezone (e.g., "America/New_York")
- `normalize` (optional, default: false): Normalize by number of weeks

**Response:**
```json
{
  "heatmap": [
    {
      "weekday": 0,
      "hour": 0,
      "count": 15,
      "normalized": 2.5
    },
    {
      "weekday": 0,
      "hour": 1,
      "count": 8,
      "normalized": 1.3
    }
  ],
  "summary": {
    "total_scrobbles": 5420,
    "weeks_in_range": 12,
    "peak_hour": 21,
    "peak_weekday": 5,
    "peak_count": 145
  },
  "weekday_totals": [
    { "weekday": 0, "name": "Monday", "count": 780 },
    { "weekday": 1, "name": "Tuesday", "count": 820 }
  ],
  "hour_totals": [
    { "hour": 0, "count": 45 },
    { "hour": 1, "count": 32 }
  ]
}
```

**Note:** 
- `weekday`: 0=Monday, 6=Sunday (ISO 8601)
- `hour`: 0-23 (24-hour format)
- `normalized`: count / number of weeks (for fair comparison across periods)

##### Endpoint: `GET /api/reports/heatmap/compare`

**Query Parameters:**
- `start1`, `end1`: First period
- `start2`, `end2`: Second period
- `timezone`: Timezone for both periods

**Response:**
```json
{
  "period1": { /* heatmap response */ },
  "period2": { /* heatmap response */ },
  "diff": [
    {
      "weekday": 0,
      "hour": 0,
      "diff": -2.3,
      "percent_change": -15.2
    }
  ]
}
```

#### Database Changes

**No new tables needed** - compute heatmap on-demand.

**New Configuration:**

Add timezone setting to database (global or per-user):

**Option 1: Global Setting (Simpler)**
- Store in environment variable or config file
- Single timezone for entire instance

**Option 2: User Timezone (Future-proof)**
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

INSERT INTO settings (key, value, updated_at) 
VALUES ('user_timezone', 'UTC', strftime('%s', 'now'));
```

**For this PR:** Use Option 1 (environment variable)

**Caching Strategy:**
- Cache key: `heatmap:{start}:{end}:{timezone}`
- TTL: 1 hour
- Invalidate on new scrobbles

#### Rust Implementation

**New Module:** `src/reports/heatmap.rs`

```rust
use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub weekday: u32,  // 0=Mon, 6=Sun
    pub hour: u32,     // 0-23
    pub count: i64,
    pub normalized: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeatmapSummary {
    pub total_scrobbles: i64,
    pub weeks_in_range: i64,
    pub peak_hour: u32,
    pub peak_weekday: u32,
    pub peak_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeatmapReport {
    pub heatmap: Vec<HeatmapCell>,
    pub summary: HeatmapSummary,
    pub weekday_totals: Vec<DayCount>,
    pub hour_totals: Vec<HourCount>,
}

pub fn generate_heatmap(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    timezone: Tz,
    normalize: bool,
) -> Result<HeatmapReport> {
    // 1. Fetch scrobbles in range
    let scrobbles = crate::db::get_scrobbles_in_range(pool, start, end)?;
    
    // 2. Build heatmap matrix (7 x 24)
    let mut heatmap_matrix: HashMap<(u32, u32), i64> = HashMap::new();
    
    for scrobble in &scrobbles {
        // Convert to user timezone
        let local_time = scrobble.timestamp.with_timezone(&timezone);
        let weekday = local_time.weekday().num_days_from_monday(); // 0=Mon
        let hour = local_time.hour();
        
        *heatmap_matrix.entry((weekday, hour)).or_insert(0) += 1;
    }
    
    // 3. Compute weeks in range (for normalization)
    let weeks_in_range = if let (Some(s), Some(e)) = (start, end) {
        (e - s).num_weeks().max(1)
    } else {
        1
    };
    
    // 4. Build heatmap cells
    let mut heatmap = Vec::new();
    for weekday in 0..7 {
        for hour in 0..24 {
            let count = *heatmap_matrix.get(&(weekday, hour)).unwrap_or(&0);
            let normalized = if normalize {
                count as f64 / weeks_in_range as f64
            } else {
                count as f64
            };
            
            heatmap.push(HeatmapCell {
                weekday,
                hour,
                count,
                normalized,
            });
        }
    }
    
    // 5. Compute summary
    let peak_cell = heatmap.iter().max_by_key(|c| c.count).unwrap();
    let summary = HeatmapSummary {
        total_scrobbles: scrobbles.len() as i64,
        weeks_in_range,
        peak_hour: peak_cell.hour,
        peak_weekday: peak_cell.weekday,
        peak_count: peak_cell.count,
    };
    
    // 6. Compute totals
    let weekday_totals = compute_weekday_totals(&heatmap);
    let hour_totals = compute_hour_totals(&heatmap);
    
    Ok(HeatmapReport {
        heatmap,
        summary,
        weekday_totals,
        hour_totals,
    })
}
```

**Dependency Addition:**

Add to `Cargo.toml`:
```toml
chrono-tz = "0.8"  # For timezone conversion
```

#### UI Components

**New Page:** `/reports/heatmap` section in index.html

**Components:**

1. **Filters Panel:**
   - Date range picker
   - Timezone selector (dropdown with common timezones)
   - Normalize toggle
   - Compare periods button

2. **Heatmap Visualization:**
   - 7x24 grid (weekdays x hours)
   - Color intensity based on count (light → dark gradient)
   - Hover to show exact count
   - Click cell to see scrobbles in that hour

3. **Summary Cards:**
   - Total scrobbles
   - Peak time (e.g., "Fridays at 9 PM")
   - Weeks analyzed

4. **Side Charts:**
   - Weekday totals (bar chart)
   - Hour totals (line chart)

**HTML/CSS:**
```html
<div id="heatmap-container">
  <table class="heatmap-table">
    <thead>
      <tr>
        <th></th>
        <th>Mon</th>
        <th>Tue</th>
        <th>Wed</th>
        <th>Thu</th>
        <th>Fri</th>
        <th>Sat</th>
        <th>Sun</th>
      </tr>
    </thead>
    <tbody id="heatmap-body">
      <!-- Generate 24 rows (hours) with 7 cells (days) -->
    </tbody>
  </table>
</div>
```

**CSS:**
```css
.heatmap-cell {
  width: 40px;
  height: 30px;
  border: 1px solid #eee;
  cursor: pointer;
  transition: transform 0.2s;
}

.heatmap-cell:hover {
  transform: scale(1.1);
  z-index: 10;
}

/* Color gradient: 0-10-50-100-200+ scrobbles */
.heatmap-cell[data-count="0"] { background: #f0f0f0; }
.heatmap-cell[data-count="1"] { background: #d6e7ff; }
.heatmap-cell[data-count="2"] { background: #aaccff; }
.heatmap-cell[data-count="3"] { background: #5599ff; }
.heatmap-cell[data-count="4"] { background: #2266cc; }
.heatmap-cell[data-count="5"] { background: #003399; }
```

**JavaScript:**
```javascript
async function loadHeatmap() {
    const start = document.getElementById('start-date').value;
    const end = document.getElementById('end-date').value;
    const timezone = document.getElementById('timezone-select').value;
    const normalize = document.getElementById('normalize-toggle').checked;
    
    const response = await fetch(
        `/api/reports/heatmap?start=${start}&end=${end}&timezone=${timezone}&normalize=${normalize}`
    );
    const data = await response.json();
    
    renderHeatmap(data.heatmap);
    renderSummary(data.summary);
}

function renderHeatmap(heatmap) {
    const tbody = document.getElementById('heatmap-body');
    tbody.innerHTML = '';
    
    // Group by hour (rows)
    for (let hour = 0; hour < 24; hour++) {
        const row = document.createElement('tr');
        
        // Hour label
        const hourCell = document.createElement('th');
        hourCell.textContent = `${hour}:00`;
        row.appendChild(hourCell);
        
        // Weekday cells
        for (let weekday = 0; weekday < 7; weekday++) {
            const cell = heatmap.find(c => c.weekday === weekday && c.hour === hour);
            const td = document.createElement('td');
            td.className = 'heatmap-cell';
            td.dataset.count = getColorBucket(cell.count);
            td.title = `${cell.count} scrobbles`;
            td.onclick = () => drilldownHeatmapCell(weekday, hour);
            row.appendChild(td);
        }
        
        tbody.appendChild(row);
    }
}

function getColorBucket(count) {
    if (count === 0) return '0';
    if (count < 10) return '1';
    if (count < 50) return '2';
    if (count < 100) return '3';
    if (count < 200) return '4';
    return '5';
}
```

#### Tests

**Unit Tests:** `src/reports/heatmap/tests.rs`

```rust
#[test]
fn test_heatmap_basic() {
    // Create scrobbles at specific hours/days
    let scrobbles = vec![
        test_scrobble_at("2024-01-01T09:00:00Z"), // Monday 9am UTC
        test_scrobble_at("2024-01-01T09:30:00Z"), // Monday 9am UTC
        test_scrobble_at("2024-01-02T14:00:00Z"), // Tuesday 2pm UTC
    ];
    
    let heatmap = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false);
    
    let monday_9am = heatmap.iter()
        .find(|c| c.weekday == 0 && c.hour == 9)
        .unwrap();
    assert_eq!(monday_9am.count, 2);
    
    let tuesday_2pm = heatmap.iter()
        .find(|c| c.weekday == 1 && c.hour == 14)
        .unwrap();
    assert_eq!(tuesday_2pm.count, 1);
}

#[test]
fn test_heatmap_timezone_conversion() {
    // Create scrobble at midnight UTC
    let scrobbles = vec![
        test_scrobble_at("2024-01-01T00:00:00Z"), // Monday midnight UTC
    ];
    
    // Convert to EST (UTC-5)
    let heatmap = build_heatmap_from_scrobbles(
        scrobbles, 
        "America/New_York".parse::<Tz>().unwrap(),
        false
    );
    
    // Should appear at Sunday 7pm EST (previous day)
    let sunday_7pm = heatmap.iter()
        .find(|c| c.weekday == 6 && c.hour == 19)
        .unwrap();
    assert_eq!(sunday_7pm.count, 1);
}

#[test]
fn test_heatmap_normalization() {
    let scrobbles = vec![
        test_scrobble_at("2024-01-01T09:00:00Z"),
        test_scrobble_at("2024-01-08T09:00:00Z"), // Same time, 1 week later
    ];
    
    let heatmap = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, true);
    
    let monday_9am = heatmap.iter()
        .find(|c| c.weekday == 0 && c.hour == 9)
        .unwrap();
    
    // 2 scrobbles over 2 weeks = 1.0 per week
    assert_eq!(monday_9am.normalized, 1.0);
}

#[test]
fn test_empty_heatmap() {
    let heatmap = build_heatmap_from_scrobbles(vec![], Tz::UTC, false);
    assert_eq!(heatmap.len(), 7 * 24); // Full matrix
    assert!(heatmap.iter().all(|c| c.count == 0));
}
```

#### Acceptance Criteria

✅ **Must Have:**
- [ ] Generate 7x24 heatmap matrix
- [ ] Support timezone conversion (at least UTC + 5 common zones)
- [ ] Normalize by number of weeks (optional)
- [ ] Return weekday and hour totals
- [ ] Handle edge cases (empty data, DST transitions)
- [ ] Unit tests with >90% coverage
- [ ] API endpoint responds in <300ms for 10k scrobbles
- [ ] UI displays heatmap with color gradient

✅ **Nice to Have:**
- [ ] Period comparison (two heatmaps side-by-side)
- [ ] Drilldown to see scrobbles in specific hour
- [ ] "Most characteristic hours" (vs. global average)
- [ ] Timezone autodetection from browser

#### Edge Cases Handled

1. **Empty scrobbles:** Return heatmap with all zeros
2. **DST transitions:** Handle via chrono-tz (automatically handled)
3. **Timezone parsing errors:** Fallback to UTC
4. **Single week of data:** Normalization divides by 1 (no change)
5. **Leap seconds:** Ignored (not relevant for hourly aggregation)
6. **Date range across years:** Handle correctly (no artificial year boundary)

---

## Phase 2: Features 3 & 4 (Future PR)

### Feature 3: Novelty vs. Re-listen Ratio

*(Detailed plan deferred to Phase 2 PR)*

**Key Components:**
- Window function or iterative marking of "first occurrence"
- Per-period aggregation (day/week/month)
- New vs. repeat classification
- Discovery timeline visualization

**Database Considerations:**
- Materialized view for first-occurrence dates?
- Or compute on-demand with window functions

### Feature 4: Artist Transitions

*(Detailed plan deferred to Phase 2 PR)*

**Key Components:**
- Reuse session detection from Feature 1
- Transition counting within sessions
- Network graph data structure
- Sankey diagram support

---

## Phase 3: Feature 5 (Future PR)

### Feature 5: Diversity / Entropy Trend

*(Detailed plan deferred to Phase 3 PR)*

**Key Components:**
- Rolling window computations
- Shannon entropy calculation
- Gini coefficient
- Unique artists per N scrobbles metric

---

## Performance Optimizations

### Database Indexes

**Existing:**
- `idx_timestamp` ON `scrobbles(timestamp DESC)` ✓
- `idx_artist` ON `scrobbles(artist)` ✓

**New (if needed):**
```sql
-- For session detection with source filter
CREATE INDEX IF NOT EXISTS idx_timestamp_source 
ON scrobbles(timestamp, source);

-- For heatmap hour extraction (probably not needed)
-- SQLite function-based indexes not supported
```

### Caching Strategy

**Implementation:** Simple in-memory cache with TTL

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};

pub struct ReportCache {
    cache: Arc<RwLock<HashMap<String, CachedReport>>>,
}

struct CachedReport {
    data: String,  // Serialized JSON
    cached_at: DateTime<Utc>,
    ttl_seconds: i64,
}

impl ReportCache {
    pub fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().unwrap();
        if let Some(cached) = cache.get(key) {
            let age = Utc::now().signed_duration_since(cached.cached_at).num_seconds();
            if age < cached.ttl_seconds {
                return Some(cached.data.clone());
            }
        }
        None
    }
    
    pub fn set(&self, key: String, data: String, ttl_seconds: i64) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(key, CachedReport {
            data,
            cached_at: Utc::now(),
            ttl_seconds,
        });
    }
    
    pub fn invalidate_all(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}
```

**Usage in API:**
```rust
async fn get_sessions_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SessionsParams>,
) -> Result<Json<SessionsReport>, StatusCode> {
    let cache_key = format!("sessions:{}:{}:{}", 
        params.start.unwrap_or_default(),
        params.end.unwrap_or_default(),
        params.gap_minutes
    );
    
    // Try cache first
    if let Some(cached) = state.cache.get(&cache_key) {
        if let Ok(report) = serde_json::from_str(&cached) {
            return Ok(Json(report));
        }
    }
    
    // Compute report
    let report = reports::generate_sessions_report(
        &state.pool,
        params.start,
        params.end,
        params.gap_minutes,
        params.source,
        params.min_tracks.unwrap_or(2),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Cache result
    if let Ok(json) = serde_json::to_string(&report) {
        state.cache.set(cache_key, json, 3600); // 1 hour TTL
    }
    
    Ok(Json(report))
}
```

### Precomputation Strategy (Future)

For large datasets (>500k scrobbles), consider:

**Daily Aggregates Table:**
```sql
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY,
    total_scrobbles INTEGER,
    unique_artists INTEGER,
    unique_tracks INTEGER,
    top_artists TEXT,  -- JSON
    computed_at INTEGER
);
```

**Benefits:**
- Faster monthly/yearly reports (aggregate daily stats instead of raw scrobbles)
- Incremental updates (only recompute today's stats)

**Tradeoffs:**
- Additional storage
- More complex invalidation logic
- Stale data until next computation

**Recommendation:** Defer until proven performance bottleneck.

---

## Documentation Updates

### README.md Updates

Add new section after "API Endpoints":

```markdown
## Reports & Charts

### Listening Sessions
Analyze your listening behavior by detecting continuous listening sessions.

- **Session Detection**: Automatically groups scrobbles into sessions using inactivity gaps (default: 45 minutes)
- **Statistics**: Average session duration, tracks per session, longest sessions
- **Visualizations**: Duration distribution, sessions per day, drilldown to track sequences
- **API**: `GET /api/reports/sessions?start={date}&end={date}&gap_minutes={N}`

### Time-of-Day Heatmap
Discover when you listen to music throughout the week.

- **Heatmap Visualization**: 7x24 grid showing listening intensity by hour and weekday
- **Timezone Support**: Convert timestamps to your local timezone
- **Peak Times**: Identifies your most active listening hours
- **Comparison**: Compare two time periods side-by-side
- **API**: `GET /api/reports/heatmap?start={date}&end={date}&timezone={tz}`

### Configuration

Add timezone setting to your `.env` file:
```env
USER_TIMEZONE=America/New_York
```

Supported timezones: Any IANA timezone (e.g., "America/New_York", "Europe/London", "Asia/Tokyo")
```

### API Documentation

Create `docs/API.md` (if doesn't exist) with:

```markdown
## Reports API

### Sessions Report

**Endpoint:** `GET /api/reports/sessions`

**Parameters:**
- `start` (optional): RFC3339 date, start of range
- `end` (optional): RFC3339 date, end of range
- `gap_minutes` (optional, default: 45): Inactivity gap threshold
- `source` (optional): Filter by "lastfm" or "listenbrainz"
- `min_tracks` (optional, default: 2): Minimum tracks per session

**Example:**
```bash
curl "http://localhost:3000/api/reports/sessions?start=2024-01-01T00:00:00Z&end=2024-12-31T23:59:59Z&gap_minutes=30"
```

**Response:** See FEATURES_PLAN.md for full schema.

---

### Heatmap Report

**Endpoint:** `GET /api/reports/heatmap`

**Parameters:**
- `start` (optional): RFC3339 date
- `end` (optional): RFC3339 date
- `timezone` (optional, default: "UTC"): IANA timezone
- `normalize` (optional, default: false): Normalize by weeks

**Example:**
```bash
curl "http://localhost:3000/api/reports/heatmap?timezone=America/New_York&normalize=true"
```
```

---

## Testing Strategy

### Unit Tests
- Session detection logic (edge cases)
- Heatmap matrix building (timezone handling)
- Summary computations
- Distribution calculations

### Integration Tests
- API endpoints (request/response)
- Database queries (performance)
- Cache hit/miss behavior

### Manual Testing
- UI interactions (filters, drilldowns)
- Large datasets (100k+ scrobbles)
- Different timezones
- Edge cases (empty data, single scrobble)

### Performance Tests
```rust
#[test]
fn bench_session_detection_100k_scrobbles() {
    let scrobbles = generate_test_scrobbles(100_000);
    let start = Instant::now();
    let sessions = detect_sessions(scrobbles, 45);
    let duration = start.elapsed();
    assert!(duration.as_millis() < 500); // Must complete in <500ms
}
```

---

## Deployment Checklist

- [ ] Add chrono-tz dependency
- [ ] Implement session detection module
- [ ] Implement heatmap module
- [ ] Add API endpoints
- [ ] Add UI components
- [ ] Write unit tests (>90% coverage)
- [ ] Write integration tests
- [ ] Update README.md
- [ ] Create API documentation
- [ ] Test with real data (>10k scrobbles)
- [ ] Performance test (measure query times)
- [ ] Create PERF_NOTES.md
- [ ] Update .env.example with USER_TIMEZONE

---

## Next Steps (Phase 2 & 3)

### Phase 2:
1. Implement Novelty vs. Re-listen tracking
2. Implement Artist Transitions
3. Add network graph visualization support

### Phase 3:
1. Implement Diversity/Entropy metrics
2. Add rolling window computations
3. Consider materialized views for performance

### Future Enhancements:
- Genre tagging (requires Last.fm tag API integration)
- Album completion tracking (requires album metadata)
- Predictive features (burnout detection, recommendation)
