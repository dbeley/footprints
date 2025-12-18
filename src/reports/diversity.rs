use crate::db::DbPool;
use crate::models::Scrobble;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Day,
    Week,
    Month,
    Year,
}

impl Granularity {
    pub fn format_period(&self, date: &DateTime<Utc>) -> String {
        match self {
            Granularity::Day => date.format("%Y-%m-%d").to_string(),
            Granularity::Week => date.format("%Y-W%W").to_string(),
            Granularity::Month => date.format("%Y-%m").to_string(),
            Granularity::Year => date.format("%Y").to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiversityPoint {
    pub period: String,
    pub total_scrobbles: i64,
    pub unique_artists: i64,
    pub unique_tracks: i64,
    pub shannon_entropy: f64,
    pub gini_coefficient: f64,
    pub diversity_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiversitySummary {
    pub total_scrobbles: i64,
    pub total_unique_artists: i64,
    pub total_unique_tracks: i64,
    pub avg_diversity_score: f64,
    pub avg_shannon_entropy: f64,
    pub avg_gini_coefficient: f64,
    pub most_diverse_period: String,
    pub least_diverse_period: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiversityReport {
    pub timeline: Vec<DiversityPoint>,
    pub summary: DiversitySummary,
}

/// Calculate Shannon entropy for artist distribution
/// H = -Σ(p_i * log2(p_i))
/// where p_i is the probability of artist i
fn calculate_shannon_entropy(artist_counts: &HashMap<String, i64>, total: i64) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0;
    for &count in artist_counts.values() {
        let p = count as f64 / total as f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Calculate Gini coefficient for artist concentration
/// Measures inequality in artist play distribution
/// 0 = perfect equality, 1 = maximum inequality
fn calculate_gini_coefficient(artist_counts: &HashMap<String, i64>) -> f64 {
    if artist_counts.is_empty() {
        return 0.0;
    }

    let mut counts: Vec<i64> = artist_counts.values().copied().collect();
    counts.sort_unstable();

    let n = counts.len() as f64;
    let sum: i64 = counts.iter().sum();

    if sum == 0 {
        return 0.0;
    }

    let mut numerator = 0.0;
    for (i, &count) in counts.iter().enumerate() {
        numerator += ((i + 1) as f64) * (count as f64);
    }

    let gini = (2.0 * numerator) / (n * sum as f64) - (n + 1.0) / n;
    gini.clamp(0.0, 1.0)
}

/// Calculate diversity score (0-100)
/// Combines entropy and uniqueness ratio
fn calculate_diversity_score(
    unique_artists: i64,
    total_scrobbles: i64,
    shannon_entropy: f64,
) -> f64 {
    if total_scrobbles == 0 {
        return 0.0;
    }

    let uniqueness_ratio = unique_artists as f64 / total_scrobbles as f64;

    // Normalize entropy (max entropy for 100 artists ≈ 6.64)
    let normalized_entropy = (shannon_entropy / 6.64).min(1.0);

    // Weighted combination: 60% entropy, 40% uniqueness
    let score = (normalized_entropy * 0.6 + uniqueness_ratio * 0.4) * 100.0;
    score.clamp(0.0, 100.0)
}

pub fn generate_diversity_report(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    granularity: Granularity,
) -> Result<DiversityReport> {
    let scrobbles = if let (Some(s), Some(e)) = (start, end) {
        crate::db::get_scrobbles_in_range(pool, s, e)?
    } else {
        crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?
    };

    if scrobbles.is_empty() {
        return Ok(DiversityReport {
            timeline: Vec::new(),
            summary: DiversitySummary {
                total_scrobbles: 0,
                total_unique_artists: 0,
                total_unique_tracks: 0,
                avg_diversity_score: 0.0,
                avg_shannon_entropy: 0.0,
                avg_gini_coefficient: 0.0,
                most_diverse_period: String::new(),
                least_diverse_period: String::new(),
            },
        });
    }

    // Group scrobbles by period
    let mut period_scrobbles: HashMap<String, Vec<&Scrobble>> = HashMap::new();
    for scrobble in &scrobbles {
        let period = granularity.format_period(&scrobble.timestamp);
        period_scrobbles.entry(period).or_default().push(scrobble);
    }

    // Build timeline
    let mut timeline = Vec::new();
    for (period, period_scrobbles_list) in &period_scrobbles {
        let point = compute_diversity_point(period.clone(), period_scrobbles_list);
        timeline.push(point);
    }

    // Sort timeline by period
    timeline.sort_by(|a, b| a.period.cmp(&b.period));

    // Compute summary
    let summary = compute_diversity_summary(&timeline, &scrobbles);

    Ok(DiversityReport { timeline, summary })
}

fn compute_diversity_point(period: String, scrobbles: &[&Scrobble]) -> DiversityPoint {
    let total_scrobbles = scrobbles.len() as i64;

    // Count artists
    let mut artist_counts: HashMap<String, i64> = HashMap::new();
    for scrobble in scrobbles {
        *artist_counts.entry(scrobble.artist.clone()).or_insert(0) += 1;
    }

    // Count unique tracks
    let unique_tracks: std::collections::HashSet<_> = scrobbles
        .iter()
        .map(|s| (s.artist.as_str(), s.track.as_str()))
        .collect();

    let unique_artists = artist_counts.len() as i64;
    let unique_tracks_count = unique_tracks.len() as i64;

    let shannon_entropy = calculate_shannon_entropy(&artist_counts, total_scrobbles);
    let gini_coefficient = calculate_gini_coefficient(&artist_counts);
    let diversity_score =
        calculate_diversity_score(unique_artists, total_scrobbles, shannon_entropy);

    DiversityPoint {
        period,
        total_scrobbles,
        unique_artists,
        unique_tracks: unique_tracks_count,
        shannon_entropy,
        gini_coefficient,
        diversity_score,
    }
}

fn compute_diversity_summary(
    timeline: &[DiversityPoint],
    scrobbles: &[Scrobble],
) -> DiversitySummary {
    let total_scrobbles = scrobbles.len() as i64;

    let unique_artists: std::collections::HashSet<_> =
        scrobbles.iter().map(|s| s.artist.as_str()).collect();

    let unique_tracks: std::collections::HashSet<_> = scrobbles
        .iter()
        .map(|s| (s.artist.as_str(), s.track.as_str()))
        .collect();

    let avg_diversity_score = if !timeline.is_empty() {
        timeline.iter().map(|p| p.diversity_score).sum::<f64>() / timeline.len() as f64
    } else {
        0.0
    };

    let avg_shannon_entropy = if !timeline.is_empty() {
        timeline.iter().map(|p| p.shannon_entropy).sum::<f64>() / timeline.len() as f64
    } else {
        0.0
    };

    let avg_gini_coefficient = if !timeline.is_empty() {
        timeline.iter().map(|p| p.gini_coefficient).sum::<f64>() / timeline.len() as f64
    } else {
        0.0
    };

    let most_diverse = timeline
        .iter()
        .max_by(|a, b| {
            a.diversity_score
                .partial_cmp(&b.diversity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| p.period.clone())
        .unwrap_or_default();

    let least_diverse = timeline
        .iter()
        .min_by(|a, b| {
            a.diversity_score
                .partial_cmp(&b.diversity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| p.period.clone())
        .unwrap_or_default();

    DiversitySummary {
        total_scrobbles,
        total_unique_artists: unique_artists.len() as i64,
        total_unique_tracks: unique_tracks.len() as i64,
        avg_diversity_score,
        avg_shannon_entropy,
        avg_gini_coefficient,
        most_diverse_period: most_diverse,
        least_diverse_period: least_diverse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Scrobble;

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
    fn test_shannon_entropy_uniform() {
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        counts.insert("C".to_string(), 10);
        counts.insert("D".to_string(), 10);

        let entropy = calculate_shannon_entropy(&counts, 40);
        assert!((entropy - 2.0).abs() < 0.01); // log2(4) = 2
    }

    #[test]
    fn test_shannon_entropy_concentrated() {
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 37);
        counts.insert("B".to_string(), 1);
        counts.insert("C".to_string(), 1);
        counts.insert("D".to_string(), 1);

        let entropy = calculate_shannon_entropy(&counts, 40);
        assert!(entropy < 1.0); // Very concentrated = low entropy
    }

    #[test]
    fn test_gini_coefficient_equal() {
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        counts.insert("C".to_string(), 10);

        let gini = calculate_gini_coefficient(&counts);
        assert!(gini < 0.01); // Near zero = equal distribution
    }

    #[test]
    fn test_gini_coefficient_unequal() {
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 90);
        counts.insert("B".to_string(), 5);
        counts.insert("C".to_string(), 5);

        let gini = calculate_gini_coefficient(&counts);
        assert!(gini > 0.4); // High inequality
    }

    #[test]
    fn test_diversity_score() {
        let score = calculate_diversity_score(50, 100, 4.0);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_diversity_point_calculation() {
        let scrobbles = [
            test_scrobble("2024-01-01T10:00:00Z", "Artist A", "Track 1"),
            test_scrobble("2024-01-01T10:05:00Z", "Artist B", "Track 2"),
            test_scrobble("2024-01-01T10:10:00Z", "Artist A", "Track 3"),
            test_scrobble("2024-01-01T10:15:00Z", "Artist C", "Track 4"),
        ];

        let scrobble_refs: Vec<_> = scrobbles.iter().collect();
        let point = compute_diversity_point("2024-01-01".to_string(), &scrobble_refs);

        assert_eq!(point.total_scrobbles, 4);
        assert_eq!(point.unique_artists, 3);
        assert_eq!(point.unique_tracks, 4);
        assert!(point.shannon_entropy > 0.0);
        assert!(point.diversity_score > 0.0);
    }
}
