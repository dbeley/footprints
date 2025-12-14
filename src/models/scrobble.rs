use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrobble {
    pub id: Option<i64>,
    pub artist: String,
    pub album: Option<String>,
    pub track: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,            // "lastfm" or "listenbrainz"
    pub source_id: Option<String>, // Unique ID from source API to prevent duplicates
}

impl Scrobble {
    pub fn new(artist: String, track: String, timestamp: DateTime<Utc>, source: String) -> Self {
        Self {
            id: None,
            artist,
            album: None,
            track,
            timestamp,
            source,
            source_id: None,
        }
    }

    pub fn with_album(mut self, album: String) -> Self {
        self.album = Some(album);
        self
    }

    pub fn with_source_id(mut self, source_id: String) -> Self {
        self.source_id = Some(source_id);
        self
    }
}
