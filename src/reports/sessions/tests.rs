use super::*;
use chrono::TimeZone;

fn test_scrobble(timestamp_str: &str, artist: &str, track: &str) -> Scrobble {
    Scrobble {
        id: None,
        artist: artist.to_string(),
        album: Some("Test Album".to_string()),
        track: track.to_string(),
        timestamp: DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc),
        source: "test".to_string(),
        source_id: None,
    }
}

#[test]
fn test_session_detection_basic() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:05:00Z", "Artist A", "Track 2"),
        test_scrobble("2024-01-01T10:10:00Z", "Artist B", "Track 3"),
        // 60 min gap - new session
        test_scrobble("2024-01-01T11:10:00Z", "Artist C", "Track 4"),
        test_scrobble("2024-01-01T11:15:00Z", "Artist C", "Track 5"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 2, "Should detect 2 sessions");
    assert_eq!(sessions[0].track_count, 3, "First session has 3 tracks");
    assert_eq!(sessions[1].track_count, 2, "Second session has 2 tracks");
}

#[test]
fn test_session_detection_single_track() {
    let scrobbles = vec![test_scrobble(
        "2024-01-01T10:00:00Z",
        "Artist A",
        "Track 1",
    )];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].track_count, 1);
    assert_eq!(sessions[0].duration_minutes, 0);
}

#[test]
fn test_session_duration_calculation() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:30:00Z", "Artist A", "Track 2"),
        test_scrobble("2024-01-01T11:00:00Z", "Artist B", "Track 3"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].duration_minutes, 60,
        "Session should be 60 minutes"
    );
}

#[test]
fn test_session_unique_artists() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:05:00Z", "Artist B", "Track 2"),
        test_scrobble("2024-01-01T10:10:00Z", "Artist A", "Track 3"),
        test_scrobble("2024-01-01T10:15:00Z", "Artist C", "Track 4"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].unique_artists, 3,
        "Session should have 3 unique artists"
    );
}

#[test]
fn test_empty_scrobbles() {
    let sessions = detect_sessions(vec![], 45);
    assert_eq!(sessions.len(), 0, "Empty scrobbles should return no sessions");
}

#[test]
fn test_session_midnight_boundary() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T23:45:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-02T00:15:00Z", "Artist A", "Track 2"), // 30 min gap
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(
        sessions.len(),
        1,
        "Session should span midnight (30 min gap < 45 min threshold)"
    );
}

#[test]
fn test_session_at_threshold_boundary() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:46:00Z", "Artist A", "Track 2"), // 46 min gap
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(
        sessions.len(),
        2,
        "Gap of 46 min should create new session (> 45 min threshold)"
    );
}

#[test]
fn test_session_gap_information() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:05:00Z", "Artist A", "Track 2"),
        test_scrobble("2024-01-01T10:12:00Z", "Artist B", "Track 3"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);

    let session = &sessions[0];
    assert_eq!(session.tracks[0].gap_after_minutes, Some(5));
    assert_eq!(session.tracks[1].gap_after_minutes, Some(7));
    assert_eq!(session.tracks[2].gap_after_minutes, None); // Last track has no gap
}

#[test]
fn test_session_id_format() {
    let scrobbles = vec![test_scrobble(
        "2024-01-01T10:00:00Z",
        "Artist A",
        "Track 1",
    )];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);

    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
    assert_eq!(
        sessions[0].id,
        format!("session_{}", timestamp.timestamp())
    );
}

#[test]
fn test_multiple_sessions_different_days() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:05:00Z", "Artist A", "Track 2"),
        // Next day
        test_scrobble("2024-01-02T10:00:00Z", "Artist B", "Track 3"),
        test_scrobble("2024-01-02T10:05:00Z", "Artist B", "Track 4"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 2, "Should have 2 sessions on different days");
}

#[test]
fn test_session_sorting() {
    // Test that scrobbles are sorted even if provided out of order
    let scrobbles = vec![
        test_scrobble("2024-01-01T10:10:00Z", "Artist A", "Track 3"),
        test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
        test_scrobble("2024-01-01T10:05:00Z", "Artist A", "Track 2"),
    ];

    let sessions = detect_sessions(scrobbles, 45);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].track_count, 3);

    // Verify tracks are in correct chronological order
    let session = &sessions[0];
    assert_eq!(session.tracks[0].track, "Track 1");
    assert_eq!(session.tracks[1].track, "Track 2");
    assert_eq!(session.tracks[2].track, "Track 3");
}

#[test]
fn test_distribution_buckets() {
    let sessions = vec![
        Session {
            id: "1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_minutes: 15,
            track_count: 5,
            unique_artists: 2,
            tracks: vec![],
        },
        Session {
            id: "2".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_minutes: 45,
            track_count: 15,
            unique_artists: 5,
            tracks: vec![],
        },
        Session {
            id: "3".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_minutes: 90,
            track_count: 25,
            unique_artists: 8,
            tracks: vec![],
        },
        Session {
            id: "4".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_minutes: 200,
            track_count: 60,
            unique_artists: 15,
            tracks: vec![],
        },
    ];

    let distribution = compute_distribution(&sessions);

    // Check duration buckets
    assert_eq!(*distribution.by_duration.get("0-30").unwrap(), 1);
    assert_eq!(*distribution.by_duration.get("30-60").unwrap(), 1);
    assert_eq!(*distribution.by_duration.get("60-120").unwrap(), 1);
    assert_eq!(*distribution.by_duration.get("180+").unwrap(), 1);

    // Check track count buckets
    assert_eq!(*distribution.by_track_count.get("2-10").unwrap(), 1);
    assert_eq!(*distribution.by_track_count.get("10-20").unwrap(), 1);
    assert_eq!(*distribution.by_track_count.get("20-30").unwrap(), 1);
    assert_eq!(*distribution.by_track_count.get("50+").unwrap(), 1);
}

#[test]
fn test_sessions_per_day() {
    let sessions = vec![
        Session {
            id: "1".to_string(),
            start_time: Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap(),
            duration_minutes: 60,
            track_count: 10,
            unique_artists: 3,
            tracks: vec![],
        },
        Session {
            id: "2".to_string(),
            start_time: Utc.with_ymd_and_hms(2024, 1, 1, 14, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap(),
            duration_minutes: 60,
            track_count: 10,
            unique_artists: 3,
            tracks: vec![],
        },
        Session {
            id: "3".to_string(),
            start_time: Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2024, 1, 2, 11, 0, 0).unwrap(),
            duration_minutes: 60,
            track_count: 10,
            unique_artists: 3,
            tracks: vec![],
        },
    ];

    let per_day = compute_sessions_per_day(&sessions);

    assert_eq!(per_day.len(), 2);
    assert_eq!(per_day[0].date, "2024-01-01");
    assert_eq!(per_day[0].count, 2);
    assert_eq!(per_day[1].date, "2024-01-02");
    assert_eq!(per_day[1].count, 1);
}
