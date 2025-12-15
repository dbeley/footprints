use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::db::DbPool;
use crate::importers::{LastFmImporter, ListenBrainzImporter};

#[derive(Clone)]
pub struct SyncScheduler {
    pool: DbPool,
    running: Arc<RwLock<bool>>,
}

impl SyncScheduler {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the sync scheduler in the background
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            tracing::warn!("Sync scheduler is already running");
            return;
        }
        *running = true;
        drop(running);

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_loop().await;
        });

        tracing::info!("Sync scheduler started");
    }

    /// Stop the sync scheduler
    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Sync scheduler stopped");
    }

    /// Check if the scheduler is running
    #[allow(dead_code)]
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Main sync loop
    async fn run_loop(&self) {
        let check_interval = Duration::from_secs(60); // Check every minute

        loop {
            // Check if we should stop
            if !*self.running.read().await {
                break;
            }

            // Process all enabled sync configs
            if let Err(e) = self.process_sync_configs().await {
                tracing::error!("Error processing sync configs: {}", e);
            }

            // Wait before next check
            tokio::time::sleep(check_interval).await;
        }
    }

    /// Process all enabled sync configurations
    async fn process_sync_configs(&self) -> Result<()> {
        let configs = crate::db::get_enabled_sync_configs(&self.pool)?;

        for config in configs {
            let should_sync = if let Some(last_sync) = config.last_sync_timestamp {
                let elapsed_minutes = (Utc::now() - last_sync).num_minutes();
                elapsed_minutes >= config.sync_interval_minutes as i64
            } else {
                // Never synced before, sync now
                true
            };

            if should_sync {
                if let Some(config_id) = config.id {
                    tracing::info!(
                        "Starting sync for {} user {}",
                        config.source,
                        config.username
                    );

                    match self.sync_config(&config).await {
                        Ok(count) => {
                            tracing::info!(
                                "Synced {} new scrobbles for {} user {}",
                                count,
                                config.source,
                                config.username
                            );
                            // Update last sync timestamp
                            if let Err(e) =
                                crate::db::update_sync_timestamp(&self.pool, config_id, Utc::now())
                            {
                                tracing::error!(
                                    "Failed to update sync timestamp for config {}: {}",
                                    config_id,
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to sync {} user {}: {}",
                                config.source,
                                config.username,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Sync a specific configuration
    async fn sync_config(&self, config: &crate::models::SyncConfig) -> Result<usize> {
        let since = config
            .last_sync_timestamp
            .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24)); // Default to last 24 hours for first sync

        match config.source.as_str() {
            "lastfm" => {
                if let Some(api_key) = &config.api_key {
                    let importer = LastFmImporter::new(api_key.clone(), config.username.clone());
                    importer.import_since(&self.pool, since).await
                } else {
                    Err(anyhow::anyhow!("API key required for Last.fm sync"))
                }
            }
            "listenbrainz" => {
                let importer =
                    ListenBrainzImporter::new(config.username.clone(), config.token.clone());
                importer.import_since(&self.pool, since).await
            }
            _ => Err(anyhow::anyhow!("Unknown source: {}", config.source)),
        }
    }

    /// Manually trigger a sync for a specific configuration
    pub async fn trigger_sync(&self, config_id: i64) -> Result<usize> {
        let config = crate::db::get_sync_config(&self.pool, config_id)?
            .ok_or_else(|| anyhow::anyhow!("Sync config not found"))?;

        if !config.enabled {
            return Err(anyhow::anyhow!("Sync config is disabled"));
        }

        let count = self.sync_config(&config).await?;

        // Update last sync timestamp
        crate::db::update_sync_timestamp(&self.pool, config_id, Utc::now())?;

        Ok(count)
    }
}
