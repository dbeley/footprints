mod cache;
mod lastfm;
mod musicbrainz;
mod types;

use anyhow::Result;

use crate::db::DbPool;

use cache::ImageCache;
use lastfm::LastFmImageClient;
use musicbrainz::MusicBrainzImageClient;
pub use types::{EntityType, ImageRequest};

pub struct ImageService {
    cache: ImageCache,
    lastfm_client: LastFmImageClient,
    musicbrainz_client: MusicBrainzImageClient,
}

impl ImageService {
    pub fn new(pool: DbPool, lastfm_api_key: String) -> Self {
        Self {
            cache: ImageCache::new(pool),
            lastfm_client: LastFmImageClient::new(lastfm_api_key),
            musicbrainz_client: MusicBrainzImageClient::new(),
        }
    }

    pub async fn get_image_url(&self, request: ImageRequest) -> Result<Option<String>> {
        // 1. Check cache first
        if let Some(cached) = self.cache.get(&request)? {
            // Update last_accessed timestamp for LRU
            let _ = self.cache.update_access_time(&request);
            return Ok(cached.url);
        }

        // 2. Fetch from appropriate source
        let url = match request.entity_type {
            EntityType::Artist => {
                // Last.fm artist images are broken, use MusicBrainz only
                self.musicbrainz_client
                    .fetch_artist_image(&request.artist_name)
                    .await
                    .ok()
                    .flatten()
            }
            EntityType::Album => {
                if let Some(album_name) = &request.album_name {
                    // Try Last.fm first for albums (still works)
                    let mut url = self
                        .lastfm_client
                        .fetch_album_image(&request.artist_name, album_name, request.size)
                        .await
                        .ok()
                        .flatten();

                    // Fallback to MusicBrainz if Last.fm fails
                    if url.is_none() {
                        url = self
                            .musicbrainz_client
                            .fetch_album_image(&request.artist_name, album_name)
                            .await
                            .ok()
                            .flatten();
                    }
                    url
                } else {
                    None
                }
            }
            EntityType::Track => {
                if let Some(track_name) = &request.track_name {
                    self.lastfm_client
                        .fetch_track_image(&request.artist_name, track_name, request.size)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                }
            }
        };

        // 3. Cache the result (even if None, to avoid repeated lookups)
        self.cache.set(&request, url.clone())?;

        Ok(url)
    }
}
