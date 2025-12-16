# Feature Backlog - Footprints Chart/Visualization/Report Ideas

## Guiding Principles
1. Insightful over "top N lists" 
2. Reproducible & self-hosted (no SaaS dependencies)
3. Clear title, explanation, filters, and "how computed" notes
4. Test coverage for core aggregation logic

## Feature Categories

---

## Category 1: Behavior Over Time (Trends, Seasonality, Streaks)

### 1.1 Listening "Sessions" Report ⭐ **TOP 5**
**What:** Detect and analyze listening sessions using inactivity gaps.

**Question Answered:** How do I listen to music? Long focused sessions or short bursts throughout the day?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** M (session detection algorithm, SQL window functions or post-processing)

**Performance Risk:** Medium (requires ordered full scan to detect gaps)

**Suggested UX:**
- **Page:** `/reports/sessions` 
- **Filters:** Time range, gap threshold (30/45/60 min), source
- **Charts:** 
  - Session length distribution (histogram)
  - Sessions per day/week (bar chart)
  - Average tracks per session (line chart over time)
  - Longest sessions (top 10 with track list)
- **Drilldown:** Click session → see full track sequence with timestamps

**How Computed:**
1. Order scrobbles by timestamp
2. Compute gap between consecutive scrobbles
3. Start new session when gap > threshold (default: 45 min)
4. Aggregate session metrics (duration, track count, start/end times)

**Edge Cases:**
- Single-track sessions (valid but not interesting)
- Sessions spanning midnight (handle date boundaries)
- Mixed sources within session (allow or separate?)

---

### 1.2 Listening Streaks
**What:** Track consecutive days/weeks with at least N scrobbles.

**Question Answered:** How consistent is my music listening? Am I on a hot streak?

**Required Fields:** `timestamp`

**Complexity:** S (simple date grouping + streak detection)

**Performance Risk:** Low (requires per-day count, then streak computation)

**Suggested UX:**
- **Page:** Dashboard widget + `/reports/streaks`
- **Filters:** Minimum scrobbles per day (default: 5)
- **Charts:**
  - Current streak (big number)
  - Longest streak ever (big number)
  - Streak calendar heatmap (GitHub-style)
- **Drilldown:** Click day → see that day's scrobbles

**How Computed:**
1. Group scrobbles by date
2. Filter days with count >= threshold
3. Detect consecutive date sequences
4. Track current and historical max

---

### 1.3 Library Growth Timeline
**What:** Track cumulative unique artists/albums/tracks over time.

**Question Answered:** How has my music library grown? When did I discover the most new music?

**Required Fields:** `timestamp`, `artist`, `album`, `track`

**Complexity:** M (requires window functions or iterative processing)

**Performance Risk:** Low (one-time computation, cacheable)

**Suggested UX:**
- **Page:** `/reports/library-growth`
- **Charts:**
  - Cumulative unique artists (line chart)
  - New artists per month (bar chart)
  - Cumulative tracks over time (line chart)
  - Growth rate (derivative of cumulative)

**How Computed:**
1. Order scrobbles by timestamp
2. Use window function to mark "first occurrence" of each artist/track/album
3. Aggregate by month/year
4. Compute cumulative sum

---

### 1.4 Seasonal Listening Patterns
**What:** Compare listening habits by season, month, day-of-week.

**Question Answered:** Do I listen to more music in winter? Am I a weekend warrior?

**Required Fields:** `timestamp`, `artist`

