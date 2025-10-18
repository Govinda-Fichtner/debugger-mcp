/// Unit tests for test-results.json validation logic
///
/// These tests validate that the enhanced validation correctly detects
/// and handles corrupted, empty, or invalid test-results.json files.
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Simulates the validation logic from integration tests
fn validate_test_results_file(file_path: &PathBuf) -> (bool, Option<String>) {
    // Check if file exists and has non-zero size
    let mut needs_extraction =
        !file_path.exists() || fs::metadata(file_path).map(|m| m.len()).unwrap_or(0) == 0;

    let mut validation_message = None;

    // Enhanced: Also validate the file contains valid, parseable JSON
    if !needs_extraction && file_path.exists() {
        if let Ok(content) = fs::read_to_string(file_path) {
            let trimmed = content.trim();

            // Check if file is empty or doesn't contain required fields
            if trimmed.is_empty()
                || !trimmed.contains("\"test_run\"")
                || !trimmed.contains("\"operations\"")
            {
                validation_message = Some(
                    "test-results.json exists but is empty or missing required fields".to_string(),
                );
                needs_extraction = true;
            } else {
                // Validate it's actually parseable JSON
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(_) => {
                        validation_message = Some(format!(
                            "Valid test-results.json found ({} bytes)",
                            trimmed.len()
                        ));
                    }
                    Err(e) => {
                        validation_message = Some(format!(
                            "test-results.json exists but contains invalid JSON: {}",
                            e
                        ));
                        needs_extraction = true;
                    }
                }
            }
        } else {
            validation_message =
                Some("test-results.json exists but cannot be read as UTF-8".to_string());
            needs_extraction = true;
        }
    }

    (needs_extraction, validation_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        let valid_json = json!({
            "test_run": {
                "language": "rust",
                "timestamp": "2025-10-18T00:00:00Z",
                "overall_success": true
            },
            "operations": {
                "session_started": true,
                "breakpoint_set": true,
                "breakpoint_verified": true,
                "execution_continued": true,
                "stopped_at_breakpoint": true,
                "stack_trace_retrieved": true,
                "variable_evaluated": true,
                "session_disconnected": true
            },
            "errors": []
        });

        fs::write(
            &file_path,
            serde_json::to_string_pretty(&valid_json).unwrap(),
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(!needs_extraction, "Valid JSON should not need extraction");
        assert!(message.is_some());
        assert!(message.unwrap().contains("Valid test-results.json found"));
    }

    #[test]
    fn test_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Create empty file
        fs::write(&file_path, "").unwrap();

        let (needs_extraction, _message) = validate_test_results_file(&file_path);

        assert!(needs_extraction, "Empty file should trigger extraction");
    }

    #[test]
    fn test_whitespace_only_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Create file with only whitespace
        fs::write(&file_path, "   \n\t  \n  ").unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(
            needs_extraction,
            "Whitespace-only file should trigger extraction"
        );
        assert!(message.is_some());
        assert!(message
            .unwrap()
            .contains("empty or missing required fields"));
    }

    #[test]
    fn test_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Create file with invalid JSON that has required fields
        fs::write(
            &file_path,
            r#"{"test_run": {"language": "rust"}, "operations": {}, INVALID}"#,
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(needs_extraction, "Invalid JSON should trigger extraction");
        assert!(message.is_some());
        let msg = message.unwrap();
        assert!(
            msg.contains("invalid JSON") || msg.contains("missing required fields"),
            "Expected error about invalid JSON or missing fields, got: {}",
            msg
        );
    }

    #[test]
    fn test_missing_test_run_field() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        let invalid_json = json!({
            "operations": {
                "session_started": true
            },
            "errors": []
        });

        fs::write(
            &file_path,
            serde_json::to_string_pretty(&invalid_json).unwrap(),
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(
            needs_extraction,
            "JSON missing test_run field should trigger extraction"
        );
        assert!(message.is_some());
        assert!(message
            .unwrap()
            .contains("empty or missing required fields"));
    }

    #[test]
    fn test_missing_operations_field() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        let invalid_json = json!({
            "test_run": {
                "language": "rust",
                "timestamp": "2025-10-18T00:00:00Z"
            },
            "errors": []
        });

        fs::write(
            &file_path,
            serde_json::to_string_pretty(&invalid_json).unwrap(),
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(
            needs_extraction,
            "JSON missing operations field should trigger extraction"
        );
        assert!(message.is_some());
        assert!(message
            .unwrap()
            .contains("empty or missing required fields"));
    }

    #[test]
    fn test_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("does-not-exist.json");

        let (needs_extraction, _message) = validate_test_results_file(&file_path);

        assert!(
            needs_extraction,
            "Nonexistent file should trigger extraction"
        );
    }

    #[test]
    fn test_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Create file with binary data (non-UTF8)
        fs::write(&file_path, vec![0xFF, 0xFE, 0xFD, 0xFC]).unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(needs_extraction, "Binary file should trigger extraction");
        assert!(message.is_some());
        assert!(message.unwrap().contains("cannot be read as UTF-8"));
    }

    #[test]
    fn test_partial_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Create file with partial JSON (missing closing braces)
        fs::write(
            &file_path,
            r#"{"test_run": {"language": "rust", "operations": {"#,
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(needs_extraction, "Partial JSON should trigger extraction");
        assert!(message.is_some());
        assert!(message.unwrap().contains("invalid JSON"));
    }

    #[test]
    fn test_json_with_extra_fields() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        // Valid JSON with extra fields should still be accepted
        let json_with_extras = json!({
            "test_run": {
                "language": "rust",
                "timestamp": "2025-10-18T00:00:00Z",
                "overall_success": true,
                "extra_field": "extra value"
            },
            "operations": {
                "session_started": true,
                "breakpoint_set": true
            },
            "errors": [],
            "metadata": {
                "some": "extra data"
            }
        });

        fs::write(
            &file_path,
            serde_json::to_string_pretty(&json_with_extras).unwrap(),
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(
            !needs_extraction,
            "Valid JSON with extra fields should be accepted"
        );
        assert!(message.is_some());
        assert!(message.unwrap().contains("Valid test-results.json found"));
    }

    #[test]
    fn test_json_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-results.json");

        let json_with_unicode = json!({
            "test_run": {
                "language": "rust",
                "timestamp": "2025-10-18T00:00:00Z",
                "message": "Test with 特殊文字 and émojis 🎉"
            },
            "operations": {
                "session_started": true
            },
            "errors": []
        });

        fs::write(
            &file_path,
            serde_json::to_string_pretty(&json_with_unicode).unwrap(),
        )
        .unwrap();

        let (needs_extraction, message) = validate_test_results_file(&file_path);

        assert!(!needs_extraction, "JSON with Unicode should be accepted");
        assert!(message.is_some());
        assert!(message.unwrap().contains("Valid test-results.json found"));
    }
}
