# Next Steps - Remaining Features

## Overview

This document outlines implementation plans for the remaining 3 features from the top 5 selected features, plus additional enhancements and future directions.

---

## Phase 2: Features 3 & 4 (Next PR)

### Feature 3: Novelty vs. Re-listen Ratio

**Priority**: High  
**Complexity**: Medium  
**Estimated Effort**: 8-12 hours

#### What It Does
Tracks the percentage of listens that are first-time tracks/artists versus repeats, broken down by time period (day/week/month).

#### Implementation Plan

**Module**: `src/reports/novelty/mod.rs`

**Core Algorithm**:
```rust
pub struct NoveltyReport {
    pub timeline: Vec<NoveltyPoint>,
    pub summary: NoveltySummary,
    pub new_artists_discovered: Vec<ArtistDiscovery>,
    pub top_comfort_tracks: Vec<(String, String, i64)>, // (artist, track, play_count)
}

pub struct NoveltyPoint {
    pub period: String,        // "2024-W01", "2024-12-15", etc.
    pub total_scrobbles: i64,
    pub new_tracks: i64,
    pub repeat_tracks: i64,
    pub new_artists: i64,
    pub repeat_artists: i64,
    pub novelty_ratio: f64,    // new / total
}
```

**Step 1: Mark First Occurrences**
```sql
-- Window function approach (requires SQLite 3.25+)
WITH first_occurrences AS (
    SELECT 
        artist,
        track,
        MIN(timestamp) as first_heard
    FROM scrobbles
    GROUP BY artist, track
)
SELECT 
    s.*,
    CASE WHEN s.timestamp = f.first_heard THEN 1 ELSE 0 END as is_first_listen
FROM scrobbles s
LEFT JOIN first_occurrences f ON s.artist = f.artist AND s.track = f.track
ORDER BY s.timestamp;
```

**Step 2: Group by Time Period**
- Daily: Use `strftime('%Y-%m-%d', ...)`
- Weekly: Use `strftime('%Y-W%W', ...)`
- Monthly: Use `strftime('%Y-%m', ...)`

**Step 3: Compute Ratios**
- Count new vs. repeat for each period
- Calculate percentage

**Database Considerations**:
- Consider materialized view for first-occurrence dates
- Or compute on-demand with window functions

**API Endpoint**:
```
GET /api/reports/novelty?start={date}&end={date}&granularity={day|week|month}
```

**Test Cases**:
- Empty database (all tracks are "new")
- First-ever scrobble for a track
- Repeated tracks in same day
- Tracks spanning multiple periods
- Artist-level novelty (distinct from track-level)

**Edge Cases**:
- Same track, different album (treat as same track? config option?)
- Renamed artists (would appear as new - unavoidable without MBID)
- Case sensitivity in artist/track names

---

### Feature 4: Artist Transitions (Flow Analysis)

**Priority**: Medium-High  
**Complexity**: Medium  
**Estimated Effort**: 6-10 hours

#### What It Does
Analyzes most common artist-to-artist transitions within listening sessions. Provides data for network graphs and Sankey diagrams.

#### Implementation Plan

**Module**: `src/reports/transitions/mod.rs`

**Core Algorithm**:
```rust
pub struct TransitionsReport {
    pub transitions: Vec<Transition>,
    pub top_transitions: Vec<Transition>,
    pub network_data: NetworkGraph,
    pub summary: TransitionsSummary,
}

pub struct Transition {
    pub from_artist: String,
    pub to_artist: String,
    pub count: i64,
    pub example_sessions: Vec<String>, // Session IDs
}

pub struct NetworkGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
```

**Step 1: Reuse Session Detection**
```rust
// Leverage existing sessions module
use crate::reports::sessions::detect_sessions;

let sessions = detect_sessions(scrobbles, gap_minutes);
```

**Step 2: Extract Transitions**
```rust
for session in sessions {
    for i in 0..session.tracks.len() - 1 {
        let from = &session.tracks[i].artist;
        let to = &session.tracks[i + 1].artist;
        
        // Skip self-transitions (optional, configurable)
        if from != to {
            transition_counts.entry((from, to)).or_insert(0) += 1;
        }
    }
}
```

**Step 3: Build Network Graph**
```rust
// Nodes: unique artists
// Edges: transitions with weights

for ((from, to), count) in transition_counts {
    nodes.insert(from);
    nodes.insert(to);
    edges.push(Edge {
        source: from,
        target: to,
        weight: count,
    });
}
```

**API Endpoint**:
```
GET /api/reports/transitions?start={date}&end={date}&gap_minutes={N}&min_count={N}
```

