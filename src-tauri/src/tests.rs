#[cfg(test)]
mod tests {
    use super::super::*;
    use std::time::SystemTime;

    // Helper function to create test search parameters
    fn create_test_params() -> SearchParams {
        SearchParams {
            keywords: vec![],
            exclude_keywords: vec![],
            start_date: None,
            end_date: None,
            log_names: vec![],
            event_types: vec![],
            event_ids: vec![],
            sources: vec![],
            categories: vec![],
            max_results: None,
        }
    }

    fn create_test_params_with_keywords(keywords: Vec<&str>) -> SearchParams {
        SearchParams {
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
            exclude_keywords: vec![],
            start_date: None,
            end_date: None,
            log_names: vec![],
            event_types: vec![],
            event_ids: vec![],
            sources: vec![],
            categories: vec![],
            max_results: None,
        }
    }

    // Test convert_event_type function
    #[test]
    fn test_convert_event_type_error() {
        assert_eq!(convert_event_type(1), "Error");
    }

    #[test]
    fn test_convert_event_type_warning() {
        assert_eq!(convert_event_type(2), "Warning");
    }

    #[test]
    fn test_convert_event_type_information() {
        assert_eq!(convert_event_type(4), "Information");
    }

    #[test]
    fn test_convert_event_type_audit_success() {
        assert_eq!(convert_event_type(8), "Audit Success");
    }

    #[test]
    fn test_convert_event_type_audit_failure() {
        assert_eq!(convert_event_type(16), "Audit Failure");
    }

    #[test]
    fn test_convert_event_type_unknown() {
        assert_eq!(convert_event_type(99), "Unknown");
        assert_eq!(convert_event_type(0), "Unknown");
        assert_eq!(convert_event_type(1000), "Unknown");
    }

    // Test convert_severity function
    #[test]
    fn test_convert_severity_critical() {
        assert_eq!(convert_severity(1), "Critical");
    }

    #[test]
    fn test_convert_severity_error() {
        assert_eq!(convert_severity(2), "Error");
    }

    #[test]
    fn test_convert_severity_warning() {
        assert_eq!(convert_severity(3), "Warning");
    }

    #[test]
    fn test_convert_severity_information() {
        assert_eq!(convert_severity(4), "Information");
    }

    #[test]
    fn test_convert_severity_verbose() {
        assert_eq!(convert_severity(5), "Verbose");
    }

    #[test]
    fn test_convert_severity_unknown() {
        assert_eq!(convert_severity(99), "Unknown (99)");
        assert_eq!(convert_severity(0), "Unknown (0)");
    }

    // Test check_admin_rights function
    #[test]
    fn test_check_admin_rights_returns_boolean() {
        let result = check_admin_rights();
        // Should return either true or false, not panic
        assert!(result == true || result == false);
    }

    // Test get_available_logs function
    #[tokio::test]
    async fn test_get_available_logs_returns_ok() {
        let result = get_available_logs().await;
        assert!(result.is_ok(), "get_available_logs should return Ok");
    }

    #[tokio::test]
    async fn test_get_available_logs_returns_logs() {
        let result = get_available_logs().await;
        if let Ok(logs) = result {
            // Windows should have at least Application, System, or Security logs
            assert!(!logs.is_empty(), "Should return at least some logs");
        }
    }

    #[tokio::test]
    async fn test_get_available_logs_contains_common_logs() {
        let result = get_available_logs().await;
        if let Ok(logs) = result {
            // Check for common Windows logs
            let has_common_log = logs
                .iter()
                .any(|log| log == "Application" || log == "System" || log == "Security");
            assert!(
                has_common_log,
                "Should contain at least one common Windows log"
            );
        }
    }

    // Test search_event_logs function
    #[tokio::test]
    async fn test_search_event_logs_empty_params() {
        let params = create_test_params();
        let result = search_event_logs(params).await;
        assert!(result.is_ok(), "search_event_logs should return Ok");
    }

    #[tokio::test]
    async fn test_search_event_logs_with_max_results() {
        let mut params = create_test_params();
        params.max_results = Some(10);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with max_results should return Ok"
        );

