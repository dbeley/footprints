use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::DbPool;
use crate::models::Scrobble;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeatmapCell {
    pub weekday: u32, // 0=Monday, 6=Sunday (ISO 8601)
    pub hour: u32,    // 0-23
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
pub struct DayTotal {
    pub weekday: u32,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HourTotal {
    pub hour: u32,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HourData {
    pub hour: u32,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DayGrid {
    pub day_of_week: u32,
    pub hours: Vec<HourData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeakDay {
    pub day_of_week: u32,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeakHour {
    pub hour: u32,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeatmapReport {
    pub grid: Vec<DayGrid>,
    pub peak_day: PeakDay,
    pub peak_hour: PeakHour,
    pub total_scrobbles: i64,
    pub is_normalized: bool,
    // Keep old format for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap: Option<Vec<HeatmapCell>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<HeatmapSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday_totals: Option<Vec<DayTotal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour_totals: Option<Vec<HourTotal>>,
}

/// Generate a heatmap showing listening patterns by hour and weekday
pub fn generate_heatmap(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    timezone: Tz,
    normalize: bool,
) -> Result<HeatmapReport> {
    // Fetch scrobbles in range
    let scrobbles = if let (Some(s), Some(e)) = (start, end) {
        crate::db::get_scrobbles_in_range(pool, s, e)?
    } else {
        // Get all scrobbles
        crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?
    };

    // Build heatmap from scrobbles
    build_heatmap_from_scrobbles(scrobbles, timezone, normalize, start, end)
}

fn build_heatmap_from_scrobbles(
    scrobbles: Vec<Scrobble>,
    timezone: Tz,
    normalize: bool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<HeatmapReport> {
    // Build heatmap matrix (7 weekdays x 24 hours)
    let mut heatmap_matrix: HashMap<(u32, u32), i64> = HashMap::new();

    for scrobble in &scrobbles {
        // Convert to user timezone
        let local_time = scrobble.timestamp.with_timezone(&timezone);
        let weekday = local_time.weekday().num_days_from_monday(); // 0=Monday
        let hour = local_time.hour();

        *heatmap_matrix.entry((weekday, hour)).or_insert(0) += 1;
    }

    // Compute weeks in range (for normalization)
    let weeks_in_range = if let (Some(s), Some(e)) = (start, end) {
        let duration = e.signed_duration_since(s);
        (duration.num_weeks()).max(1)
    } else {
        1
    };

    // Build heatmap cells
    let mut heatmap = Vec::new();
    for weekday in 0..7 {
        for hour in 0..24 {
            let count = *heatmap_matrix.get(&(weekday, hour)).unwrap_or(&0);
            let normalized_value = if normalize {
                count as f64 / weeks_in_range as f64
            } else {
                count as f64
            };

            heatmap.push(HeatmapCell {
                weekday,
                hour,
                count,
                normalized: normalized_value,
            });
        }
    }

    // Find peak cell
    let peak_cell = heatmap
        .iter()
        .max_by_key(|c| c.count)
        .cloned()
        .unwrap_or(HeatmapCell {
            weekday: 0,
            hour: 0,
            count: 0,
            normalized: 0.0,
        });

    let summary = HeatmapSummary {
        total_scrobbles: scrobbles.len() as i64,
        weeks_in_range,
        peak_hour: peak_cell.hour,
        peak_weekday: peak_cell.weekday,
        peak_count: peak_cell.count,
    };

    // Compute weekday totals
    let mut weekday_counts: HashMap<u32, i64> = HashMap::new();
    for cell in &heatmap {
        *weekday_counts.entry(cell.weekday).or_insert(0) += cell.count;
    }

    let weekday_names = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let mut weekday_totals: Vec<DayTotal> = weekday_counts
        .into_iter()
        .map(|(weekday, count)| DayTotal {
            weekday,
            name: weekday_names[weekday as usize].to_string(),
            count,
        })
        .collect();
    weekday_totals.sort_by_key(|d| d.weekday);

    // Compute hour totals
    let mut hour_counts: HashMap<u32, i64> = HashMap::new();
    for cell in &heatmap {
        *hour_counts.entry(cell.hour).or_insert(0) += cell.count;
    }

    let mut hour_totals: Vec<HourTotal> = hour_counts
        .into_iter()
        .map(|(hour, count)| HourTotal { hour, count })
        .collect();
    hour_totals.sort_by_key(|h| h.hour);

    // Build grid structure for frontend (7 days x 24 hours)
    let mut grid: Vec<DayGrid> = Vec::new();
    for weekday in 0..7 {
        let mut hours: Vec<HourData> = Vec::new();
        for hour in 0..24 {
            let count = heatmap
                .iter()
                .find(|c| c.weekday == weekday && c.hour == hour)
                .map(|c| c.count)
                .unwrap_or(0);
            hours.push(HourData { hour, count });
        }
        grid.push(DayGrid {
            day_of_week: weekday,
            hours,
        });
    }

    // Find peak day (by total count across all hours)
    let peak_day = weekday_totals
        .iter()
        .max_by_key(|d| d.count)
        .map(|d| PeakDay {
            day_of_week: d.weekday,
            count: d.count,
        })
        .unwrap_or(PeakDay {
            day_of_week: 0,
            count: 0,
        });

    // Find peak hour (by total count across all days)
    let peak_hour = hour_totals
        .iter()
        .max_by_key(|h| h.count)
        .map(|h| PeakHour {
            hour: h.hour,
            count: h.count,
        })
        .unwrap_or(PeakHour { hour: 0, count: 0 });

    Ok(HeatmapReport {
        grid,
        peak_day,
        peak_hour,
        total_scrobbles: scrobbles.len() as i64,
        is_normalized: normalize,
        heatmap: Some(heatmap),
        summary: Some(summary),
        weekday_totals: Some(weekday_totals),
        hour_totals: Some(hour_totals),
    })
}

#[cfg(test)]
mod tests;
