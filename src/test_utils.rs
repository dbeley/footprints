// Test utilities for creating mock scrobble data
use crate::models::Scrobble;
use chrono::{DateTime, Duration, TimeZone, Utc};

/// Create a test scrobble with specified parameters
pub fn test_scrobble(
    artist: &str,
    track: &str,
    timestamp: DateTime<Utc>,
    source: &str,
) -> Scrobble {
    Scrobble {
        id: None,
        artist: artist.to_string(),
        album: Some("Test Album".to_string()),
        track: track.to_string(),
        timestamp,
        source: source.to_string(),
        source_id: None,
    }
}

/// Create a test scrobble from RFC3339 timestamp string
pub fn test_scrobble_from_rfc3339(artist: &str, track: &str, timestamp_str: &str) -> Scrobble {
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .unwrap()
        .with_timezone(&Utc);
    test_scrobble(artist, track, timestamp, "test")
}

/// Generate a sequence of scrobbles for testing
pub fn generate_scrobbles_sequence(
    count: usize,
    start_time: DateTime<Utc>,
    interval_minutes: i64,
) -> Vec<Scrobble> {
    (0..count)
        .map(|i| {
            let timestamp = start_time + Duration::minutes(interval_minutes * i as i64);
            test_scrobble(
                &format!("Artist {}", i % 5),
                &format!("Track {}", i),
                timestamp,
                "test",
            )
        })
        .collect()
}

/// Generate scrobbles with specific artist/track distribution for testing diversity
pub fn generate_diverse_scrobbles(
    artists: &[(&str, usize)], // (artist_name, play_count)
    start_time: DateTime<Utc>,
) -> Vec<Scrobble> {
    let mut scrobbles = Vec::new();
    let mut offset_minutes = 0;

    for (artist, count) in artists {
        for i in 0..*count {
            let timestamp = start_time + Duration::minutes(offset_minutes);
            scrobbles.push(test_scrobble(
                artist,
                &format!("{} Track {}", artist, i),
                timestamp,
                "test",
            ));
            offset_minutes += 5;
        }
    }

    scrobbles
}

/// Generate scrobbles with repeated tracks for testing novelty
pub fn generate_repeated_scrobbles(
    tracks: &[(&str, &str, usize)], // (artist, track, repeat_count)
    start_time: DateTime<Utc>,
) -> Vec<Scrobble> {
    let mut scrobbles = Vec::new();
    let mut offset_minutes = 0;

    for (artist, track, count) in tracks {
        for _ in 0..*count {
            let timestamp = start_time + Duration::minutes(offset_minutes);
            scrobbles.push(test_scrobble(artist, track, timestamp, "test"));
            offset_minutes += 5;
        }
    }

    scrobbles
}

/// Generate listening session with realistic gaps
pub fn generate_listening_session(
    artist_track_pairs: &[(&str, &str)],
    start_time: DateTime<Utc>,
    track_duration_minutes: i64,
) -> Vec<Scrobble> {
    artist_track_pairs
        .iter()
        .enumerate()
        .map(|(i, (artist, track))| {
            let timestamp = start_time + Duration::minutes(track_duration_minutes * i as i64);
            test_scrobble(artist, track, timestamp, "test")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrobble_creation() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let scrobble = test_scrobble("Artist", "Track", timestamp, "test");

        assert_eq!(scrobble.artist, "Artist");
        assert_eq!(scrobble.track, "Track");
        assert_eq!(scrobble.timestamp, timestamp);
        assert_eq!(scrobble.source, "test");
    }

    #[test]
    fn test_scrobble_from_rfc3339_helper() {
        let scrobble = test_scrobble_from_rfc3339("Artist", "Track", "2024-01-01T12:00:00Z");

        assert_eq!(scrobble.artist, "Artist");
        assert_eq!(scrobble.track, "Track");
    }

    #[test]
    fn test_generate_scrobbles_sequence() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let scrobbles = generate_scrobbles_sequence(10, start, 5);

        assert_eq!(scrobbles.len(), 10);
        assert_eq!(scrobbles[0].timestamp, start);
        assert_eq!(scrobbles[9].timestamp, start + Duration::minutes(45));
    }

    #[test]
    fn test_generate_diverse_scrobbles() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let artists = vec![("Artist A", 3), ("Artist B", 2)];
        let scrobbles = generate_diverse_scrobbles(&artists, start);

        assert_eq!(scrobbles.len(), 5);

        let artist_a_count = scrobbles.iter().filter(|s| s.artist == "Artist A").count();
        let artist_b_count = scrobbles.iter().filter(|s| s.artist == "Artist B").count();

        assert_eq!(artist_a_count, 3);
        assert_eq!(artist_b_count, 2);
    }

    #[test]
    fn test_generate_repeated_scrobbles() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let tracks = vec![("Artist", "Track A", 3), ("Artist", "Track B", 2)];
        let scrobbles = generate_repeated_scrobbles(&tracks, start);

        assert_eq!(scrobbles.len(), 5);

        let track_a_count = scrobbles.iter().filter(|s| s.track == "Track A").count();
        let track_b_count = scrobbles.iter().filter(|s| s.track == "Track B").count();

        assert_eq!(track_a_count, 3);
        assert_eq!(track_b_count, 2);
    }

    #[test]
    fn test_generate_listening_session() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let tracks = vec![("Artist A", "Track 1"), ("Artist B", "Track 2")];
        let scrobbles = generate_listening_session(&tracks, start, 4);

        assert_eq!(scrobbles.len(), 2);
        assert_eq!(scrobbles[0].timestamp, start);
        assert_eq!(scrobbles[1].timestamp, start + Duration::minutes(4));
    }
}
