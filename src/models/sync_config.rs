use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub id: Option<i64>,
    pub source: String, // "lastfm" or "listenbrainz"
    pub username: String,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub sync_interval_minutes: i32,
    pub last_sync_timestamp: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SyncConfig {
    pub fn new(source: String, username: String, sync_interval_minutes: i32) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            source,
            username,
            api_key: None,
            token: None,
            sync_interval_minutes,
            last_sync_timestamp: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_new() {
        let config = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 60);

        assert_eq!(config.source, "lastfm");
        assert_eq!(config.username, "testuser");
        assert_eq!(config.sync_interval_minutes, 60);
        assert!(config.enabled);
        assert!(config.api_key.is_none());
        assert!(config.token.is_none());
        assert!(config.id.is_none());
        assert!(config.last_sync_timestamp.is_none());
    }

    #[test]
    fn test_sync_config_with_api_key() {
        let config = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 60)
            .with_api_key("test_api_key".to_string());

        assert_eq!(config.api_key, Some("test_api_key".to_string()));
    }

    #[test]
    fn test_sync_config_with_token() {
        let config = SyncConfig::new("listenbrainz".to_string(), "testuser".to_string(), 60)
            .with_token("test_token".to_string());

        assert_eq!(config.token, Some("test_token".to_string()));
    }

    #[test]
    fn test_sync_config_with_enabled() {
        let config = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 60)
            .with_enabled(false);

        assert!(!config.enabled);
    }

    #[test]
    fn test_sync_config_builder_chain() {
        let config = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 120)
            .with_api_key("my_key".to_string())
            .with_enabled(false);

        assert_eq!(config.source, "lastfm");
        assert_eq!(config.sync_interval_minutes, 120);
        assert_eq!(config.api_key, Some("my_key".to_string()));
        assert!(!config.enabled);
    }
}