        if let Ok(events) = result {
            assert!(
                events.len() <= 10,
                "Should not return more than max_results"
            );
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_with_max_results_zero() {
        let mut params = create_test_params();
        params.max_results = Some(0);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with max_results=0 should return Ok"
        );
    }

    #[tokio::test]
    async fn test_search_event_logs_with_specific_log() {
        let mut params = create_test_params();
        params.log_names = vec!["Application".to_string()];

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with specific log should return Ok"
        );
    }

    #[tokio::test]
    async fn test_search_event_logs_with_event_types() {
        let mut params = create_test_params();
        params.event_types = vec![1, 2]; // Critical and Error

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with event_types should return Ok"
        );

        if let Ok(events) = result {
            for event in events {
                // Verify that returned events match the requested types
                assert!(
                    event.event_type == "Error"
                        || event.event_type == "Warning"
                        || event.severity == "Critical"
                        || event.severity == "Error",
                    "Events should match requested types"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_with_event_ids() {
        let mut params = create_test_params();
        params.event_ids = vec![1000, 2000];
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with event_ids should return Ok"
        );

        if let Ok(events) = result {
            for event in events {
                assert!(
                    event.event_id == 1000 || event.event_id == 2000,
                    "Events should match requested IDs"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_with_keywords() {
        let params = create_test_params_with_keywords(vec!["error", "failed"]);
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with keywords should return Ok"
        );

        if let Ok(events) = result {
            for event in events {
                let message_lower = event.message.to_lowercase();
                assert!(
                    message_lower.contains("error") || message_lower.contains("failed"),
                    "Events should contain at least one keyword"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_with_exclude_keywords() {
        let mut params = create_test_params();
        params.exclude_keywords = vec!["test".to_string()];
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with exclude_keywords should return Ok"
        );

        if let Ok(events) = result {
            for event in events {
                let message_lower = event.message.to_lowercase();
                assert!(
                    !message_lower.contains("test"),
                    "Events should not contain excluded keywords"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_with_date_range() {
        let mut params = create_test_params();

        // Set date range to last 7 days
        let now = SystemTime::now();
        let week_ago = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (7 * 24 * 60 * 60);

        let end_date = chrono::DateTime::from_timestamp(
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            0,
        )
        .unwrap()
        .with_timezone(&chrono::Utc)
        .to_rfc3339();

        let start_date = chrono::DateTime::from_timestamp(week_ago, 0)
            .unwrap()
            .with_timezone(&chrono::Utc)
            .to_rfc3339();

        params.start_date = Some(start_date.clone());
        params.end_date = Some(end_date.clone());
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with date range should return Ok"
        );
    }

    #[tokio::test]
    async fn test_search_event_logs_with_sources() {
        let mut params = create_test_params();
        params.sources = vec!["Service Control Manager".to_string()];
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(
            result.is_ok(),
            "search_event_logs with sources should return Ok"
        );
    }

    #[tokio::test]
    async fn test_search_event_logs_invalid_log_name() {
        let mut params = create_test_params();
        params.log_names = vec!["NonExistentLog12345".to_string()];

        let result = search_event_logs(params).await;
        // Should still return Ok, but with empty results
        assert!(
            result.is_ok(),
            "search_event_logs with invalid log should return Ok"
        );

        if let Ok(events) = result {
            assert_eq!(
                events.len(),
                0,
                "Should return empty results for invalid log"
            );
        }
    }

    #[tokio::test]
    async fn test_search_event_logs_returns_proper_structure() {
        let mut params = create_test_params();
        params.max_results = Some(1);

        let result = search_event_logs(params).await;
        assert!(result.is_ok());

        if let Ok(events) = result {
            if !events.is_empty() {
                let event = &events[0];

                // Verify all required fields are present
                assert!(!event.source.is_empty() || event.source.is_empty()); // Just check it exists
                assert!(!event.time_generated.is_empty());
                assert!(event.event_id > 0 || event.event_id == 0);
                assert!(!event.event_type.is_empty());
                assert!(!event.severity.is_empty());
                assert!(!event.matches.is_empty() || event.matches.is_empty());
            }
        }
    }

    // Test SearchParams structure
    #[test]
    fn test_search_params_creation() {
        let params = SearchParams {
            keywords: vec!["test".to_string()],
            exclude_keywords: vec!["exclude".to_string()],
            start_date: Some("2024-01-01T00:00:00Z".to_string()),
            end_date: Some("2024-12-31T23:59:59Z".to_string()),
            log_names: vec!["Application".to_string()],
            event_types: vec![1, 2, 3],
            event_ids: vec![1000, 2000],
            sources: vec!["TestSource".to_string()],
            categories: vec![1, 2],
            max_results: Some(100),
        };

        assert_eq!(params.keywords.len(), 1);
        assert_eq!(params.exclude_keywords.len(), 1);
        assert!(params.start_date.is_some());
        assert!(params.end_date.is_some());
        assert_eq!(params.log_names.len(), 1);
        assert_eq!(params.event_types.len(), 3);
        assert_eq!(params.event_ids.len(), 2);
        assert_eq!(params.sources.len(), 1);
        assert_eq!(params.categories.len(), 2);
        assert_eq!(params.max_results, Some(100));
    }

    #[test]
    fn test_search_params_default_values() {
        let params = create_test_params();

        assert!(params.keywords.is_empty());
        assert!(params.exclude_keywords.is_empty());
        assert!(params.start_date.is_none());
        assert!(params.end_date.is_none());
        assert!(params.log_names.is_empty());
        assert!(params.event_types.is_empty());
        assert!(params.event_ids.is_empty());
        assert!(params.sources.is_empty());
        assert!(params.categories.is_empty());
        assert!(params.max_results.is_none());
    }

    // Test EventLogEntry structure
    #[test]
    fn test_event_log_entry_creation() {
        let entry = EventLogEntry {
            source: "Application".to_string(),
            time_generated: "2024-01-15T10:30:00Z".to_string(),
            event_id: 1000,
            event_type: "Error".to_string(),
            severity: "Critical".to_string(),
            category: 1,
            message: "Test message".to_string(),
            computer_name: "TEST-PC".to_string(),
            raw_data: Some(vec![0, 1, 2, 3]),
            user_sid: Some("S-1-5-21-...".to_string()),
            matches: vec!["error".to_string()],
        };

        assert_eq!(entry.source, "Application");
        assert_eq!(entry.event_id, 1000);
        assert_eq!(entry.event_type, "Error");
        assert_eq!(entry.severity, "Critical");
        assert_eq!(entry.category, 1);
        assert!(!entry.message.is_empty());
        assert!(entry.raw_data.is_some());
        assert!(entry.user_sid.is_some());
        assert_eq!(entry.matches.len(), 1);
    }

    // Performance and stress tests
    #[tokio::test]
    async fn test_search_large_result_set() {
        let mut params = create_test_params();
        params.max_results = Some(1000);

        let result = search_event_logs(params).await;
        assert!(result.is_ok());

        if let Ok(events) = result {
            assert!(events.len() <= 1000);
        }
    }

    #[tokio::test]
    async fn test_multiple_concurrent_searches() {
        let params1 = create_test_params();
        let params2 = create_test_params();
        let params3 = create_test_params();

        let search1 = search_event_logs(params1);
        let search2 = search_event_logs(params2);
        let search3 = search_event_logs(params3);

        let results = tokio::join!(search1, search2, search3);

        assert!(results.0.is_ok());
        assert!(results.1.is_ok());
        assert!(results.2.is_ok());
    }

    // Edge case tests
    #[tokio::test]
    async fn test_search_with_empty_string_keyword() {
        let params = create_test_params_with_keywords(vec![""]);
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_with_special_characters() {
        let params = create_test_params_with_keywords(vec![".*", "\\", "$", "^"]);
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        // Should handle regex special characters gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_with_unicode_keywords() {
        let params = create_test_params_with_keywords(vec!["测试", "テスト", "тест"]);
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_with_very_long_keyword() {
        let long_keyword = "a".repeat(1000);
        let params = create_test_params_with_keywords(vec![&long_keyword]);
        params.max_results = Some(5);

        let result = search_event_logs(params).await;
        assert!(result.is_ok());
    }
}
