use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

use crate::db::DbPool;

use super::types::{ImageMetadata, ImageRequest};

pub struct ImageCache {
    pool: DbPool,
}

impl ImageCache {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get(&self, request: &ImageRequest) -> Result<Option<ImageMetadata>> {
        let conn = self.pool.get()?;

        let result = conn.query_row(
            "SELECT image_url, fetched_at FROM image_cache
             WHERE entity_type = ?1 AND entity_name = ?2 AND
                   ((?3 IS NULL AND entity_album IS NULL) OR entity_album = ?3)
                   AND image_size = ?4",
            params![
                request.entity_type.as_str(),
                request.artist_name,
                request.album_name,
                request.size.as_str()
            ],
            |row| {
                Ok(ImageMetadata {
                    url: row.get(0)?,
                    fetched_at: row.get(1)?,
                })
            },
        );

        match result {
            Ok(metadata) => Ok(Some(metadata)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(&self, request: &ImageRequest, url: Option<String>) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO image_cache
             (entity_type, entity_name, entity_album, image_url, image_size, fetched_at, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(entity_type, entity_name, entity_album, image_size)
             DO UPDATE SET image_url = ?4, fetched_at = ?6, last_accessed = ?7",
            params![
                request.entity_type.as_str(),
                request.artist_name,
                request.album_name,
                url,
                request.size.as_str(),
                now,
                now
            ],
        )?;

        Ok(())
    }

    pub fn update_access_time(&self, request: &ImageRequest) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE image_cache SET last_accessed = ?1
             WHERE entity_type = ?2 AND entity_name = ?3 AND
                   ((?4 IS NULL AND entity_album IS NULL) OR entity_album = ?4)
                   AND image_size = ?5",
            params![
                now,
                request.entity_type.as_str(),
                request.artist_name,
                request.album_name,
                request.size.as_str()
            ],
        )?;

        Ok(())
    }
}