**Response Format** (compatible with D3.js, Cytoscape.js):
```json
{
  "nodes": [
    {"id": "Radiohead", "label": "Radiohead", "size": 150},
    {"id": "Björk", "label": "Björk", "size": 120}
  ],
  "edges": [
    {"source": "Radiohead", "target": "Björk", "weight": 25},
    {"source": "Björk", "target": "Radiohead", "weight": 18}
  ]
}
```

**Test Cases**:
- Single-artist sessions (no transitions)
- Self-transitions (A→A)
- Bidirectional transitions (A→B, B→A)
- Isolated artists (no transitions to/from)

---

## Phase 3: Feature 5 (Future PR)

### Feature 5: Diversity / Entropy Trend

**Priority**: Medium  
**Complexity**: Medium  
**Estimated Effort**: 8-10 hours

#### What It Does
Computes rolling diversity metrics to quantify listening breadth over time:
1. Unique artists per N listens
2. Shannon entropy per period
3. Gini coefficient (concentration)

#### Implementation Plan

**Module**: `src/reports/diversity/mod.rs`

**Metrics**:

1. **Unique Artists per N Listens**:
```rust
// Rolling window: count unique artists in last N scrobbles
let window_size = 100;
for i in window_size..scrobbles.len() {
    let window = &scrobbles[i-window_size..i];
    let unique = window.iter()
        .map(|s| &s.artist)
        .collect::<HashSet<_>>()
        .len();
    
    diversity_points.push((window.last().timestamp, unique));
}
```

2. **Shannon Entropy**:
```rust
// Per period (week/month):
let total: f64 = artist_counts.values().sum();
let entropy: f64 = artist_counts.values()
    .map(|&count| {
        let p = count as f64 / total;
        -p * p.log2()
    })
    .sum();

// Higher entropy = more diverse
// log2(n) is maximum entropy (uniform distribution)
```

3. **Gini Coefficient**:
```rust
// Measure of inequality in artist distribution
// 0 = perfect equality (all artists played equally)
// 1 = perfect inequality (one artist dominates)

let mut sorted_counts = artist_counts.values().collect::<Vec<_>>();
sorted_counts.sort();

let n = sorted_counts.len() as f64;
let sum: f64 = sorted_counts.iter().map(|&&x| x as f64).sum();

let gini = (2.0 / (n * sum)) 
    * sorted_counts.iter().enumerate()
        .map(|(i, &&x)| ((i + 1) as f64) * (x as f64))
        .sum::<f64>()
    - (n + 1.0) / n;
```

**API Endpoint**:
```
GET /api/reports/diversity?start={date}&end={date}&granularity={week|month}&window_size={N}
```

**UI Explanation**:
```
Your diversity score measures how evenly you distribute listens across artists.

- High diversity (H > 4.0): You listen to many artists equally
- Medium diversity (2.0 < H < 4.0): Mix of favorites and exploration
- Low diversity (H < 2.0): Concentrated on few favorites

This week's entropy: 3.2 (you listened to 45 artists)
Last week's entropy: 2.8 (you listened to 38 artists)
→ You're exploring more!
```

**Test Cases**:
- Uniform distribution (all artists equal) → max entropy
- Single artist (monopoly) → zero entropy, max Gini
- Rolling windows at dataset boundaries
- Empty periods

---

## UI/Frontend Development

### Priority Features for UI

1. **Sessions Report Page** (High Priority)
   - Summary cards (total sessions, avg duration, etc.)
   - Duration distribution histogram (Chart.js or D3)
   - Sessions timeline (line chart)
   - Sessions list (table with expand/collapse)
   - Filters: date range, gap threshold, source

2. **Heatmap Page** (High Priority)
   - 7×24 grid with color gradient
   - Timezone selector
   - Normalize toggle
   - Hover tooltips
   - Click cell → drilldown to scrobbles

3. **Novelty Dashboard** (Medium Priority)
   - Timeline chart (% new vs repeat)
   - New artists discovery list
   - "Comfort food" tracks
   - Period selector

4. **Transitions Network Visualization** (Medium Priority)
   - Force-directed graph (D3.js)
   - Or Sankey diagram
   - Interactive: click artist → highlight connections
   - Filter by transition count threshold

5. **Diversity Trends** (Lower Priority)
   - Multi-line chart (entropy, unique artists, Gini)
   - Explanation tooltips
   - Period comparison

### Recommended Approach

**Option 1: Vanilla JS + Chart.js** (consistent with current codebase)
- Pros: No build step, lightweight, easy to maintain
- Cons: More verbose, limited interactivity

