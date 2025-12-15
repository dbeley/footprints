use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Artist,
    Album,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Artist => "artist",
            EntityType::Album => "album",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSize {
    ExtraLarge, // 300x300 from Last.fm
}

impl ImageSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageSize::ExtraLarge => "extralarge",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageRequest {
    pub entity_type: EntityType,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub size: ImageSize,
}

impl ImageRequest {
    pub fn artist(name: String) -> Self {
        Self {
            entity_type: EntityType::Artist,
            artist_name: name,
            album_name: None,
            size: ImageSize::ExtraLarge,
        }
    }

    pub fn album(artist: String, album: String) -> Self {
        Self {
            entity_type: EntityType::Album,
            artist_name: artist,
            album_name: Some(album),
            size: ImageSize::ExtraLarge,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub url: Option<String>,
    pub fetched_at: i64,
}