**Complexity:** S (date extraction + grouping)

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/seasonal`
- **Charts:**
  - Scrobbles by month (all years combined, bar chart)
  - Scrobbles by weekday (bar chart)
  - Scrobbles by season (pie chart)
  - Average daily scrobbles by month

**How Computed:**
1. Extract month, weekday, season from timestamp
2. Group and count
3. Normalize by number of days in period (for fair comparison)

---

### 1.5 "Comeback" Artists
**What:** Artists you stopped listening to, then rediscovered after 6+ months.

**Question Answered:** Which artists did I revisit after a long break?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M (requires gap detection per artist)

**Performance Risk:** Medium (per-artist gap computation)

**Suggested UX:**
- **Page:** `/reports/comebacks`
- **Filters:** Minimum gap (default: 6 months)
- **Charts:** List of artists with gap duration and comeback date

**How Computed:**
1. Group scrobbles by artist
2. Order each artist's scrobbles by timestamp
3. Detect gaps > threshold
4. List artists with gaps + comeback dates

---

## Category 2: Discovery & Exploration (Rabbit Holes, Similarity, Transitions)

### 2.1 Novelty vs. Re-listen Ratio ⭐ **TOP 5**
**What:** Track percentage of listens that are first-time vs. repeats, by day/week/month.

**Question Answered:** Am I exploring new music or comfort-listening to favorites?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** M (requires window functions to mark first occurrence)

**Performance Risk:** Medium (requires ordered processing)

**Suggested UX:**
- **Page:** `/reports/novelty`
- **Filters:** Time range, granularity (day/week/month), exclude artists
- **Charts:**
  - % new tracks per week (line chart)
  - % new artists per week (line chart)
  - "Discovery timeline" (scatter plot: new artists over time)
  - Top "comfort food" tracks (most re-listened)
- **Drilldown:** Click week → see new artists discovered that week

**How Computed:**
1. Order scrobbles by timestamp
2. Mark first occurrence of each track/artist (globally)
3. Group by period (day/week/month)
4. Count new vs. repeat listens
5. Compute percentage

**Edge Cases:**
- First-ever scrobbles (all "new")
- Renamed artists (treat as new)
- Same track, different album (count as same or different?)

---

### 2.2 Artist-to-Artist Transitions (Flow) ⭐ **TOP 5**
**What:** Most common artist transitions within listening sessions.

**Question Answered:** What artists do I pair together? What's my listening flow?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M (requires session detection + transition counting)

**Performance Risk:** Medium (depends on session detection)

**Suggested UX:**
- **Page:** `/reports/transitions`
- **Filters:** Time range, minimum transition count, genre filter (if tags available)
- **Charts:**
  - Top 20 transitions (table: Artist A → Artist B, count)
  - Sankey diagram data (JSON for visualization)
  - Network graph data (nodes: artists, edges: transitions)
- **Drilldown:** Click transition → see example sessions with that transition

**How Computed:**
1. Detect sessions (using gap threshold)
2. Within each session, extract artist transitions (A→B→C→...)
3. Count each unique transition (A→B)
4. Rank by frequency

**Edge Cases:**
- Single-artist sessions (no transitions)
- Self-transitions (A→A, could filter or include)

---

### 2.3 "Rabbit Hole" Sessions
**What:** Sessions where you deep-dived into a single artist (>80% same artist).

**Question Answered:** When did I binge on a specific artist?

**Required Fields:** `timestamp`, `artist`

**Complexity:** S (session detection + per-session artist dominance)

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/rabbit-holes`
- **Filters:** Time range, minimum session length
- **Charts:** List of rabbit hole sessions (artist, date, track count, duration)

**How Computed:**
1. Detect sessions
2. For each session, compute artist distribution
3. Flag sessions where one artist >80% of tracks
4. Sort by dominance or duration

---

### 2.4 Exploration Score
**What:** Monthly metric combining unique artists, tracks, and genre diversity.

**Question Answered:** Am I in an exploratory phase or a repetitive phase?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** M (requires defining a composite metric)

**Performance Risk:** Low

**Suggested UX:**
- **Page:** Dashboard widget + `/reports/exploration`
- **Charts:**
  - Exploration score over time (line chart)
  - Components breakdown (stacked bar: unique artists, tracks, diversity)

**How Computed:**
1. Per month, compute:
   - Unique artists count
   - Unique tracks count
   - Shannon entropy of artist distribution
2. Normalize each component (z-score or percentile)
3. Combine into single score (weighted average)

---

### 2.5 Genre Journey (Requires Tags)
**What:** How your genre listening has evolved over time.

**Question Answered:** What genre phases have I gone through?

**Required Fields:** `timestamp`, `artist`, tags/genres (NOT AVAILABLE)

**Complexity:** L (requires fetching genre tags from external API, e.g., Last.fm)

**Performance Risk:** High (external API calls)

**Suggested UX:**
- **Page:** `/reports/genre-journey`
- **Charts:**
  - Genre stacked area chart over time
  - Genre percentage pie chart by year

**How Computed:**
1. Fetch artist tags from Last.fm API
2. Map scrobbles to genres
3. Group by time period
4. Compute genre distribution

**Note:** Requires implementation of genre tagging system.

---

