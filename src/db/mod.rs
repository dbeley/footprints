use anyhow::Result;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use crate::models::Scrobble;

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
            Ok(Scrobble {
                id: Some(row.get(0)?),
                artist: row.get(1)?,
                album: row.get(2)?,
                track: row.get(3)?,
                timestamp: DateTime::from_timestamp(row.get(4)?, 0)
                    .unwrap_or_else(|| Utc::now()),
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

pub fn get_top_artists(pool: &DbPool, limit: i64, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> Result<Vec<(String, i64)>> {
    let conn = pool.get()?;
    
    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as count FROM scrobbles 
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist ORDER BY count DESC LIMIT ?3"
        )?;
        let artists_iter = stmt.query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let artists: Vec<(String, i64)> = artists_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(artists)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as count FROM scrobbles 
             GROUP BY artist ORDER BY count DESC LIMIT ?1"
        )?;
        let artists_iter = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let artists: Vec<(String, i64)> = artists_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(artists)
    }
}

pub fn get_top_tracks(pool: &DbPool, limit: i64, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> Result<Vec<(String, String, i64)>> {
    let conn = pool.get()?;
    
    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, track, COUNT(*) as count FROM scrobbles 
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist, track ORDER BY count DESC LIMIT ?3"
        )?;
        let tracks_iter = stmt.query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let tracks: Vec<(String, String, i64)> = tracks_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(tracks)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, track, COUNT(*) as count FROM scrobbles 
             GROUP BY artist, track ORDER BY count DESC LIMIT ?1"
        )?;
        let tracks_iter = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let tracks: Vec<(String, String, i64)> = tracks_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(tracks)
    }
}

pub fn get_top_albums(pool: &DbPool, limit: i64, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> Result<Vec<(String, String, i64)>> {
    let conn = pool.get()?;
    
    if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut stmt = conn.prepare(
            "SELECT artist, album, COUNT(*) as count FROM scrobbles 
             WHERE album IS NOT NULL AND timestamp >= ?1 AND timestamp <= ?2
             GROUP BY artist, album ORDER BY count DESC LIMIT ?3"
        )?;
        let albums_iter = stmt.query_map(params![start.timestamp(), end.timestamp(), limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let albums: Vec<(String, String, i64)> = albums_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(albums)
    } else {
        let mut stmt = conn.prepare(
            "SELECT artist, album, COUNT(*) as count FROM scrobbles 
             WHERE album IS NOT NULL
             GROUP BY artist, album ORDER BY count DESC LIMIT ?1"
        )?;
        let albums_iter = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let albums: Vec<(String, String, i64)> = albums_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(albums)
    }
}
