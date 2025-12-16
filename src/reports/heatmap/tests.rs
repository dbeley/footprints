use super::*;
use chrono::TimeZone;

fn test_scrobble(timestamp_str: &str) -> Scrobble {
    Scrobble {
        id: None,
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        track: "Test Track".to_string(),
        timestamp: DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc),
        source: "test".to_string(),
        source_id: None,
    }
}

#[test]
fn test_heatmap_basic() {
    // Create scrobbles at specific hours/days
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // Monday 9am UTC
        test_scrobble("2024-01-01T09:30:00Z"), // Monday 9am UTC
        test_scrobble("2024-01-02T14:00:00Z"), // Tuesday 2pm UTC
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Find Monday 9am cell
    let monday_9am = report
        .heatmap
        .iter()
        .find(|c| c.weekday == 0 && c.hour == 9)
        .unwrap();
    assert_eq!(monday_9am.count, 2);

    // Find Tuesday 2pm cell
    let tuesday_2pm = report
        .heatmap
        .iter()
        .find(|c| c.weekday == 1 && c.hour == 14)
        .unwrap();
    assert_eq!(tuesday_2pm.count, 1);

    // Check summary
    assert_eq!(report.summary.total_scrobbles, 3);
}

#[test]
fn test_heatmap_timezone_conversion() {
    // Create scrobble at midnight UTC (Monday)
    let scrobbles = vec![test_scrobble("2024-01-01T00:00:00Z")];

    // Convert to EST (UTC-5)
    let tz: Tz = "America/New_York".parse().unwrap();
    let report = build_heatmap_from_scrobbles(scrobbles, tz, false, None, None).unwrap();

    // Should appear at Sunday 7pm EST (previous day, 5 hours earlier)
    let sunday_7pm = report
        .heatmap
        .iter()
        .find(|c| c.weekday == 6 && c.hour == 19)
        .unwrap();
    assert_eq!(sunday_7pm.count, 1);
}

#[test]
fn test_heatmap_normalization() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // Week 1, Monday 9am
        test_scrobble("2024-01-08T09:00:00Z"), // Week 2, Monday 9am
    ];

    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, true, Some(start), Some(end)).unwrap();

    let monday_9am = report
        .heatmap
        .iter()
        .find(|c| c.weekday == 0 && c.hour == 9)
        .unwrap();

    // 2 scrobbles over 2 weeks = 1.0 per week
    assert_eq!(monday_9am.count, 2);
    assert_eq!(monday_9am.normalized, 1.0);
}

#[test]
fn test_empty_heatmap() {
    let report = build_heatmap_from_scrobbles(vec![], Tz::UTC, false, None, None).unwrap();

    // Should have full 7x24 matrix
    assert_eq!(report.heatmap.len(), 7 * 24);

    // All cells should have zero count
    assert!(report.heatmap.iter().all(|c| c.count == 0));

    // Summary should show zero scrobbles
    assert_eq!(report.summary.total_scrobbles, 0);
}

#[test]
fn test_heatmap_matrix_dimensions() {
    let scrobbles = vec![test_scrobble("2024-01-01T12:00:00Z")];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Should have exactly 168 cells (7 days * 24 hours)
    assert_eq!(report.heatmap.len(), 168);

    // Should have cells for all weekdays (0-6)
    for weekday in 0..7 {
        assert!(report.heatmap.iter().any(|c| c.weekday == weekday));
    }

    // Should have cells for all hours (0-23)
    for hour in 0..24 {
        assert!(report.heatmap.iter().any(|c| c.hour == hour));
    }
}

#[test]
fn test_peak_detection() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // Monday 9am
        test_scrobble("2024-01-01T09:30:00Z"), // Monday 9am
        test_scrobble("2024-01-01T09:45:00Z"), // Monday 9am
        test_scrobble("2024-01-02T14:00:00Z"), // Tuesday 2pm
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Peak should be Monday 9am with 3 scrobbles
    assert_eq!(report.summary.peak_weekday, 0); // Monday
    assert_eq!(report.summary.peak_hour, 9);
    assert_eq!(report.summary.peak_count, 3);
}

