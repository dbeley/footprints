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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrobble_new() {
        let timestamp = DateTime::from_timestamp(1640995200, 0).unwrap();
        let scrobble = Scrobble::new(
            "Test Artist".to_string(),
            "Test Track".to_string(),
            timestamp,
            "test".to_string(),
        );

        assert_eq!(scrobble.artist, "Test Artist");
        assert_eq!(scrobble.track, "Test Track");
        assert_eq!(scrobble.timestamp, timestamp);
        assert_eq!(scrobble.source, "test");
        assert!(scrobble.album.is_none());
        assert!(scrobble.source_id.is_none());
        assert!(scrobble.id.is_none());
    }

    #[test]
    fn test_scrobble_with_album() {
        let timestamp = DateTime::from_timestamp(1640995200, 0).unwrap();
        let scrobble = Scrobble::new(
            "Test Artist".to_string(),
            "Test Track".to_string(),
            timestamp,
            "test".to_string(),
        )
        .with_album("Test Album".to_string());

        assert_eq!(scrobble.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_scrobble_with_source_id() {
        let timestamp = DateTime::from_timestamp(1640995200, 0).unwrap();
        let scrobble = Scrobble::new(
            "Test Artist".to_string(),
            "Test Track".to_string(),
            timestamp,
            "test".to_string(),
        )
        .with_source_id("12345".to_string());

        assert_eq!(scrobble.source_id, Some("12345".to_string()));
    }

    #[test]
    fn test_scrobble_builder_chain() {
        let timestamp = DateTime::from_timestamp(1640995200, 0).unwrap();
        let scrobble = Scrobble::new(
            "Test Artist".to_string(),
            "Test Track".to_string(),
            timestamp,
            "lastfm".to_string(),
        )
        .with_album("Test Album".to_string())
        .with_source_id("67890".to_string());

        assert_eq!(scrobble.album, Some("Test Album".to_string()));
        assert_eq!(scrobble.source_id, Some("67890".to_string()));
        assert_eq!(scrobble.source, "lastfm");
    }
}
