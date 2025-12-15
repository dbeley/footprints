use anyhow::Result;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use crate::models::{Scrobble, SyncConfig};

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn create_pool(db_path: &str) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    Ok(pool)
}

pub fn init_database(pool: &DbPool) -> Result<()> {
    let conn = pool.get()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scrobbles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            artist TEXT NOT NULL,
            album TEXT,
            track TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            source TEXT NOT NULL,
            source_id TEXT,
            UNIQUE(artist, track, timestamp, source)
        )",
        [],
    )?;

    // Create indices for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timestamp ON scrobbles(timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_artist ON scrobbles(artist)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_source_id ON scrobbles(source_id)",
        [],
    )?;

    // Create image cache table for storing Last.fm artist/album images
    conn.execute(
        "CREATE TABLE IF NOT EXISTS image_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            entity_name TEXT NOT NULL,
            entity_album TEXT,
            image_url TEXT,
            image_size TEXT NOT NULL,
            fetched_at INTEGER NOT NULL,
            last_accessed INTEGER NOT NULL,
            UNIQUE(entity_type, entity_name, entity_album, image_size)
        )",
        [],
    )?;

    // Create indices for image cache
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_image_cache_lookup
         ON image_cache(entity_type, entity_name, entity_album)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_image_cache_lru
         ON image_cache(last_accessed)",
        [],
    )?;

    // Create sync_configs table for automatic sync configuration
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            username TEXT NOT NULL,
            api_key TEXT,
            token TEXT,
            sync_interval_minutes INTEGER NOT NULL DEFAULT 60,
            last_sync_timestamp INTEGER,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(source, username)
        )",
        [],
    )?;

    // Create index for enabled sync configs
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sync_configs_enabled
         ON sync_configs(enabled, source)",
        [],
    )?;

    Ok(())
}

pub fn insert_scrobble(pool: &DbPool, scrobble: &Scrobble) -> Result<i64> {
    let conn = pool.get()?;

    conn.execute(
        "INSERT OR IGNORE INTO scrobbles (artist, album, track, timestamp, source, source_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            scrobble.artist,
            scrobble.album,
            scrobble.track,
            scrobble.timestamp.timestamp(),
            scrobble.source,
            scrobble.source_id,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn insert_scrobbles_batch(pool: &DbPool, scrobbles: &[Scrobble]) -> Result<usize> {
    if scrobbles.is_empty() {
        return Ok(0);
    }

    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let mut inserted = 0;
    for scrobble in scrobbles {
        let changes = tx.execute(
            "INSERT OR IGNORE INTO scrobbles (artist, album, track, timestamp, source, source_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                scrobble.artist,
                scrobble.album,
                scrobble.track,
                scrobble.timestamp.timestamp(),
                scrobble.source,
                scrobble.source_id,
            ],
        )?;
        inserted += changes;
    }

    tx.commit()?;
    Ok(inserted)
}

pub fn get_scrobbles(
    pool: &DbPool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Scrobble>> {
    let conn = pool.get()?;
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let mut stmt = conn.prepare(
        "SELECT id, artist, album, track, timestamp, source, source_id
         FROM scrobbles
         ORDER BY timestamp DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let scrobbles = stmt
        .query_map(params![limit, offset], |row| {
            let timestamp_value: i64 = row.get(4)?;
            let timestamp = DateTime::from_timestamp(timestamp_value, 0).unwrap_or_else(|| {
                tracing::warn!(
                    "Invalid timestamp {} in database for scrobble id {:?}, using current time",
                    timestamp_value,
                    row.get::<_, i64>(0).ok()
                );
                Utc::now()
            });
            Ok(Scrobble {
                id: Some(row.get(0)?),
                artist: row.get(1)?,
                album: row.get(2)?,
                track: row.get(3)?,
                timestamp,
                source: row.get(5)?,
                source_id: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scrobbles)
}

pub fn get_scrobbles_count(pool: &DbPool) -> Result<i64> {
    let conn = pool.get()?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM scrobbles", [], |row| row.get(0))?;
    Ok(count)
}

pub fn get_scrobbles_count_in_range(
    pool: &DbPool,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<i64> {
    let conn = pool.get()?;

    if let (Some(start), Some(end)) = (start_date, end_date) {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM scrobbles WHERE timestamp >= ?1 AND timestamp <= ?2",
            params![start.timestamp(), end.timestamp()],
            |row| row.get(0),
        )?;
        Ok(count)
    } else {
        get_scrobbles_count(pool)
    }
}

pub fn get_top_artists(
    pool: &DbPool,
    limit: i64,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<Vec<(String, i64)>> {
    let conn = pool.get()?;

    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as count FROM scrobbles
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist ORDER BY count DESC LIMIT ?3",
        )?;
        let artists_iter = stmt
            .query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        let artists: Vec<(String, i64)> = artists_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(artists)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as count FROM scrobbles
             GROUP BY artist ORDER BY count DESC LIMIT ?1",
        )?;
        let artists_iter = stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let artists: Vec<(String, i64)> = artists_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(artists)
    }
}

pub fn get_top_tracks(
    pool: &DbPool,
    limit: i64,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<Vec<(String, String, i64)>> {
    let conn = pool.get()?;

    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, track, COUNT(*) as count FROM scrobbles
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist, track ORDER BY count DESC LIMIT ?3",
        )?;
        let tracks_iter = stmt
            .query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        let tracks: Vec<(String, String, i64)> = tracks_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(tracks)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, track, COUNT(*) as count FROM scrobbles
             GROUP BY artist, track ORDER BY count DESC LIMIT ?1",
        )?;
        let tracks_iter = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let tracks: Vec<(String, String, i64)> = tracks_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(tracks)
    }
}

pub fn get_top_albums(
    pool: &DbPool,
    limit: i64,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<Vec<(String, String, i64)>> {
    let conn = pool.get()?;

    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, album, COUNT(*) as count FROM scrobbles
             WHERE album IS NOT NULL AND timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist, album ORDER BY count DESC LIMIT ?3",
        )?;
        let albums_iter = stmt
            .query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        let albums: Vec<(String, String, i64)> = albums_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(albums)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, album, COUNT(*) as count FROM scrobbles
             WHERE album IS NOT NULL
             GROUP BY artist, album ORDER BY count DESC LIMIT ?1",
        )?;
        let albums_iter = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let albums: Vec<(String, String, i64)> = albums_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(albums)
    }
}

