#[cfg(test)]
mod tests {
    use crate::db::*;
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

        let scrobble = crate::models::Scrobble::new(
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

        // Insert multiple scrobbles
        for i in 0..5 {
            let scrobble = crate::models::Scrobble::new(
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
        let scrobble = crate::models::Scrobble::new(
            "Test Artist".to_string(),
            "Test Track".to_string(),
            timestamp,
            "test".to_string(),
        );

        // Insert the same scrobble twice
        insert_scrobble(&pool, &scrobble).unwrap();
        insert_scrobble(&pool, &scrobble).unwrap();

        // Should still only have 1 scrobble due to UNIQUE constraint
        let count = get_scrobbles_count(&pool).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_top_artists() {
        let (pool, _temp_file) = setup_test_db();

        // Insert scrobbles for different artists at different times
        use std::thread;
        use std::time::Duration;

        for i in 0..3 {
            let scrobble = crate::models::Scrobble::new(
                "Popular Artist".to_string(),
                format!("Track {}", i),
                chrono::Utc::now(),
                "test".to_string(),
            );
            insert_scrobble(&pool, &scrobble).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        let scrobble = crate::models::Scrobble::new(
            "Less Popular Artist".to_string(),
            "Track 2".to_string(),
            chrono::Utc::now(),
            "test".to_string(),
        );
        insert_scrobble(&pool, &scrobble).unwrap();

        let top_artists = get_top_artists(&pool, 10, None, None).unwrap();
        assert_eq!(top_artists.len(), 2);

        // Find the artists in the list (order might vary)
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
}