## Category 3: Habits & Routines (Time-of-Day, Weekday, Session Analysis)

### 3.1 Time-of-Day / Week Heatmap ⭐ **TOP 5**
**What:** Heatmap showing when you listen (hour x weekday).

**Question Answered:** When am I most likely to listen? Do I have listening routines?

**Required Fields:** `timestamp`

**Complexity:** M (requires timezone handling, hour/weekday extraction)

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/heatmap`
- **Filters:** Time range, timezone, compare periods
- **Charts:**
  - Heatmap (hour x weekday, color intensity = scrobble count)
  - "Most characteristic hours" (when you listen most vs. average)
  - Period comparison (current 30 days vs. previous 30 days)

**How Computed:**
1. Extract hour and weekday from timestamp (in user's timezone)
2. Group and count
3. Normalize by number of weeks in range
4. Generate heatmap matrix (7 days x 24 hours)

**Edge Cases:**
- Timezone changes (DST)
- User timezone configuration (currently not stored)
- Missing data (no listens in some hours)

**Implementation Note:** Requires adding user timezone setting.

---

### 3.2 Late Night Listening
**What:** Tracks/artists most listened to after midnight.

**Question Answered:** What's my 2 AM music?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** S

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/late-night`
- **Filters:** Time range, timezone, hour threshold (default: 00:00-05:00)
- **Charts:**
  - Top late-night artists
  - Top late-night tracks
  - Late-night vs. daytime listening ratio

**How Computed:**
1. Filter scrobbles where hour(timestamp) ∈ [0, 5)
2. Aggregate top artists/tracks
3. Compare with overall top artists (uniqueness score)

---

### 3.3 Weekend vs. Weekday Patterns
**What:** Compare listening habits on weekdays vs. weekends.

**Question Answered:** Do I listen differently on weekends?

**Required Fields:** `timestamp`, `artist`

**Complexity:** S

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/weekend-weekday`
- **Charts:**
  - Average scrobbles weekday vs. weekend
  - Top artists weekday vs. weekend (side-by-side)
  - Genre distribution (if tags available)

**How Computed:**
1. Classify each scrobble as weekday (Mon-Fri) or weekend (Sat-Sun)
2. Aggregate separately
3. Compare distributions

---

### 3.4 Productivity Soundtrack
**What:** Identify "focus mode" sessions (long, consistent, instrumental-heavy).

**Question Answered:** What music do I use for focused work?

**Required Fields:** `timestamp`, `artist`, `track`, genres/tags

**Complexity:** L (requires genre data, session detection)

**Performance Risk:** Medium

**Suggested UX:**
- **Page:** `/reports/productivity`
- **Charts:**
  - Top "focus" artists
  - Typical focus session characteristics (length, time of day)

**How Computed:**
1. Detect long sessions (>60 min)
2. Filter by time-of-day (9am-5pm weekdays)
3. Score sessions by genre (instrumental, ambient, classical = higher)
4. List top artists from these sessions

---

### 3.5 Commute Music
**What:** Music listened during typical commute hours (7-9am, 5-7pm).

**Question Answered:** What's my commute soundtrack?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** S

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/commute`
- **Filters:** Timezone, custom commute hours
- **Charts:**
  - Top commute artists
  - Commute listening trends over time

**How Computed:**
1. Filter scrobbles in commute hours
2. Aggregate artists/tracks
3. Compare with overall listening

---

## Category 4: Collection Intelligence (Library Growth, Re-listens, Burnout)

### 4.1 Artist "Lifecycle" Analysis
**What:** Track progression from discovery → obsession → burnout → rediscovery.

**Question Answered:** What's my relationship history with each artist?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M

**Performance Risk:** Medium (per-artist analysis)

**Suggested UX:**
- **Page:** `/reports/artist-lifecycle`
- **Filters:** Select artist, time range
- **Charts:**
  - Artist listening over time (line chart)
  - Phases: discovery, peak, decline, comeback
  - Comparison with similar artists

**How Computed:**
1. Per artist, get all scrobbles over time
2. Compute rolling average (smooth curve)
3. Detect phases: peak (max), decline (negative derivative), comeback (local min → increase)

---

### 4.2 "Burnout" Detection
**What:** Artists you over-played and then stopped listening to.

**Question Answered:** What artists did I burn out on?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M

**Performance Risk:** Medium

