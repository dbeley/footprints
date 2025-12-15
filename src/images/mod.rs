mod cache;
mod lastfm;
mod types;

use anyhow::Result;

use crate::db::DbPool;

pub use types::{EntityType, ImageRequest};
use cache::ImageCache;
use lastfm::LastFmImageClient;

pub struct ImageService {
    cache: ImageCache,
    lastfm_client: LastFmImageClient,
}

impl ImageService {
    pub fn new(pool: DbPool, lastfm_api_key: String) -> Self {
        Self {
            cache: ImageCache::new(pool),
            lastfm_client: LastFmImageClient::new(lastfm_api_key),
        }
    }

    pub async fn get_image_url(&self, request: ImageRequest) -> Result<Option<String>> {
        // 1. Check cache first
        if let Some(cached) = self.cache.get(&request)? {
            // Update last_accessed timestamp for LRU
            let _ = self.cache.update_access_time(&request);
            return Ok(cached.url);
        }

        // 2. Fetch from Last.fm
        let url = match request.entity_type {
            EntityType::Artist => {
                self.lastfm_client
                    .fetch_artist_image(&request.artist_name, request.size)
                    .await
                    .ok()
                    .flatten()
            }
            EntityType::Album => {
                if let Some(album_name) = &request.album_name {
                    self.lastfm_client
                        .fetch_album_image(&request.artist_name, album_name, request.size)
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
