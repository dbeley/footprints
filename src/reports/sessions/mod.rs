use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::db::DbPool;
use crate::models::Scrobble;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String, // Format: "session_{start_timestamp}"
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
pub struct SessionDistribution {
    pub by_duration: std::collections::HashMap<String, usize>,
    pub by_track_count: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsReport {
    pub sessions: Vec<Session>,
    pub summary: SessionsSummary,
    pub distribution: SessionDistribution,
    pub sessions_per_day: Vec<DayCount>,
}

/// Detect listening sessions from a list of scrobbles
///
/// A new session starts when the gap between consecutive scrobbles exceeds
/// the gap_threshold_minutes parameter (default: 45 minutes).
///
/// # Arguments
/// * `scrobbles` - List of scrobbles (will be sorted by timestamp)
/// * `gap_threshold_minutes` - Maximum gap between scrobbles in the same session
///
/// # Returns
/// List of detected sessions with metadata
pub fn detect_sessions(mut scrobbles: Vec<Scrobble>, gap_threshold_minutes: i64) -> Vec<Session> {
    if scrobbles.is_empty() {
        return Vec::new();
    }

    // Sort by timestamp ascending
    scrobbles.sort_by_key(|s| s.timestamp);

    let gap_threshold = Duration::minutes(gap_threshold_minutes);
    let mut sessions = Vec::new();
    let mut current_session_tracks: Vec<Scrobble> = Vec::new();

    for scrobble in scrobbles {
        if current_session_tracks.is_empty() {
            // Start first session
            current_session_tracks.push(scrobble);
        } else {
            // Check gap from last track
            let last_track = current_session_tracks.last().unwrap();
            let gap = scrobble.timestamp.signed_duration_since(last_track.timestamp);

            if gap > gap_threshold {
                // End current session, start new one
                sessions.push(build_session(current_session_tracks));
                current_session_tracks = vec![scrobble];
            } else {
                // Continue current session
                current_session_tracks.push(scrobble);
            }
        }
    }

    // Don't forget last session
    if !current_session_tracks.is_empty() {
        sessions.push(build_session(current_session_tracks));
    }

    sessions
}

/// Build a Session from a list of scrobbles
fn build_session(tracks: Vec<Scrobble>) -> Session {
    let start_time = tracks.first().unwrap().timestamp;
    let end_time = tracks.last().unwrap().timestamp;
    let duration_minutes = end_time
        .signed_duration_since(start_time)
        .num_minutes()
        .max(0); // Ensure non-negative

    let unique_artists: HashSet<String> = tracks.iter().map(|t| t.artist.clone()).collect();

    let mut session_tracks: Vec<SessionTrack> = Vec::new();

    // Build tracks with gap information
    for i in 0..tracks.len() {
        let gap_after_minutes = if i < tracks.len() - 1 {
            let gap = tracks[i + 1]
                .timestamp
                .signed_duration_since(tracks[i].timestamp)
                .num_minutes();
            Some(gap)
        } else {
            None
        };

        session_tracks.push(SessionTrack {
            artist: tracks[i].artist.clone(),
            album: tracks[i].album.clone(),
            track: tracks[i].track.clone(),
            timestamp: tracks[i].timestamp,
            gap_after_minutes,
        });
    }

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

/// Generate a comprehensive sessions report
pub fn generate_sessions_report(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    gap_minutes: i64,
    source: Option<String>,
    min_tracks: usize,
) -> Result<SessionsReport> {
    // Fetch scrobbles in range (no pagination - get all)
    let mut scrobbles = if let (Some(s), Some(e)) = (start, end) {
        crate::db::get_scrobbles_in_range(pool, s, e)?
    } else {
        // Get all scrobbles (use a very large limit)
        crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?
    };

    // Filter by source if specified
    if let Some(src) = source {
        scrobbles.retain(|s| s.source == src);
    }

    // Detect sessions
    let mut sessions = detect_sessions(scrobbles, gap_minutes);

    // Filter by minimum track count
    sessions.retain(|s| s.track_count >= min_tracks);

    // Compute summary
    let total_sessions = sessions.len();
    let avg_duration_minutes = if total_sessions > 0 {
        sessions.iter().map(|s| s.duration_minutes).sum::<i64>() as f64 / total_sessions as f64
    } else {
        0.0
    };
    let avg_tracks_per_session = if total_sessions > 0 {
        sessions.iter().map(|s| s.track_count).sum::<usize>() as f64 / total_sessions as f64
    } else {
        0.0
    };
    let longest_session_minutes = sessions
        .iter()
        .map(|s| s.duration_minutes)
        .max()
        .unwrap_or(0);
    let total_listening_hours = sessions.iter().map(|s| s.duration_minutes).sum::<i64>() as f64 / 60.0;

    let summary = SessionsSummary {
        total_sessions,
        avg_duration_minutes,
        avg_tracks_per_session,
        longest_session_minutes,
        total_listening_hours,
    };

    // Compute distribution
    let distribution = compute_distribution(&sessions);

    // Compute sessions per day
    let sessions_per_day = compute_sessions_per_day(&sessions);

    Ok(SessionsReport {
        sessions,
        summary,
        distribution,
        sessions_per_day,
    })
}

fn compute_distribution(sessions: &[Session]) -> SessionDistribution {
    use std::collections::HashMap;

    let mut by_duration: HashMap<String, usize> = HashMap::new();
    let mut by_track_count: HashMap<String, usize> = HashMap::new();

    // Initialize buckets
    by_duration.insert("0-30".to_string(), 0);
    by_duration.insert("30-60".to_string(), 0);
    by_duration.insert("60-120".to_string(), 0);
    by_duration.insert("120-180".to_string(), 0);
    by_duration.insert("180+".to_string(), 0);

    by_track_count.insert("2-10".to_string(), 0);
    by_track_count.insert("10-20".to_string(), 0);
    by_track_count.insert("20-30".to_string(), 0);
    by_track_count.insert("30-50".to_string(), 0);
    by_track_count.insert("50+".to_string(), 0);

    for session in sessions {
        // Duration bucket
        let duration_bucket = match session.duration_minutes {
            0..=29 => "0-30",
            30..=59 => "30-60",
            60..=119 => "60-120",
            120..=179 => "120-180",
            _ => "180+",
        };
        *by_duration.get_mut(duration_bucket).unwrap() += 1;

        // Track count bucket
        let track_bucket = match session.track_count {
            0..=10 => "2-10",
            11..=20 => "10-20",
            21..=30 => "20-30",
            31..=50 => "30-50",
            _ => "50+",
        };
        *by_track_count.get_mut(track_bucket).unwrap() += 1;
    }

    SessionDistribution {
        by_duration,
        by_track_count,
    }
}

fn compute_sessions_per_day(sessions: &[Session]) -> Vec<DayCount> {
    use std::collections::HashMap;

    let mut counts: HashMap<String, usize> = HashMap::new();

    for session in sessions {
        let date = session.start_time.format("%Y-%m-%d").to_string();
        *counts.entry(date).or_insert(0) += 1;
    }

    let mut result: Vec<DayCount> = counts
        .into_iter()
        .map(|(date, count)| DayCount { date, count })
        .collect();

    // Sort by date
    result.sort_by(|a, b| a.date.cmp(&b.date));

    result
}

#[cfg(test)]
mod tests;