**Suggested UX:**
- **Page:** `/reports/burnout`
- **Charts:**
  - List of "burnt out" artists (high peak, then long absence)
  - Burnout risk for current favorites (predictive)

**How Computed:**
1. Identify artists with high peak (>50 listens/month)
2. Detect >6 month absence after peak
3. Rank by peak height and absence duration

---

### 4.3 "One-Hit Wonders" (Personal)
**What:** Artists you listened to once and never again.

**Question Answered:** What music didn't stick with me?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** S

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/one-hit-wonders`
- **Charts:**
  - List of artists with exactly 1 scrobble
  - List of artists with <5 scrobbles (threshold configurable)

**How Computed:**
1. Group by artist, count scrobbles
2. Filter where count = 1 (or < threshold)
3. Sort by recency

---

### 4.4 Album Completion Rate
**What:** For albums you've started, how many did you finish?

**Question Answered:** Do I listen to full albums or just singles?

**Required Fields:** `timestamp`, `artist`, `album`, `track`

**Complexity:** L (requires fetching album track counts from external API)

**Performance Risk:** High (external API calls)

**Suggested UX:**
- **Page:** `/reports/album-completion`
- **Charts:**
  - Albums with completion % (track count / total tracks)
  - Most completed albums
  - Average completion rate over time

**How Computed:**
1. Fetch album track counts from MusicBrainz/Spotify
2. Count unique tracks listened per album
3. Compute completion %

**Note:** Requires album metadata enrichment.

---

### 4.5 Catalog Deep Cuts
**What:** Identify lesser-known tracks from favorite artists.

**Question Answered:** Am I exploring artist catalogs or just hits?

**Required Fields:** `artist`, `track`, popularity data

**Complexity:** L (requires external popularity data)

**Performance Risk:** High

**Suggested UX:**
- **Page:** `/reports/deep-cuts`
- **Charts:**
  - Deep cuts listened (low popularity, high personal listens)
  - Mainstream vs. deep-cut ratio by artist

**How Computed:**
1. Fetch track popularity from Spotify/Last.fm
2. Score tracks: low popularity + high personal listens = deep cut
3. Rank by score

---

## Category 5: Listening Quality Signals (Repeat Ratio, Diversity, Entropy)

### 5.1 Diversity / Entropy Trend ⭐ **TOP 5**
**What:** Rolling diversity metrics: unique artists per N listens, Shannon entropy per period.

**Question Answered:** Is my listening diverse or concentrated? Am I broadening or narrowing?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M (requires entropy calculation, rolling windows)

**Performance Risk:** Low-Medium

**Suggested UX:**
- **Page:** `/reports/diversity`
- **Filters:** Time range, window size (default: weekly)
- **Charts:**
  - Unique artists per 100 scrobbles (line chart)
  - Shannon entropy per week (line chart)
  - Gini coefficient (artist concentration) over time
  - Diversity score explanation (tooltip)

**How Computed:**
1. **Unique Artists Metric:**
   - Rolling window of N scrobbles
   - Count unique artists in window
   - Plot over time

2. **Shannon Entropy:**
   - Per week, compute artist distribution
   - H = -Σ(p_i * log(p_i)) where p_i = proportion of artist i
   - Higher H = more diverse

3. **Gini Coefficient:**
   - Per week, compute artist scrobble distribution
   - Gini = measure of inequality (0=perfect equality, 1=one artist dominates)

**Explanation in UI:**
- "Your diversity score is based on how evenly you distribute listens across artists. Higher is more diverse."

---

### 5.2 Repeat Ratio
**What:** Percentage of scrobbles that are re-listens (track played >1 time).

**Question Answered:** Do I discover new tracks or replay favorites?

**Required Fields:** `timestamp`, `artist`, `track`

**Complexity:** M

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/repeat-ratio`
- **Charts:**
  - Repeat ratio over time (% of replays)
  - Most repeated tracks (top 20)
  - Distribution of track play counts (histogram)

**How Computed:**
1. Count total plays per track
2. Count tracks with plays > 1
3. Compute % = (repeat tracks / total tracks) * 100

---

### 5.3 Listening Consistency Score
**What:** Measure regularity of listening (std dev of daily scrobbles).

**Question Answered:** Do I listen consistently or in bursts?

**Required Fields:** `timestamp`

**Complexity:** S

**Performance Risk:** Low

