use super::*;
use crate::models::Scrobble;
use tempfile::NamedTempFile;

fn setup_test_db() -> (DbPool, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let pool = create_pool(temp_file.path().to_str().unwrap()).unwrap();
    init_database(&pool).unwrap();
    (pool, temp_file)
}

#[test]
fn test_database_init() {
    let (pool, _temp_file) = setup_test_db();
    let count = get_scrobbles_count(&pool).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_insert_scrobble() {
    let (pool, _temp_file) = setup_test_db();

    let scrobble = Scrobble::new(
        "Test Artist".to_string(),
        "Test Track".to_string(),
        chrono::Utc::now(),
        "test".to_string(),
    );

    let result = insert_scrobble(&pool, &scrobble);
    assert!(result.is_ok());

    let count = get_scrobbles_count(&pool).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_get_scrobbles() {
    let (pool, _temp_file) = setup_test_db();

    for i in 0..5 {
        let scrobble = Scrobble::new(
            format!("Artist {}", i),
            format!("Track {}", i),
            chrono::Utc::now(),
            "test".to_string(),
        );
        insert_scrobble(&pool, &scrobble).unwrap();
    }

    let scrobbles = get_scrobbles(&pool, Some(10), Some(0)).unwrap();
    assert_eq!(scrobbles.len(), 5);
}

#[test]
fn test_duplicate_prevention() {
    let (pool, _temp_file) = setup_test_db();

    let timestamp = chrono::Utc::now();
    let scrobble = Scrobble::new(
        "Test Artist".to_string(),
        "Test Track".to_string(),
        timestamp,
        "test".to_string(),
    );

    insert_scrobble(&pool, &scrobble).unwrap();
    insert_scrobble(&pool, &scrobble).unwrap();

    let count = get_scrobbles_count(&pool).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_top_artists() {
    let (pool, _temp_file) = setup_test_db();

    use std::thread;
    use std::time::Duration;

    for i in 0..3 {
        let scrobble = Scrobble::new(
            "Popular Artist".to_string(),
            format!("Track {}", i),
            chrono::Utc::now(),
            "test".to_string(),
        );
        insert_scrobble(&pool, &scrobble).unwrap();
        thread::sleep(Duration::from_millis(10));
    }

    let scrobble = Scrobble::new(
        "Less Popular Artist".to_string(),
        "Track 2".to_string(),
        chrono::Utc::now(),
        "test".to_string(),
    );
    insert_scrobble(&pool, &scrobble).unwrap();

    let top_artists = get_top_artists(&pool, 10, None, None).unwrap();
    assert_eq!(top_artists.len(), 2);

    let popular = top_artists
        .iter()
        .find(|(name, _)| name == "Popular Artist")
        .unwrap();
    let less_popular = top_artists
        .iter()
        .find(|(name, _)| name == "Less Popular Artist")
        .unwrap();

    assert_eq!(popular.1, 3);
    assert_eq!(less_popular.1, 1);
}

#[test]
fn test_sync_config_crud() {
    use crate::models::SyncConfig;

    let (pool, _temp_file) = setup_test_db();

    // Create a sync config
    let config = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 60)
        .with_api_key("test_api_key".to_string())
        .with_enabled(true);

    let result = insert_sync_config(&pool, &config);
    assert!(result.is_ok());

    // Get all sync configs
    let configs = get_all_sync_configs(&pool).unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].source, "lastfm");
    assert_eq!(configs[0].username, "testuser");
    assert_eq!(configs[0].sync_interval_minutes, 60);

    // Get enabled sync configs
    let enabled_configs = get_enabled_sync_configs(&pool).unwrap();
    assert_eq!(enabled_configs.len(), 1);

    // Update sync timestamp
    let config_id = configs[0].id.unwrap();
    let new_timestamp = chrono::Utc::now();
    update_sync_timestamp(&pool, config_id, new_timestamp).unwrap();

    let updated_config = get_sync_config(&pool, config_id).unwrap().unwrap();
    assert!(updated_config.last_sync_timestamp.is_some());

    // Delete sync config
    delete_sync_config(&pool, config_id).unwrap();
    let configs = get_all_sync_configs(&pool).unwrap();
    assert_eq!(configs.len(), 0);
}

#[test]
fn test_sync_config_unique_constraint() {
    use crate::models::SyncConfig;

    let (pool, _temp_file) = setup_test_db();

    // Create a sync config
    let config1 = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 60)
        .with_api_key("test_api_key_1".to_string());

    insert_sync_config(&pool, &config1).unwrap();

    // Try to create another config with same source and username
    let config2 = SyncConfig::new("lastfm".to_string(), "testuser".to_string(), 120)
        .with_api_key("test_api_key_2".to_string());

    // This should succeed but update the existing record
    insert_sync_config(&pool, &config2).unwrap();

    let configs = get_all_sync_configs(&pool).unwrap();
    assert_eq!(configs.len(), 1); // Should still be only 1 config
    assert_eq!(configs[0].sync_interval_minutes, 120); // Should be updated
}

#[test]
fn test_sync_config_disabled() {
    use crate::models::SyncConfig;

    let (pool, _temp_file) = setup_test_db();

    // Create enabled config
    let config1 = SyncConfig::new("lastfm".to_string(), "user1".to_string(), 60);
    insert_sync_config(&pool, &config1).unwrap();

    // Create disabled config
    let config2 =
        SyncConfig::new("listenbrainz".to_string(), "user2".to_string(), 60).with_enabled(false);
    insert_sync_config(&pool, &config2).unwrap();

    // Get all configs
    let all_configs = get_all_sync_configs(&pool).unwrap();
    assert_eq!(all_configs.len(), 2);

    // Get only enabled configs
    let enabled_configs = get_enabled_sync_configs(&pool).unwrap();
    assert_eq!(enabled_configs.len(), 1);
    assert_eq!(enabled_configs[0].source, "lastfm");
}
