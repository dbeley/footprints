#[cfg(test)]
mod tests {
    use crate::reports::{
        generate_all_time_report, generate_monthly_report, generate_yearly_report,
    };
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (crate::db::DbPool, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let pool = crate::db::create_pool(temp_file.path().to_str().unwrap()).unwrap();
        crate::db::init_database(&pool).unwrap();
        (pool, temp_file)
    }

    #[test]
    fn test_yearly_report_generation() {
        let (pool, _temp_file) = setup_test_db();
        let result = generate_yearly_report(&pool, 2024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yearly_report_invalid_year() {
        let (pool, _temp_file) = setup_test_db();

        // Test year too old
        let result = generate_yearly_report(&pool, 1900);
        assert!(result.is_err());

        // Test year too far in future
        let result = generate_yearly_report(&pool, 2200);
        assert!(result.is_err());
    }

    #[test]
    fn test_monthly_report_invalid_month() {
        let (pool, _temp_file) = setup_test_db();

        // Test invalid month 0
        let result = generate_monthly_report(&pool, 2024, 0);
        assert!(result.is_err());

        // Test invalid month 13
        let result = generate_monthly_report(&pool, 2024, 13);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_time_report() {
        let (pool, _temp_file) = setup_test_db();
        let result = generate_all_time_report(&pool);
        assert!(result.is_ok());
    }
}