#[test]
fn test_weekday_totals() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // Monday
        test_scrobble("2024-01-01T14:00:00Z"), // Monday
        test_scrobble("2024-01-02T10:00:00Z"), // Tuesday
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Monday should have 2 scrobbles
    let monday = report.weekday_totals.iter().find(|d| d.weekday == 0).unwrap();
    assert_eq!(monday.count, 2);
    assert_eq!(monday.name, "Monday");

    // Tuesday should have 1 scrobble
    let tuesday = report.weekday_totals.iter().find(|d| d.weekday == 1).unwrap();
    assert_eq!(tuesday.count, 1);
    assert_eq!(tuesday.name, "Tuesday");
}

#[test]
fn test_hour_totals() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // 9am
        test_scrobble("2024-01-01T09:30:00Z"), // 9am
        test_scrobble("2024-01-02T14:00:00Z"), // 2pm
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Hour 9 should have 2 scrobbles
    let hour_9 = report.hour_totals.iter().find(|h| h.hour == 9).unwrap();
    assert_eq!(hour_9.count, 2);

    // Hour 14 should have 1 scrobble
    let hour_14 = report.hour_totals.iter().find(|h| h.hour == 14).unwrap();
    assert_eq!(hour_14.count, 1);
}

#[test]
fn test_weekday_names() {
    let scrobbles = vec![
        test_scrobble("2024-01-01T09:00:00Z"), // Monday
        test_scrobble("2024-01-07T09:00:00Z"), // Sunday
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Check all weekday names are present
    let names: Vec<String> = report.weekday_totals.iter().map(|d| d.name.clone()).collect();
    assert!(names.contains(&"Monday".to_string()));
    assert!(names.contains(&"Sunday".to_string()));
}

#[test]
fn test_weeks_calculation() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2024, 1, 29, 0, 0, 0).unwrap();

    let scrobbles = vec![test_scrobble("2024-01-15T12:00:00Z")];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, Some(start), Some(end)).unwrap();

    // 28 days = 4 weeks
    assert_eq!(report.summary.weeks_in_range, 4);
}

#[test]
fn test_midnight_edge_case() {
    // Test scrobbles at exactly midnight
    let scrobbles = vec![
        test_scrobble("2024-01-01T00:00:00Z"),
        test_scrobble("2024-01-02T00:00:00Z"),
    ];

    let report = build_heatmap_from_scrobbles(scrobbles, Tz::UTC, false, None, None).unwrap();

    // Both should be in hour 0
    let hour_0 = report.hour_totals.iter().find(|h| h.hour == 0).unwrap();
    assert_eq!(hour_0.count, 2);
}

#[test]
fn test_different_timezones() {
    // Same UTC time, different timezones
    let scrobbles = vec![test_scrobble("2024-01-01T12:00:00Z")]; // Noon UTC

    // UTC: should be Monday 12:00
    let report_utc = build_heatmap_from_scrobbles(scrobbles.clone(), Tz::UTC, false, None, None).unwrap();
    let utc_cell = report_utc.heatmap.iter().find(|c| c.weekday == 0 && c.hour == 12);
    assert!(utc_cell.is_some());
    assert_eq!(utc_cell.unwrap().count, 1);

    // Tokyo (UTC+9): should be Monday 21:00
    let tz_tokyo: Tz = "Asia/Tokyo".parse().unwrap();
    let report_tokyo = build_heatmap_from_scrobbles(scrobbles.clone(), tz_tokyo, false, None, None).unwrap();
    let tokyo_cell = report_tokyo.heatmap.iter().find(|c| c.weekday == 0 && c.hour == 21);
    assert!(tokyo_cell.is_some());
    assert_eq!(tokyo_cell.unwrap().count, 1);

    // Los Angeles (UTC-8): should be Monday 04:00
    let tz_la: Tz = "America/Los_Angeles".parse().unwrap();
    let report_la = build_heatmap_from_scrobbles(scrobbles, tz_la, false, None, None).unwrap();
    let la_cell = report_la.heatmap.iter().find(|c| c.weekday == 0 && c.hour == 4);
    assert!(la_cell.is_some());
    assert_eq!(la_cell.unwrap().count, 1);
}