**Option 2: Add Vue.js or React** (more powerful, but adds complexity)
- Pros: Better interactivity, component reuse
- Cons: Build step, larger bundle, learning curve

**Recommendation**: Start with Option 1 (vanilla + Chart.js), migrate to Option 2 if complexity grows.

---

## Additional Enhancements (Backlog)

### Near-Term (Low-Hanging Fruit)

1. **Listening Streaks** (1-2 hours)
   - Track consecutive days with N+ scrobbles
   - Calendar heatmap (GitHub-style)
   - Current streak vs longest streak

2. **Late Night Listening** (1 hour)
   - Filter scrobbles between midnight-5am
   - Top late-night artists/tracks
   - Compare with daytime listening

3. **Weekend vs Weekday** (1 hour)
   - Split by weekday/weekend
   - Compare top artists
   - Side-by-side charts

### Medium-Term (Requires External Data)

4. **Genre Journey** (4-6 hours)
   - Fetch genre tags from Last.fm API
   - Map artists → genres
   - Stacked area chart over time
   - **Dependency**: Last.fm tag API integration

5. **Album Completion Rate** (4-6 hours)
   - Fetch album track counts from MusicBrainz
   - Track which albums you've "completed"
   - Completion percentage per album
   - **Dependency**: MusicBrainz API integration

### Long-Term (Research/Experimental)

6. **Taste Drift Analysis** (8-12 hours)
   - Compute Jaccard similarity between periods
   - Measure how taste changes over time
   - Predict future preferences

7. **Mood/Energy Analysis** (12-16 hours)
   - Fetch audio features from Spotify API
   - Track valence, energy, danceability over time
   - Mood calendar
   - **Dependency**: Spotify API integration

8. **Social Features** (20+ hours)
   - Compare listening with friends
   - Shared artists/tracks
   - Listening compatibility score
   - **Dependency**: Multi-user support

---

## Infrastructure Improvements

### Performance Optimizations

1. **Server-Side Caching** (2-3 hours)
   - Implement in-memory cache with TTL
   - Cache keys based on query parameters
   - Invalidation on new imports

2. **Background Jobs** (4-6 hours)
   - Precompute expensive reports nightly
   - Store in database or file cache
   - Serve stale data with "last updated" timestamp

3. **Query Optimization** (2-3 hours)
   - Add composite indexes if needed
   - Analyze slow query log
   - Consider materialized views for large datasets

### Code Quality

4. **Integration Tests** (4-6 hours)
   - API endpoint tests
   - End-to-end scenarios
   - Performance benchmarks

5. **Documentation** (2-3 hours)
   - API reference (OpenAPI/Swagger)
   - Architecture diagrams
   - Contributing guide

6. **CI/CD** (3-4 hours)
   - Automated testing on PR
   - Linting (clippy)
   - Code coverage tracking

---

## Timeline Estimate

### Phase 2 (Next Sprint - 2-3 weeks)
- ✅ Feature 3: Novelty vs Re-listen (8-12 hours)
- ✅ Feature 4: Artist Transitions (6-10 hours)
- ✅ UI for Sessions (4-6 hours)
- ✅ UI for Heatmap (4-6 hours)
- ✅ Integration tests (4-6 hours)
- **Total**: 26-40 hours

### Phase 3 (Future Sprint - 1-2 weeks)
- ✅ Feature 5: Diversity/Entropy (8-10 hours)
- ✅ UI for Novelty (4-6 hours)
- ✅ UI for Transitions (6-8 hours)
- ✅ Server-side caching (2-3 hours)
- **Total**: 20-27 hours

### Phase 4 (Ongoing)
- Additional features from backlog
- Performance optimizations as needed
- External API integrations (genres, audio features)

---

## Success Metrics

### Adoption Metrics
- % of users who access new reports (target: >60%)
- Most popular report (sessions? heatmap? novelty?)
- API endpoint usage

### Performance Metrics
- P95 query response time (target: <1s for 100k scrobbles)
- Cache hit rate (target: >70%)
- Memory usage (target: <100MB per request)

### Quality Metrics
- Test coverage (target: >85%)
- Bug reports for new features
- User feedback/feature requests

---

## Conclusion

**Phase 2** (Novelty + Transitions + UI) is the next logical step, building on the foundation of Phase 1 (Sessions + Heatmap).

**Key Focus Areas**:
1. Complete the top 5 features
2. Build polished UIs for all reports
3. Add caching for performance
4. Comprehensive testing

After Phase 3, the system will have a solid foundation of insightful charts, enabling exploration of more advanced features (genres, audio analysis, social) in Phase 4+.