**Suggested UX:**
- **Page:** Dashboard widget
- **Charts:**
  - Consistency score (single number: 0-100)
  - Daily scrobbles histogram
  - Variance over time

**How Computed:**
1. Compute daily scrobble counts
2. Calculate standard deviation
3. Normalize to 0-100 scale (lower variance = higher consistency)

---

### 5.4 "Taste Drift" Analysis
**What:** Measure how much your taste has changed over time.

**Question Answered:** Is my music taste evolving or stable?

**Required Fields:** `timestamp`, `artist`

**Complexity:** M

**Performance Risk:** Low

**Suggested UX:**
- **Page:** `/reports/taste-drift`
- **Charts:**
  - Taste drift score over time
  - Top artists by period (compare across years)
  - Artist overlap between periods (Jaccard similarity)

**How Computed:**
1. Divide history into periods (e.g., yearly)
2. Compute top N artists for each period
3. Calculate Jaccard similarity between consecutive periods
4. Lower similarity = higher drift

---

### 5.5 Obscurity Score
**What:** Measure how "obscure" your taste is (based on global Last.fm popularity).

**Question Answered:** Am I a hipster or a mainstream listener?

**Required Fields:** `artist`, global popularity data

**Complexity:** L (requires external API for popularity)

**Performance Risk:** High

**Suggested UX:**
- **Page:** Dashboard widget + `/reports/obscurity`
- **Charts:**
  - Obscurity score (0-100)
  - Most obscure artists you listen to
  - Obscurity over time

**How Computed:**
1. Fetch global playcount for each artist (Last.fm)
2. Score = inverse of popularity (lower playcount = higher obscurity)
3. Weight by your listen count
4. Aggregate to single score

---

## Top 5 Features Selected for Implementation

### Priority Ranking (Value/Effort)

| Rank | Feature | Value | Effort | Ratio | Why Selected |
|------|---------|-------|--------|-------|--------------|
| 1 | **Listening Sessions** | High | Medium | 1.5 | Foundation for many other features, immediately useful |
| 2 | **Time-of-Day Heatmap** | High | Medium | 1.5 | Visual, insightful, unique value vs. existing reports |
| 3 | **Novelty vs. Re-listen** | High | Medium | 1.5 | Unique insight, quantifies exploration behavior |
| 4 | **Artist Transitions** | Medium | Medium | 1.0 | Interesting, builds on sessions, enables network viz |
| 5 | **Diversity/Entropy Trend** | High | Medium | 1.5 | Quantitative, actionable, complements novelty |

### Why These Five?

1. **Sessions**: Core building block for time-based analysis. Enables:
   - Transition analysis
   - Focus/productivity tracking
   - Binge detection

2. **Time-of-Day Heatmap**: 
   - Visual and immediately understandable
   - Reveals routine patterns
   - Differentiates from basic "top N" reports
   - Requires timezone handling (good learning)

3. **Novelty vs. Re-listen**:
   - Unique metric not found in most tools
   - Actionable (tells you if you're exploring or stagnating)
   - Builds foundation for discovery tracking

4. **Artist Transitions**:
   - Leverages session detection
   - Enables network/Sankey visualizations
   - Reveals listening flow and pairing preferences

5. **Diversity/Entropy**:
   - Quantitative measure of taste breadth
   - Complements novelty tracking
   - Actionable (encourages exploration)

### Features Deferred (But in Backlog)

**Low-hanging fruit for later:**
- Streaks (simple, motivational)
- Late-night listening (niche but fun)
- Weekend vs. weekday (easy comparison)

**Requires external data:**
- Genre journey (needs Last.fm tags)
- Album completion (needs track counts)
- Obscurity score (needs global popularity)

**Complex but valuable:**
- Artist lifecycle (requires per-artist time-series analysis)
- Burnout detection (predictive modeling)
- Taste drift (similarity computation)

## Implementation Order

**Phase 1 (This PR):**
1. **Listening Sessions** - Build session detection, add database helpers
2. **Time-of-Day Heatmap** - Add timezone config, implement heatmap aggregation

**Phase 2 (Next):**
3. **Novelty vs. Re-listen** - Window functions for first-occurrence marking
4. **Artist Transitions** - Build on sessions, add transition counting

**Phase 3 (Future):**
5. **Diversity/Entropy** - Rolling window metrics, entropy calculation