pub fn get_scrobbles_per_day(
    pool: &DbPool,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<Vec<(String, i64)>> {
    let conn = pool.get()?;

    let (query, params) = if let (Some(start), Some(end)) = (start_date, end_date) {
        (
            "SELECT strftime('%Y-%m-%d', datetime(timestamp, 'unixepoch')) as day, COUNT(*) as count
             FROM scrobbles
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY day
             ORDER BY day ASC",
            params![start.timestamp(), end.timestamp()],
        )
    } else {
        (
            "SELECT strftime('%Y-%m-%d', datetime(timestamp, 'unixepoch')) as day, COUNT(*) as count
             FROM scrobbles
             GROUP BY day
             ORDER BY day ASC",
            params![],
        )
    };

    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map(params, |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_top_album_for_artist(pool: &DbPool, artist: &str) -> Result<Option<String>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT album, COUNT(*) as count FROM scrobbles
         WHERE artist = ?1 AND album IS NOT NULL
         GROUP BY album
         ORDER BY count DESC
         LIMIT 1",
    )?;

    let mut rows = stmt.query(params![artist])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_album_for_track(pool: &DbPool, artist: &str, track: &str) -> Result<Option<String>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT album, COUNT(*) as count FROM scrobbles
         WHERE artist = ?1 AND track = ?2 AND album IS NOT NULL
         GROUP BY album
         ORDER BY count DESC
         LIMIT 1",
    )?;

    let mut rows = stmt.query(params![artist, track])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

// Helper function to safely convert database timestamps
fn parse_timestamp_with_warning(ts: i64, field_name: &str, id: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| {
        tracing::warn!(
            "Invalid {} timestamp {} in sync_config id {}, using current time",
            field_name,
            ts,
            id
        );
        Utc::now()
    })
}

