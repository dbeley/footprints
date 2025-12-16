use crate::db::DbPool;
use crate::models::Scrobble;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoveltyReport {
    pub timeline: Vec<NoveltyPoint>,
    pub summary: NoveltySummary,
    pub new_artists_discovered: Vec<ArtistDiscovery>,
    pub top_comfort_tracks: Vec<ComfortTrack>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoveltyPoint {
    pub period: String,
    pub total_scrobbles: i64,
    pub new_tracks: i64,
    pub repeat_tracks: i64,
    pub new_artists: i64,
    pub repeat_artists: i64,
    pub novelty_ratio: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoveltySummary {
    pub total_scrobbles: i64,
    pub total_unique_tracks: i64,
    pub total_unique_artists: i64,
    pub avg_novelty_ratio: f64,
    pub most_exploratory_period: String,
    pub least_exploratory_period: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtistDiscovery {
    pub artist: String,
    pub first_heard: DateTime<Utc>,
    pub period: String,
    pub total_plays: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComfortTrack {
    pub artist: String,
    pub track: String,
    pub play_count: i64,
    pub first_heard: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum Granularity {
    Day,
    Week,
    Month,
}

impl Granularity {
    pub fn format_period(&self, dt: &DateTime<Utc>) -> String {
        match self {
            Granularity::Day => dt.format("%Y-%m-%d").to_string(),
            Granularity::Week => dt.format("%Y-W%V").to_string(),
            Granularity::Month => dt.format("%Y-%m").to_string(),
        }
    }
}

pub fn generate_novelty_report(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    granularity: Granularity,
) -> Result<NoveltyReport> {
    // Fetch all scrobbles in range
    let mut scrobbles = if let (Some(s), Some(e)) = (start, end) {
        crate::db::get_scrobbles_in_range(pool, s, e)?
    } else {
        crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?
    };

    // IMPORTANT: Sort by timestamp ascending (oldest first) to track novelty correctly
    // Without a date range, get_scrobbles returns DESC order, so we must sort
    scrobbles.sort_by_key(|s| s.timestamp);

    if scrobbles.is_empty() {
        return Ok(NoveltyReport {
            timeline: Vec::new(),
            summary: NoveltySummary {
                total_scrobbles: 0,
                total_unique_tracks: 0,
                total_unique_artists: 0,
                avg_novelty_ratio: 0.0,
                most_exploratory_period: String::new(),
                least_exploratory_period: String::new(),
            },
            new_artists_discovered: Vec::new(),
            top_comfort_tracks: Vec::new(),
        });
    }

    // Build timeline chronologically, tracking cumulative history
    let mut timeline = Vec::new();
    let mut seen_tracks_ever: HashSet<(String, String)> = HashSet::new();
    let mut seen_artists_ever: HashSet<String> = HashSet::new();
    let mut artist_discoveries: Vec<ArtistDiscovery> = Vec::new();

    // Group scrobbles by period while maintaining order
    let mut period_groups: Vec<(String, Vec<&Scrobble>)> = Vec::new();
    let mut current_period: Option<String> = None;
    let mut current_group: Vec<&Scrobble> = Vec::new();

    for scrobble in &scrobbles {
        let period = granularity.format_period(&scrobble.timestamp);

        if current_period.as_ref() != Some(&period) {
            if let Some(p) = current_period {
                period_groups.push((p, current_group));
                current_group = Vec::new();
            }
            current_period = Some(period);
        }
        current_group.push(scrobble);
    }

    // Don't forget the last group
    if let Some(p) = current_period {
        period_groups.push((p, current_group));
    }

    // Process each period chronologically
    for (period, period_scrobbles) in period_groups {
        let point = compute_novelty_point_cumulative(
            period.clone(),
            &period_scrobbles,
            &mut seen_tracks_ever,
            &mut seen_artists_ever,
            &mut artist_discoveries,
            granularity,
        );
        timeline.push(point);
    }

    // Compute summary
    let summary = compute_novelty_summary(&timeline, &scrobbles);

    // Count total plays for each discovered artist
    let mut artist_play_counts: HashMap<String, i64> = HashMap::new();
    for scrobble in &scrobbles {
        *artist_play_counts
            .entry(scrobble.artist.clone())
            .or_insert(0) += 1;
    }

    for discovery in &mut artist_discoveries {
        discovery.total_plays = artist_play_counts
            .get(&discovery.artist)
            .copied()
            .unwrap_or(0);
    }

    // Find top comfort tracks
    let top_comfort_tracks = find_top_comfort_tracks(&scrobbles, 10);

    Ok(NoveltyReport {
        timeline,
        summary,
        new_artists_discovered: artist_discoveries,
        top_comfort_tracks,
    })
}

fn compute_novelty_point_cumulative(
    period: String,
    scrobbles: &[&Scrobble],
    seen_tracks_ever: &mut HashSet<(String, String)>,
    seen_artists_ever: &mut HashSet<String>,
    artist_discoveries: &mut Vec<ArtistDiscovery>,
    _granularity: Granularity,
) -> NoveltyPoint {
    let total_scrobbles = scrobbles.len() as i64;

    let mut new_tracks = 0;
    let mut new_artists = 0;

    for scrobble in scrobbles {
        let track_key = (scrobble.artist.clone(), scrobble.track.clone());

        // Check if this is the first time seeing this track EVER
        if !seen_tracks_ever.contains(&track_key) {
            new_tracks += 1;
            seen_tracks_ever.insert(track_key);
        }

        // Check if this is the first time seeing this artist EVER
        if !seen_artists_ever.contains(&scrobble.artist) {
            new_artists += 1;
            seen_artists_ever.insert(scrobble.artist.clone());

            // Record this discovery
            artist_discoveries.push(ArtistDiscovery {
                artist: scrobble.artist.clone(),
                first_heard: scrobble.timestamp,
                period: period.clone(),
                total_plays: 0, // Will be counted later
            });
        }
    }

    let repeat_tracks = total_scrobbles - new_tracks;
    let repeat_artists = scrobbles
        .iter()
        .map(|s| s.artist.as_str())
        .collect::<HashSet<_>>()
        .len() as i64
        - new_artists;

    let novelty_ratio = if total_scrobbles > 0 {
        new_tracks as f64 / total_scrobbles as f64
    } else {
        0.0
    };

    NoveltyPoint {
        period,
        total_scrobbles,
        new_tracks,
        repeat_tracks,
        new_artists,
        repeat_artists,
        novelty_ratio,
    }
}

fn compute_novelty_summary(timeline: &[NoveltyPoint], scrobbles: &[Scrobble]) -> NoveltySummary {
    let total_scrobbles = scrobbles.len() as i64;

    let unique_tracks: HashSet<_> = scrobbles
        .iter()
        .map(|s| (s.artist.clone(), s.track.clone()))
        .collect();

    let unique_artists: HashSet<_> = scrobbles.iter().map(|s| s.artist.clone()).collect();

    let avg_novelty_ratio = if !timeline.is_empty() {
        timeline.iter().map(|p| p.novelty_ratio).sum::<f64>() / timeline.len() as f64
    } else {
        0.0
    };

    let most_exploratory = timeline
        .iter()
        .max_by(|a, b| a.novelty_ratio.partial_cmp(&b.novelty_ratio).unwrap())
        .map(|p| p.period.clone())
        .unwrap_or_default();

    let least_exploratory = timeline
        .iter()
        .min_by(|a, b| a.novelty_ratio.partial_cmp(&b.novelty_ratio).unwrap())
        .map(|p| p.period.clone())
        .unwrap_or_default();

    NoveltySummary {
        total_scrobbles,
        total_unique_tracks: unique_tracks.len() as i64,
        total_unique_artists: unique_artists.len() as i64,
        avg_novelty_ratio,
        most_exploratory_period: most_exploratory,
        least_exploratory_period: least_exploratory,
    }
}

fn find_top_comfort_tracks(scrobbles: &[Scrobble], limit: usize) -> Vec<ComfortTrack> {
    let mut track_counts: HashMap<(String, String), (i64, DateTime<Utc>)> = HashMap::new();

    for scrobble in scrobbles {
        let track_key = (scrobble.artist.clone(), scrobble.track.clone());
        track_counts
            .entry(track_key)
            .and_modify(|(count, first)| {
                *count += 1;
                if scrobble.timestamp < *first {
                    *first = scrobble.timestamp;
                }
            })
            .or_insert((1, scrobble.timestamp));
    }

    let mut comfort_tracks: Vec<_> = track_counts
        .into_iter()
        .map(|((artist, track), (count, first_heard))| ComfortTrack {
            artist,
            track,
            play_count: count,
            first_heard,
        })
        .collect();

    comfort_tracks.sort_by(|a, b| b.play_count.cmp(&a.play_count));
    comfort_tracks.truncate(limit);

    comfort_tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scrobble(timestamp: &str, artist: &str, track: &str) -> Scrobble {
        Scrobble {
            id: Some(0),
            artist: artist.to_string(),
            album: Some("Test Album".to_string()),
            track: track.to_string(),
            timestamp: timestamp.parse().unwrap(),
            source: "test".to_string(),
            source_id: None,
        }
    }

    #[test]
    fn test_novelty_point_all_new() {
        let scrobbles = [
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T10:05:00Z", "Artist B", "Track 2"),
            test_scrobble("2024-01-01T10:10:00Z", "Artist C", "Track 3"),
        ];

        let scrobble_refs: Vec<_> = scrobbles.iter().collect();
        let mut seen_tracks = HashSet::new();
        let mut seen_artists = HashSet::new();
        let mut discoveries = Vec::new();

        let point = compute_novelty_point_cumulative(
            "2024-01-01".to_string(),
            &scrobble_refs,
            &mut seen_tracks,
            &mut seen_artists,
            &mut discoveries,
            Granularity::Day,
        );

        assert_eq!(point.total_scrobbles, 3);
        assert_eq!(point.new_tracks, 3);
        assert_eq!(point.repeat_tracks, 0);
        assert_eq!(point.new_artists, 3);
        assert!((point.novelty_ratio - 1.0).abs() < 0.001);
        assert_eq!(discoveries.len(), 3);
    }

    #[test]
    fn test_novelty_point_all_repeats() {
        let scrobbles = [
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-02T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-03T10:00:00Z", "Artist A", "Track 1"),
        ];

        // First period - everything is new
        let period1: Vec<_> = scrobbles[0..1].iter().collect();
        let mut seen_tracks = HashSet::new();
        let mut seen_artists = HashSet::new();
        let mut discoveries = Vec::new();

        let point1 = compute_novelty_point_cumulative(
            "2024-01-01".to_string(),
            &period1,
            &mut seen_tracks,
            &mut seen_artists,
            &mut discoveries,
            Granularity::Day,
        );

        assert_eq!(point1.new_tracks, 1);
        assert_eq!(point1.novelty_ratio, 1.0);

        // Second period - all repeats
        let period2: Vec<_> = scrobbles[1..2].iter().collect();
        let point2 = compute_novelty_point_cumulative(
            "2024-01-02".to_string(),
            &period2,
            &mut seen_tracks,
            &mut seen_artists,
            &mut discoveries,
            Granularity::Day,
        );

        assert_eq!(point2.new_tracks, 0);
        assert!((point2.novelty_ratio - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cumulative_tracking() {
        let scrobbles = [
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T11:00:00Z", "Artist A", "Track 2"),
            // Week 2: 1 new, 1 repeat
            test_scrobble("2024-01-08T10:00:00Z", "Artist A", "Track 1"), // Repeat
            test_scrobble("2024-01-08T11:00:00Z", "Artist B", "Track 3"),
        ];

        let mut seen_tracks = HashSet::new();
        let mut seen_artists = HashSet::new();
        let mut discoveries = Vec::new();

        // Week 1
        let week1: Vec<_> = scrobbles[0..2].iter().collect();
        let point1 = compute_novelty_point_cumulative(
            "2024-W01".to_string(),
            &week1,
            &mut seen_tracks,
            &mut seen_artists,
            &mut discoveries,
            Granularity::Week,
        );

        assert_eq!(point1.new_tracks, 2);
        assert_eq!(point1.new_artists, 1);
        assert_eq!(point1.repeat_tracks, 0);
        assert!((point1.novelty_ratio - 1.0).abs() < 0.001);

        // Week 2
        let week2: Vec<_> = scrobbles[2..4].iter().collect();
        let point2 = compute_novelty_point_cumulative(
            "2024-W02".to_string(),
            &week2,
            &mut seen_tracks,
            &mut seen_artists,
            &mut discoveries,
            Granularity::Week,
        );

        assert_eq!(point2.new_tracks, 1); // Only Track 3 is new
        assert_eq!(point2.new_artists, 1); // Artist B is new
        assert_eq!(point2.repeat_tracks, 1); // Track 1 is repeat
        assert!((point2.novelty_ratio - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_top_comfort_tracks() {
        let scrobbles = vec![
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T10:05:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T10:10:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T10:15:00Z", "Artist B", "Track 2"),
            test_scrobble("2024-01-01T10:20:00Z", "Artist B", "Track 2"),
        ];

        let comfort = find_top_comfort_tracks(&scrobbles, 10);

        assert_eq!(comfort.len(), 2);
        assert_eq!(comfort[0].artist, "Artist A");
        assert_eq!(comfort[0].track, "Track 1");
        assert_eq!(comfort[0].play_count, 3);
    }

    #[test]
    fn test_novelty_chronological_order() {
        // Integration test: verify novelty decreases over time as expected
        use crate::db::{create_pool, init_database};
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        let pool = create_pool(db_path).unwrap();
        init_database(&pool).unwrap();

        // Insert scrobbles in a realistic pattern:
        // Month 1: All new tracks
        // Month 2: Mix of new and repeats
        // Month 3: Mostly repeats
        let test_data = vec![
            // January 2024: 3 unique tracks (should be 100% novel)
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-05T10:00:00Z", "Artist B", "Track 2"),
            test_scrobble("2024-01-10T10:00:00Z", "Artist C", "Track 3"),
            // February 2024: 1 new + 2 repeats (should be 33% novel)
            test_scrobble("2024-02-01T10:00:00Z", "Artist A", "Track 1"), // repeat
            test_scrobble("2024-02-05T10:00:00Z", "Artist B", "Track 2"), // repeat
            test_scrobble("2024-02-10T10:00:00Z", "Artist D", "Track 4"), // new
            // March 2024: 0 new + 3 repeats (should be 0% novel)
            test_scrobble("2024-03-01T10:00:00Z", "Artist A", "Track 1"), // repeat
            test_scrobble("2024-03-05T10:00:00Z", "Artist B", "Track 2"), // repeat
            test_scrobble("2024-03-10T10:00:00Z", "Artist C", "Track 3"), // repeat
        ];

        for scrobble in &test_data {
            crate::db::insert_scrobble(&pool, scrobble).unwrap();
        }

        // Generate report
        let report = generate_novelty_report(&pool, None, None, Granularity::Month).unwrap();

        // Verify we have 3 periods
        assert_eq!(report.timeline.len(), 3, "Should have 3 monthly periods");

        // Verify chronological ordering and novelty progression
        assert_eq!(
            report.timeline[0].period, "2024-01",
            "First period should be January"
        );
        assert_eq!(
            report.timeline[1].period, "2024-02",
            "Second period should be February"
        );
        assert_eq!(
            report.timeline[2].period, "2024-03",
            "Third period should be March"
        );

        // Verify novelty ratios decrease over time (early = high, later = low)
        let jan_novelty = report.timeline[0].novelty_ratio;
        let feb_novelty = report.timeline[1].novelty_ratio;
        let mar_novelty = report.timeline[2].novelty_ratio;

        assert!(
            (jan_novelty - 1.0).abs() < 0.001,
            "January (first period) should be 100% novel, got {}",
            jan_novelty
        );
        assert!(
            (feb_novelty - 0.333).abs() < 0.01,
            "February should be ~33% novel (1/3), got {}",
            feb_novelty
        );
        assert!(
            (mar_novelty - 0.0).abs() < 0.001,
            "March should be 0% novel (all repeats), got {}",
            mar_novelty
        );

        // Verify novelty is strictly decreasing or equal
        assert!(
            jan_novelty >= feb_novelty,
            "Novelty should decrease or stay same from Jan to Feb: {} >= {}",
            jan_novelty,
            feb_novelty
        );
        assert!(
            feb_novelty >= mar_novelty,
            "Novelty should decrease or stay same from Feb to Mar: {} >= {}",
            feb_novelty,
            mar_novelty
        );
    }
}
