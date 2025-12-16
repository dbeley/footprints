use crate::db::DbPool;
use crate::models::Scrobble;
use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct YearlyReport {
    pub year: i32,
    pub overview: YearOverview,
    pub top_content: TopContent,
    pub listening_patterns: ListeningPatterns,
    pub discoveries: Discoveries,
    pub diversity: DiversityStats,
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YearOverview {
    pub total_scrobbles: i64,
    pub total_artists: i64,
    pub total_tracks: i64,
    pub total_albums: i64,
    pub total_minutes: i64,
    pub average_per_day: f64,
    pub most_active_month: String,
    pub most_active_day: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopContent {
    pub top_artists: Vec<TopArtist>,
    pub top_tracks: Vec<TopTrack>,
    pub top_albums: Vec<TopAlbum>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopArtist {
    pub artist: String,
    pub play_count: i64,
    pub percentage: f64,
    pub rank: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopTrack {
    pub artist: String,
    pub track: String,
    pub play_count: i64,
    pub rank: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopAlbum {
    pub artist: String,
    pub album: String,
    pub play_count: i64,
    pub rank: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListeningPatterns {
    pub peak_hour: u32,
    pub peak_day: u32,
    pub longest_session_minutes: i64,
    pub avg_session_minutes: f64,
    pub night_owl_score: f64,
    pub early_bird_score: f64,
    pub weekend_warrior_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Discoveries {
    pub new_artists: i64,
    pub new_tracks: i64,
    pub first_artist: Option<FirstPlay>,
    pub top_discovery: Option<TopDiscovery>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirstPlay {
    pub artist: String,
    pub track: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopDiscovery {
    pub artist: String,
    pub first_heard: DateTime<Utc>,
    pub plays_this_year: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiversityStats {
    pub diversity_score: f64,
    pub genre_count: i64,
    pub artist_loyalty: f64,
    pub exploration_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Milestone {
    pub title: String,
    pub description: String,
    pub value: String,
    pub icon: String,
}

/// Generate year-over-year comparison
#[derive(Debug, Serialize, Deserialize)]
pub struct YearComparison {
    pub current_year: i32,
    pub previous_year: i32,
    pub scrobbles_change: i64,
    pub scrobbles_change_percent: f64,
    pub artists_change: i64,
    pub artists_change_percent: f64,
    pub diversity_change: f64,
    pub top_artists_overlap: Vec<String>,
    pub new_favorites: Vec<String>,
}

pub fn generate_yearly_report(pool: &DbPool, year: i32) -> Result<YearlyReport> {
    let start = format!("{}-01-01T00:00:00Z", year).parse()?;
    let end = format!("{}-12-31T23:59:59Z", year).parse()?;

    let scrobbles = crate::db::get_scrobbles_in_range(pool, start, end)?;

    if scrobbles.is_empty() {
        return Ok(create_empty_report(year));
    }

    let overview = compute_overview(&scrobbles, year);
    let top_content = compute_top_content(&scrobbles);
    let listening_patterns = compute_listening_patterns(&scrobbles);
    let discoveries = compute_discoveries(&scrobbles, pool, year)?;
    let diversity = compute_diversity_stats(&scrobbles);
    let milestones = compute_milestones(&overview, &top_content, &listening_patterns, &discoveries);

    Ok(YearlyReport {
        year,
        overview,
        top_content,
        listening_patterns,
        discoveries,
        diversity,
        milestones,
    })
}

pub fn generate_year_comparison(pool: &DbPool, year1: i32, year2: i32) -> Result<YearComparison> {
    let report1 = generate_yearly_report(pool, year1)?;
    let report2 = generate_yearly_report(pool, year2)?;

    let scrobbles_change = report1.overview.total_scrobbles - report2.overview.total_scrobbles;
    let scrobbles_change_percent = if report2.overview.total_scrobbles > 0 {
        (scrobbles_change as f64 / report2.overview.total_scrobbles as f64) * 100.0
    } else {
        0.0
    };

    let artists_change = report1.overview.total_artists - report2.overview.total_artists;
    let artists_change_percent = if report2.overview.total_artists > 0 {
        (artists_change as f64 / report2.overview.total_artists as f64) * 100.0
    } else {
        0.0
    };

    let diversity_change = report1.diversity.diversity_score - report2.diversity.diversity_score;

    // Find top artists overlap
    let top1: Vec<String> = report1
        .top_content
        .top_artists
        .iter()
        .take(10)
        .map(|a| a.artist.clone())
        .collect();

    let top2_set: std::collections::HashSet<String> = report2
        .top_content
        .top_artists
        .iter()
        .take(10)
        .map(|a| a.artist.clone())
        .collect();

    let overlap: Vec<String> = top1
        .iter()
        .filter(|a| top2_set.contains(*a))
        .cloned()
        .collect();

    let new_favorites: Vec<String> = top1
        .iter()
        .filter(|a| !top2_set.contains(*a))
        .cloned()
        .collect();

    Ok(YearComparison {
        current_year: year1,
        previous_year: year2,
        scrobbles_change,
        scrobbles_change_percent,
        artists_change,
        artists_change_percent,
        diversity_change,
        top_artists_overlap: overlap,
        new_favorites,
    })
}

fn compute_overview(scrobbles: &[Scrobble], year: i32) -> YearOverview {
    let total_scrobbles = scrobbles.len() as i64;

    let unique_artists: std::collections::HashSet<_> =
        scrobbles.iter().map(|s| s.artist.as_str()).collect();

    let unique_tracks: std::collections::HashSet<_> = scrobbles
        .iter()
        .map(|s| (s.artist.as_str(), s.track.as_str()))
        .collect();

    let unique_albums: std::collections::HashSet<_> = scrobbles
        .iter()
        .filter_map(|s| s.album.as_ref())
        .map(|a| a.as_str())
        .collect();

    // Estimate total minutes (average 3.5 minutes per track)
    let total_minutes = (total_scrobbles as f64 * 3.5) as i64;

    // Average per day
    let days_in_year = if is_leap_year(year) { 366 } else { 365 };
    let average_per_day = total_scrobbles as f64 / days_in_year as f64;

    // Most active month
    let mut month_counts: HashMap<u32, i64> = HashMap::new();
    for scrobble in scrobbles {
        *month_counts.entry(scrobble.timestamp.month()).or_insert(0) += 1;
    }
    let most_active_month = month_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(month, _)| format!("{}-{:02}", year, month))
        .unwrap_or_default();

    // Most active day
    let mut day_counts: HashMap<String, i64> = HashMap::new();
    for scrobble in scrobbles {
        let day = scrobble.timestamp.format("%Y-%m-%d").to_string();
        *day_counts.entry(day).or_insert(0) += 1;
    }
    let most_active_day = day_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(day, _)| day.clone())
        .unwrap_or_default();

    YearOverview {
        total_scrobbles,
        total_artists: unique_artists.len() as i64,
        total_tracks: unique_tracks.len() as i64,
        total_albums: unique_albums.len() as i64,
        total_minutes,
        average_per_day,
        most_active_month,
        most_active_day,
    }
}

fn compute_top_content(scrobbles: &[Scrobble]) -> TopContent {
    // Top artists
    let mut artist_counts: HashMap<String, i64> = HashMap::new();
    for scrobble in scrobbles {
        *artist_counts.entry(scrobble.artist.clone()).or_insert(0) += 1;
    }

    let total_scrobbles = scrobbles.len() as f64;
    let mut top_artists: Vec<_> = artist_counts.into_iter().collect();
    top_artists.sort_by(|a, b| b.1.cmp(&a.1));

    let top_artists: Vec<TopArtist> = top_artists
        .into_iter()
        .take(50)
        .enumerate()
        .map(|(i, (artist, count))| TopArtist {
            artist,
            play_count: count,
            percentage: (count as f64 / total_scrobbles) * 100.0,
            rank: i + 1,
        })
        .collect();

    // Top tracks
    let mut track_counts: HashMap<(String, String), i64> = HashMap::new();
    for scrobble in scrobbles {
        *track_counts
            .entry((scrobble.artist.clone(), scrobble.track.clone()))
            .or_insert(0) += 1;
    }

    let mut top_tracks: Vec<_> = track_counts.into_iter().collect();
    top_tracks.sort_by(|a, b| b.1.cmp(&a.1));

    let top_tracks: Vec<TopTrack> = top_tracks
        .into_iter()
        .take(50)
        .enumerate()
        .map(|(i, ((artist, track), count))| TopTrack {
            artist,
            track,
            play_count: count,
            rank: i + 1,
        })
        .collect();

    // Top albums
    let mut album_counts: HashMap<(String, String), i64> = HashMap::new();
    for scrobble in scrobbles {
        if let Some(album) = &scrobble.album {
            *album_counts
                .entry((scrobble.artist.clone(), album.clone()))
                .or_insert(0) += 1;
        }
    }

    let mut top_albums: Vec<_> = album_counts.into_iter().collect();
    top_albums.sort_by(|a, b| b.1.cmp(&a.1));

    let top_albums: Vec<TopAlbum> = top_albums
        .into_iter()
        .take(50)
        .enumerate()
        .map(|(i, ((artist, album), count))| TopAlbum {
            artist,
            album,
            play_count: count,
            rank: i + 1,
        })
        .collect();

    TopContent {
        top_artists,
        top_tracks,
        top_albums,
    }
}

fn compute_listening_patterns(scrobbles: &[Scrobble]) -> ListeningPatterns {
    // Hour distribution
    let mut hour_counts: HashMap<u32, i64> = HashMap::new();
    for scrobble in scrobbles {
        *hour_counts.entry(scrobble.timestamp.hour()).or_insert(0) += 1;
    }
    let peak_hour = hour_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(hour, _)| *hour)
        .unwrap_or(0);

    // Day distribution
    let mut day_counts: HashMap<u32, i64> = HashMap::new();
    for scrobble in scrobbles {
        *day_counts
            .entry(scrobble.timestamp.weekday().num_days_from_monday())
            .or_insert(0) += 1;
    }
    let peak_day = day_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(day, _)| *day)
        .unwrap_or(0);

    // Sessions (simple: gap > 30 min = new session)
    let mut sorted_scrobbles = scrobbles.to_vec();
    sorted_scrobbles.sort_by_key(|s| s.timestamp);

    let mut session_durations = Vec::new();
    let mut current_session_start = sorted_scrobbles[0].timestamp;

    for window in sorted_scrobbles.windows(2) {
        let gap = window[1]
            .timestamp
            .signed_duration_since(window[0].timestamp)
            .num_minutes();

        if gap > 30 {
            let duration = window[0]
                .timestamp
                .signed_duration_since(current_session_start)
                .num_minutes();
            session_durations.push(duration);
            current_session_start = window[1].timestamp;
        }
    }

    let longest_session_minutes = session_durations.iter().max().copied().unwrap_or(0);
    let avg_session_minutes = if !session_durations.is_empty() {
        session_durations.iter().sum::<i64>() as f64 / session_durations.len() as f64
    } else {
        0.0
    };

    // Listening personality scores
    let night_owl_score = calculate_night_owl_score(&hour_counts);
    let early_bird_score = calculate_early_bird_score(&hour_counts);
    let weekend_warrior_score = calculate_weekend_warrior_score(&day_counts);

    ListeningPatterns {
        peak_hour,
        peak_day,
        longest_session_minutes,
        avg_session_minutes,
        night_owl_score,
        early_bird_score,
        weekend_warrior_score,
    }
}

fn compute_discoveries(scrobbles: &[Scrobble], pool: &DbPool, year: i32) -> Result<Discoveries> {
    // Get all scrobbles before this year to determine what's "new"
    let year_start: DateTime<Utc> = format!("{}-01-01T00:00:00Z", year).parse()?;
    let all_time_scrobbles = crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?;

    let mut seen_artists: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_tracks: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    // Mark everything before year as "seen"
    for scrobble in &all_time_scrobbles {
        if scrobble.timestamp < year_start {
            seen_artists.insert(scrobble.artist.clone());
            seen_tracks.insert((scrobble.artist.clone(), scrobble.track.clone()));
        }
    }

    let mut new_artists = 0i64;
    let mut new_tracks = 0i64;
    let mut first_artist: Option<FirstPlay> = None;
    let mut discoveries_map: HashMap<String, (DateTime<Utc>, i64)> = HashMap::new();

    for scrobble in scrobbles {
        let track_key = (scrobble.artist.clone(), scrobble.track.clone());

        if !seen_artists.contains(&scrobble.artist) {
            new_artists += 1;
            seen_artists.insert(scrobble.artist.clone());

            if first_artist.is_none() {
                first_artist = Some(FirstPlay {
                    artist: scrobble.artist.clone(),
                    track: scrobble.track.clone(),
                    timestamp: scrobble.timestamp,
                });
            }

            discoveries_map.insert(scrobble.artist.clone(), (scrobble.timestamp, 0));
        }

        if !seen_tracks.contains(&track_key) {
            new_tracks += 1;
            seen_tracks.insert(track_key);
        }

        // Count plays for discovered artists
        if let Some((_, count)) = discoveries_map.get_mut(&scrobble.artist) {
            *count += 1;
        }
    }

    let top_discovery = discoveries_map
        .into_iter()
        .max_by_key(|(_, (_, count))| *count)
        .map(|(artist, (first_heard, plays))| TopDiscovery {
            artist,
            first_heard,
            plays_this_year: plays,
        });

    Ok(Discoveries {
        new_artists,
        new_tracks,
        first_artist,
        top_discovery,
    })
}

fn compute_diversity_stats(scrobbles: &[Scrobble]) -> DiversityStats {
    let unique_artists: std::collections::HashSet<_> =
        scrobbles.iter().map(|s| s.artist.as_str()).collect();

    let total_scrobbles = scrobbles.len() as f64;
    let unique_count = unique_artists.len() as f64;

    // Simple diversity score: unique / total
    let diversity_score = (unique_count / total_scrobbles * 100.0).min(100.0);

    // Artist loyalty: percentage of top artist
    let mut artist_counts: HashMap<String, i64> = HashMap::new();
    for scrobble in scrobbles {
        *artist_counts.entry(scrobble.artist.clone()).or_insert(0) += 1;
    }

    let top_artist_plays = artist_counts.values().max().copied().unwrap_or(0);
    let artist_loyalty = (top_artist_plays as f64 / total_scrobbles) * 100.0;

    // Exploration score: inverse of loyalty
    let exploration_score = 100.0 - artist_loyalty;

    DiversityStats {
        diversity_score,
        genre_count: 0, // Placeholder for future genre integration
        artist_loyalty,
        exploration_score,
    }
}

fn compute_milestones(
    overview: &YearOverview,
    top_content: &TopContent,
    patterns: &ListeningPatterns,
    discoveries: &Discoveries,
) -> Vec<Milestone> {
    let mut milestones = Vec::new();

    // Total listening milestone
    let hours = overview.total_minutes / 60;
    milestones.push(Milestone {
        title: "Music Marathon".to_string(),
        description: format!("You listened to {} hours of music", hours),
        value: format!("{} hours", hours),
        icon: "⏱️".to_string(),
    });

    // Top artist milestone
    if let Some(top_artist) = top_content.top_artists.first() {
        milestones.push(Milestone {
            title: "Your #1 Artist".to_string(),
            description: format!("You played {} songs", top_artist.play_count),
            value: top_artist.artist.clone(),
            icon: "🎤".to_string(),
        });
    }

    // Discovery milestone
    milestones.push(Milestone {
        title: "Explorer".to_string(),
        description: format!("You discovered {} new artists", discoveries.new_artists),
        value: format!("{} artists", discoveries.new_artists),
        icon: "🗺️".to_string(),
    });

    // Personality trait
    if patterns.night_owl_score > 60.0 {
        milestones.push(Milestone {
            title: "Night Owl".to_string(),
            description: "Most of your listening happens after 8 PM".to_string(),
            value: format!("{}% night listening", patterns.night_owl_score as i32),
            icon: "🦉".to_string(),
        });
    } else if patterns.early_bird_score > 60.0 {
        milestones.push(Milestone {
            title: "Early Bird".to_string(),
            description: "You love morning music sessions".to_string(),
            value: format!("{}% morning listening", patterns.early_bird_score as i32),
            icon: "🐦".to_string(),
        });
    }

    // Longest session
    if patterns.longest_session_minutes > 180 {
        milestones.push(Milestone {
            title: "Marathon Listener".to_string(),
            description: "Your longest listening session".to_string(),
            value: format!("{} minutes", patterns.longest_session_minutes),
            icon: "🏃".to_string(),
        });
    }

    milestones
}

fn calculate_night_owl_score(hour_counts: &HashMap<u32, i64>) -> f64 {
    let total: i64 = hour_counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let night_hours: i64 = (20..24)
        .chain(0..6)
        .map(|h| hour_counts.get(&h).copied().unwrap_or(0))
        .sum();

    (night_hours as f64 / total as f64) * 100.0
}

fn calculate_early_bird_score(hour_counts: &HashMap<u32, i64>) -> f64 {
    let total: i64 = hour_counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let morning_hours: i64 = (6..12)
        .map(|h| hour_counts.get(&h).copied().unwrap_or(0))
        .sum();

    (morning_hours as f64 / total as f64) * 100.0
}

fn calculate_weekend_warrior_score(day_counts: &HashMap<u32, i64>) -> f64 {
    let total: i64 = day_counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let weekend_count: i64 = [5, 6] // Saturday, Sunday
        .iter()
        .map(|d| day_counts.get(d).copied().unwrap_or(0))
        .sum();

    (weekend_count as f64 / total as f64) * 100.0
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn create_empty_report(year: i32) -> YearlyReport {
    YearlyReport {
        year,
        overview: YearOverview {
            total_scrobbles: 0,
            total_artists: 0,
            total_tracks: 0,
            total_albums: 0,
            total_minutes: 0,
            average_per_day: 0.0,
            most_active_month: String::new(),
            most_active_day: String::new(),
        },
        top_content: TopContent {
            top_artists: Vec::new(),
            top_tracks: Vec::new(),
            top_albums: Vec::new(),
        },
        listening_patterns: ListeningPatterns {
            peak_hour: 0,
            peak_day: 0,
            longest_session_minutes: 0,
            avg_session_minutes: 0.0,
            night_owl_score: 0.0,
            early_bird_score: 0.0,
            weekend_warrior_score: 0.0,
        },
        discoveries: Discoveries {
            new_artists: 0,
            new_tracks: 0,
            first_artist: None,
            top_discovery: None,
        },
        diversity: DiversityStats {
            diversity_score: 0.0,
            genre_count: 0,
            artist_loyalty: 0.0,
            exploration_score: 0.0,
        },
        milestones: Vec::new(),
    }
}
