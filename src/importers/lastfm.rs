use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::models::Scrobble;

#[derive(Debug, Deserialize, Serialize)]
struct LastFmResponse {
    recenttracks: RecentTracks,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecentTracks {
    track: Vec<Track>,
    #[serde(rename = "@attr")]
    attr: Option<Attributes>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Attributes {
    total: String,
    page: String,
    #[serde(rename = "perPage")]
    per_page: String,
    #[serde(rename = "totalPages")]
    total_pages: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Track {
    artist: Artist,
    album: Option<Album>,
    name: String,
    date: Option<DateInfo>,
    #[serde(rename = "@attr")]
    attr: Option<TrackAttr>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Artist {
    #[serde(rename = "#text")]
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Album {
    #[serde(rename = "#text")]
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DateInfo {
    uts: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TrackAttr {
    nowplaying: Option<String>,
}

pub struct LastFmImporter {
    api_key: String,
    username: String,
    client: reqwest::Client,
}

impl LastFmImporter {
    pub fn new(api_key: String, username: String) -> Self {
        Self {
            api_key,
            username,
            client: reqwest::Client::new(),
        }
    }

    pub async fn import_all(&self, pool: &DbPool) -> Result<usize> {
        let mut imported_count = 0;
        let mut page = 1;
        let per_page = 200;

        loop {
            tracing::info!("Fetching Last.fm page {}", page);

            let url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit={}&page={}",
                self.username, self.api_key, per_page, page
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to fetch from Last.fm")?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Last.fm API returned error: {}",
                    response.status()
                ));
            }

            let data: LastFmResponse = response
                .json()
                .await
                .context("Failed to parse Last.fm response")?;

            if data.recenttracks.track.is_empty() {
                break;
            }

            for track in &data.recenttracks.track {
                // Skip currently playing tracks
                if track
                    .attr
                    .as_ref()
                    .and_then(|a| a.nowplaying.as_ref())
                    .is_some()
                {
                    continue;
                }

                if let Some(date_info) = &track.date {
                    if let Ok(timestamp) = date_info.uts.parse::<i64>() {
                        let mut scrobble = Scrobble::new(
                            track.artist.text.clone(),
                            track.name.clone(),
                            DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now),
                            "lastfm".to_string(),
                        );

                        if let Some(album) = &track.album {
                            if !album.text.is_empty() {
                                scrobble = scrobble.with_album(album.text.clone());
                            }
                        }

                        // Use timestamp as unique identifier
                        scrobble = scrobble.with_source_id(format!("lastfm_{}", timestamp));

                        if crate::db::insert_scrobble(pool, &scrobble).is_ok() {
                            imported_count += 1;
                        }
                    }
                }
            }

            // Check if we have more pages
            if let Some(attr) = &data.recenttracks.attr {
                if let (Ok(current_page), Ok(total_pages)) =
                    (attr.page.parse::<i32>(), attr.total_pages.parse::<i32>())
                {
                    if current_page >= total_pages {
                        break;
                    }
                }
            } else {
                break;
            }

            page += 1;
        }

        tracing::info!("Imported {} scrobbles from Last.fm", imported_count);
        Ok(imported_count)
    }

    /// Import scrobbles since a specific timestamp (for incremental sync)
    pub async fn import_since(&self, pool: &DbPool, since: DateTime<Utc>) -> Result<usize> {
        let mut imported_count = 0;
        let mut page = 1;
        let per_page = 200;
        let since_timestamp = since.timestamp();

        loop {
            tracing::info!("Fetching Last.fm page {} (since {})", page, since);

            let url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit={}&page={}&from={}",
                self.username, self.api_key, per_page, page, since_timestamp
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to fetch from Last.fm")?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Last.fm API returned error: {}",
                    response.status()
                ));
            }

            let data: LastFmResponse = response
                .json()
                .await
                .context("Failed to parse Last.fm response")?;

            if data.recenttracks.track.is_empty() {
                break;
            }

            for track in &data.recenttracks.track {
                // Skip currently playing tracks
                if track
                    .attr
                    .as_ref()
                    .and_then(|a| a.nowplaying.as_ref())
                    .is_some()
                {
                    continue;
                }

                if let Some(date_info) = &track.date {
                    if let Ok(timestamp) = date_info.uts.parse::<i64>() {
                        // Skip tracks older than our "since" timestamp
                        if timestamp <= since_timestamp {
                            continue;
                        }

                        let mut scrobble = Scrobble::new(
                            track.artist.text.clone(),
                            track.name.clone(),
                            DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now),
                            "lastfm".to_string(),
                        );

                        if let Some(album) = &track.album {
                            if !album.text.is_empty() {
                                scrobble = scrobble.with_album(album.text.clone());
                            }
                        }

                        // Use timestamp as unique identifier
                        scrobble = scrobble.with_source_id(format!("lastfm_{}", timestamp));

                        if crate::db::insert_scrobble(pool, &scrobble).is_ok() {
                            imported_count += 1;
                        }
                    }
                }
            }

            // Check if we have more pages
            if let Some(attr) = &data.recenttracks.attr {
                if let (Ok(current_page), Ok(total_pages)) =
                    (attr.page.parse::<i32>(), attr.total_pages.parse::<i32>())
                {
                    if current_page >= total_pages {
                        break;
                    }
                }
            } else {
                break;
            }

            page += 1;
        }

        tracing::info!(
            "Imported {} new scrobbles from Last.fm since {}",
            imported_count,
            since
        );
        Ok(imported_count)
    }
}
