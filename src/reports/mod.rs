use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub period: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_scrobbles: i64,
    pub top_artists: Vec<(String, i64)>,
    pub top_tracks: Vec<(String, String, i64)>,
    pub top_albums: Vec<(String, String, i64)>,
}

pub fn generate_yearly_report(pool: &DbPool, year: i32) -> Result<Report> {
    let start_date = chrono::Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let end_date = chrono::Utc
        .with_ymd_and_hms(year, 12, 31, 23, 59, 59)
        .unwrap();

    generate_report(pool, start_date, end_date, format!("Year {}", year))
}

pub fn generate_monthly_report(pool: &DbPool, year: i32, month: u32) -> Result<Report> {
    let start_date = chrono::Utc
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .unwrap();

    let next_month = if month == 12 {
        chrono::Utc
            .with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0)
            .unwrap()
    } else {
        chrono::Utc
            .with_ymd_and_hms(year, month + 1, 1, 0, 0, 0)
            .unwrap()
    };

    let end_date = next_month - Duration::seconds(1);

    generate_report(pool, start_date, end_date, format!("{}-{:02}", year, month))
}

pub fn generate_last_month_report(pool: &DbPool) -> Result<Report> {
    let now = Utc::now();
    let (year, month) = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };

    generate_monthly_report(pool, year, month)
}

pub fn generate_all_time_report(pool: &DbPool) -> Result<Report> {
    let start_date = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
    let end_date = Utc::now();

    generate_report(pool, start_date, end_date, "All Time".to_string())
}

fn generate_report(
    pool: &DbPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    period: String,
) -> Result<Report> {
    let top_artists = crate::db::get_top_artists(pool, 50, Some(start_date), Some(end_date))?;
    let top_tracks = crate::db::get_top_tracks(pool, 50, Some(start_date), Some(end_date))?;
    let top_albums = crate::db::get_top_albums(pool, 50, Some(start_date), Some(end_date))?;

    let total_scrobbles: i64 = top_artists.iter().map(|(_, count)| count).sum();

    Ok(Report {
        period,
        start_date,
        end_date,
        total_scrobbles,
        top_artists,
        top_tracks,
        top_albums,
    })
}