// Sync configuration database operations
pub fn insert_sync_config(pool: &DbPool, config: &SyncConfig) -> Result<i64> {
    let conn = pool.get()?;
    let now = Utc::now().timestamp();

    conn.execute(
        "INSERT INTO sync_configs (source, username, api_key, token, sync_interval_minutes, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(source, username) DO UPDATE SET
            api_key = ?3,
            token = ?4,
            sync_interval_minutes = ?5,
            enabled = ?6,
            updated_at = ?8",
        params![
            config.source,
            config.username,
            config.api_key,
            config.token,
            config.sync_interval_minutes,
            if config.enabled { 1 } else { 0 },
            now,
            now,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn get_sync_config(pool: &DbPool, id: i64) -> Result<Option<SyncConfig>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, source, username, api_key, token, sync_interval_minutes, last_sync_timestamp, enabled, created_at, updated_at
         FROM sync_configs WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let config_id: i64 = row.get(0)?;
        let created_ts: i64 = row.get(8)?;
        let updated_ts: i64 = row.get(9)?;
        let last_sync_ts: Option<i64> = row.get(6)?;

        Ok(Some(SyncConfig {
            id: Some(config_id),
            source: row.get(1)?,
            username: row.get(2)?,
            api_key: row.get(3)?,
            token: row.get(4)?,
            sync_interval_minutes: row.get(5)?,
            last_sync_timestamp: last_sync_ts.and_then(|ts| {
                DateTime::from_timestamp(ts, 0).or_else(|| {
                    tracing::warn!(
                        "Invalid last_sync_timestamp {} in sync_config id {}",
                        ts,
                        config_id
                    );
                    None
                })
            }),
            enabled: row.get::<_, i32>(7)? != 0,
            created_at: parse_timestamp_with_warning(created_ts, "created_at", config_id),
            updated_at: parse_timestamp_with_warning(updated_ts, "updated_at", config_id),
        }))
    } else {
        Ok(None)
    }
}

pub fn get_all_sync_configs(pool: &DbPool) -> Result<Vec<SyncConfig>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, source, username, api_key, token, sync_interval_minutes, last_sync_timestamp, enabled, created_at, updated_at
         FROM sync_configs ORDER BY created_at DESC",
    )?;

    let configs = stmt
        .query_map([], |row| {
            let config_id: i64 = row.get(0)?;
            let created_ts: i64 = row.get(8)?;
            let updated_ts: i64 = row.get(9)?;
            let last_sync_ts: Option<i64> = row.get(6)?;

            Ok(SyncConfig {
                id: Some(config_id),
                source: row.get(1)?,
                username: row.get(2)?,
                api_key: row.get(3)?,
                token: row.get(4)?,
                sync_interval_minutes: row.get(5)?,
                last_sync_timestamp: last_sync_ts.and_then(|ts| {
                    DateTime::from_timestamp(ts, 0).or_else(|| {
                        tracing::warn!(
                            "Invalid last_sync_timestamp {} in sync_config id {}",
                            ts,
                            config_id
                        );
                        None
                    })
                }),
                enabled: row.get::<_, i32>(7)? != 0,
                created_at: parse_timestamp_with_warning(created_ts, "created_at", config_id),
                updated_at: parse_timestamp_with_warning(updated_ts, "updated_at", config_id),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(configs)
}

pub fn get_enabled_sync_configs(pool: &DbPool) -> Result<Vec<SyncConfig>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, source, username, api_key, token, sync_interval_minutes, last_sync_timestamp, enabled, created_at, updated_at
         FROM sync_configs WHERE enabled = 1 ORDER BY created_at DESC",
    )?;

    let configs = stmt
        .query_map([], |row| {
            let config_id: i64 = row.get(0)?;
            let created_ts: i64 = row.get(8)?;
            let updated_ts: i64 = row.get(9)?;
            let last_sync_ts: Option<i64> = row.get(6)?;

            Ok(SyncConfig {
                id: Some(config_id),
                source: row.get(1)?,
                username: row.get(2)?,
                api_key: row.get(3)?,
                token: row.get(4)?,
                sync_interval_minutes: row.get(5)?,
                last_sync_timestamp: last_sync_ts.and_then(|ts| {
                    DateTime::from_timestamp(ts, 0).or_else(|| {
                        tracing::warn!(
                            "Invalid last_sync_timestamp {} in sync_config id {}",
                            ts,
                            config_id
                        );
                        None
                    })
                }),
                enabled: row.get::<_, i32>(7)? != 0,
                created_at: parse_timestamp_with_warning(created_ts, "created_at", config_id),
                updated_at: parse_timestamp_with_warning(updated_ts, "updated_at", config_id),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(configs)
}

pub fn update_sync_timestamp(pool: &DbPool, id: i64, timestamp: DateTime<Utc>) -> Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE sync_configs SET last_sync_timestamp = ?1, updated_at = ?2 WHERE id = ?3",
        params![timestamp.timestamp(), Utc::now().timestamp(), id],
    )?;
    Ok(())
}

pub fn delete_sync_config(pool: &DbPool, id: i64) -> Result<()> {
    let conn = pool.get()?;
    conn.execute("DELETE FROM sync_configs WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests;
