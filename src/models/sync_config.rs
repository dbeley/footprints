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
