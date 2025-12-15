use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::models::Scrobble;

#[derive(Debug, Deserialize, Serialize)]
struct ListenBrainzResponse {
    payload: Payload,
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    count: i32,
    listens: Vec<Listen>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Listen {
    listened_at: i64,
    track_metadata: TrackMetadata,
    recording_msid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TrackMetadata {
    artist_name: String,
    track_name: String,
    release_name: Option<String>,
}

pub struct ListenBrainzImporter {
    username: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl ListenBrainzImporter {
    pub fn new(username: String, token: Option<String>) -> Self {
        Self {
            username,
            token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn import_all(&self, pool: &DbPool) -> Result<usize> {
        let mut imported_count = 0;
        let mut max_ts: Option<i64> = None;
        let count = 100;

        loop {
            tracing::info!(
                "Fetching ListenBrainz listens{}",
                max_ts
                    .map(|ts| format!(" before timestamp {}", ts))
                    .unwrap_or_default()
            );

            let mut url = format!(
                "https://api.listenbrainz.org/1/user/{}/listens?count={}",
                self.username, count
            );

            if let Some(ts) = max_ts {
                url.push_str(&format!("&max_ts={}", ts));
            }

            let mut request = self.client.get(&url);

            if let Some(token) = &self.token {
                request = request.header("Authorization", format!("Token {}", token));
            }

            let response = request
                .send()
                .await
                .context("Failed to fetch from ListenBrainz")?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "ListenBrainz API returned error: {}",
                    response.status()
                ));
            }

            let data: ListenBrainzResponse = response
                .json()
                .await
                .context("Failed to parse ListenBrainz response")?;

            if data.payload.listens.is_empty() {
                break;
            }

            for listen in &data.payload.listens {
                let mut scrobble = Scrobble::new(
                    listen.track_metadata.artist_name.clone(),
                    listen.track_metadata.track_name.clone(),
                    DateTime::from_timestamp(listen.listened_at, 0).unwrap_or_else(Utc::now),
                    "listenbrainz".to_string(),
                );

                if let Some(album) = &listen.track_metadata.release_name {
                    if !album.is_empty() {
                        scrobble = scrobble.with_album(album.clone());
                    }
                }

                // Use recording_msid or timestamp as unique identifier
                let source_id = if let Some(msid) = &listen.recording_msid {
                    format!("listenbrainz_{}", msid)
                } else {
                    format!("listenbrainz_{}", listen.listened_at)
                };
                scrobble = scrobble.with_source_id(source_id);

                if crate::db::insert_scrobble(pool, &scrobble).is_ok() {
                    imported_count += 1;
                }

                // Update max_ts for pagination
                max_ts = Some(listen.listened_at);
            }

            // If we got fewer results than requested, we've reached the end
            if data.payload.listens.len() < count as usize {
                break;
            }
        }

        tracing::info!("Imported {} scrobbles from ListenBrainz", imported_count);
        Ok(imported_count)
    }

    /// Import scrobbles since a specific timestamp (for incremental sync)
    pub async fn import_since(&self, pool: &DbPool, since: DateTime<Utc>) -> Result<usize> {
        let mut imported_count = 0;
        let mut max_ts: Option<i64> = None;
        let count = 100;
        let since_timestamp = since.timestamp();

        loop {
            tracing::info!(
                "Fetching ListenBrainz listens since {}{}",
                since,
                max_ts
                    .map(|ts| format!(" (before timestamp {})", ts))
                    .unwrap_or_default()
            );

            let mut url = format!(
                "https://api.listenbrainz.org/1/user/{}/listens?count={}&min_ts={}",
                self.username, count, since_timestamp
            );

            if let Some(ts) = max_ts {
                url.push_str(&format!("&max_ts={}", ts));
            }

            let mut request = self.client.get(&url);

            if let Some(token) = &self.token {
                request = request.header("Authorization", format!("Token {}", token));
            }

            let response = request
                .send()
                .await
                .context("Failed to fetch from ListenBrainz")?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "ListenBrainz API returned error: {}",
                    response.status()
                ));
            }

            let data: ListenBrainzResponse = response
                .json()
                .await
                .context("Failed to parse ListenBrainz response")?;

            if data.payload.listens.is_empty() {
                break;
            }

            for listen in &data.payload.listens {
                // Skip listens at or before our "since" timestamp to avoid duplicates
                // Using <= ensures we don't re-import the exact timestamp from last sync
                if listen.listened_at <= since_timestamp {
                    continue;
                }

                let mut scrobble = Scrobble::new(
                    listen.track_metadata.artist_name.clone(),
                    listen.track_metadata.track_name.clone(),
                    DateTime::from_timestamp(listen.listened_at, 0).unwrap_or_else(Utc::now),
                    "listenbrainz".to_string(),
                );

                if let Some(album) = &listen.track_metadata.release_name {
                    if !album.is_empty() {
                        scrobble = scrobble.with_album(album.clone());
                    }
                }

                // Use recording_msid or timestamp as unique identifier
                let source_id = if let Some(msid) = &listen.recording_msid {
                    format!("listenbrainz_{}", msid)
                } else {
                    format!("listenbrainz_{}", listen.listened_at)
                };
                scrobble = scrobble.with_source_id(source_id);

                if crate::db::insert_scrobble(pool, &scrobble).is_ok() {
                    imported_count += 1;
                }

                // Update max_ts for pagination
                max_ts = Some(listen.listened_at);
            }

            // If we got fewer results than requested, we've reached the end
            if data.payload.listens.len() < count as usize {
                break;
            }
        }

        tracing::info!(
            "Imported {} new scrobbles from ListenBrainz since {}",
            imported_count,
            since
        );
        Ok(imported_count)
    }
}
