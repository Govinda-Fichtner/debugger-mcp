/// Unit tests for test result extraction logic
///
/// This module provides comprehensive test coverage for the critical
/// test-results.json extraction and reconstruction logic used in CI.
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Reconstruct test-results.json from mcp_protocol_log.md by parsing MCP tool operations
fn reconstruct_test_results_from_protocol_log(log_content: &str, language: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    // Parse the log to detect which operations succeeded
    let session_started =
        log_content.contains("debugger_start") && log_content.contains("\"status\": \"started\"");

    let breakpoint_set = log_content.contains("debugger_set_breakpoint");
    let breakpoint_verified = log_content.contains("\"verified\": true");

    let execution_continued = log_content.contains("debugger_continue")
        && log_content.contains("\"status\": \"continued\"");

    let stopped_at_breakpoint = log_content.contains("debugger_wait_for_stop")
        && log_content.contains("\"reason\": \"breakpoint\"");

    let stack_trace_retrieved =
        log_content.contains("debugger_stack_trace") && log_content.contains("\"stackFrames\"");

    let variable_evaluated = log_content.contains("debugger_evaluate")
        && (log_content.contains("\"result\":") || log_content.contains("\"value\":"));

    let session_disconnected = log_content.contains("debugger_disconnect")
        && log_content.contains("\"status\": \"disconnected\"");

    // Collect errors from the log
    let mut errors = Vec::new();

    if session_started && !breakpoint_verified {
        errors.push(json!({
            "operation": "breakpoint_set",
            "message": "Breakpoint was not verified (likely missing debug symbols)"
        }));
    }

    if !stopped_at_breakpoint && execution_continued {
        errors.push(json!({
            "operation": "execution",
            "message": "Program did not stop at breakpoint"
        }));
    }

    let overall_success = session_started
        && breakpoint_set
        && execution_continued
        && session_disconnected
        && errors.is_empty();

    // Generate JSON
    let result = json!({
        "test_run": {
            "language": language,
            "timestamp": timestamp,
            "overall_success": overall_success,
            "reconstructed_from": "mcp_protocol_log.md"
        },
        "operations": {
            "session_started": session_started,
            "breakpoint_set": breakpoint_set,
            "breakpoint_verified": breakpoint_verified,
            "execution_continued": execution_continued,
            "stopped_at_breakpoint": stopped_at_breakpoint,
            "stack_trace_retrieved": stack_trace_retrieved,
            "variable_evaluated": variable_evaluated,
            "session_disconnected": session_disconnected
        },
        "errors": errors
    });

    serde_json::to_string_pretty(&result).unwrap()
}

/// Extract JSON from stdout markdown code blocks
fn extract_json_from_stdout(stdout: &str) -> Option<String> {
    if let Some(json_start) = stdout.find("```json") {
        let search_slice = &stdout[json_start + 7..]; // Skip "```json"
        if let Some(json_end_offset) = search_slice.find("```") {
            let json_content = search_slice[..json_end_offset].trim();

            // Validate it's actually JSON for test_run
            if json_content.contains("\"test_run\"") && json_content.contains("\"operations\"") {
                return Some(json_content.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Reconstruction Tests =====

    #[test]
    fn test_reconstruct_all_operations_successful() {
        let log = r#"
### Tool: debugger_start
Result: {"status": "started"}

### Tool: debugger_set_breakpoint
Result: {"verified": true}

### Tool: debugger_continue
Result: {"status": "continued"}

### Tool: debugger_wait_for_stop
Result: {"reason": "breakpoint"}

### Tool: debugger_stack_trace
Result: {"stackFrames": []}

### Tool: debugger_evaluate
Result: {"result": "42"}

### Tool: debugger_disconnect
Result: {"status": "disconnected"}
"#;

        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["test_run"]["language"], "python");
        assert_eq!(json["test_run"]["overall_success"], true);
        assert_eq!(json["operations"]["session_started"], true);
        assert_eq!(json["operations"]["breakpoint_set"], true);
        assert_eq!(json["operations"]["breakpoint_verified"], true);
        assert_eq!(json["operations"]["execution_continued"], true);
        assert_eq!(json["operations"]["stopped_at_breakpoint"], true);
        assert_eq!(json["operations"]["stack_trace_retrieved"], true);
        assert_eq!(json["operations"]["variable_evaluated"], true);
        assert_eq!(json["operations"]["session_disconnected"], true);
        assert_eq!(json["errors"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_reconstruct_breakpoint_not_verified() {
        let log = r#"
debugger_start
"status": "started"
debugger_set_breakpoint
"verified": false
debugger_continue
"status": "continued"
debugger_disconnect
"status": "disconnected"
"#;

        let result = reconstruct_test_results_from_protocol_log(log, "rust");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["test_run"]["overall_success"], false);
        assert_eq!(json["operations"]["breakpoint_verified"], false);
        // Should have 2 errors: unverified breakpoint AND didn't stop at breakpoint
        let errors = json["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        assert_eq!(errors[0]["operation"], "breakpoint_set");
    }

    #[test]
    fn test_reconstruct_program_exited_without_stopping() {
        let log = r#"
debugger_start
"status": "started"
debugger_set_breakpoint
"verified": true
debugger_continue
"status": "continued"
debugger_wait_for_stop
"reason": "exited"
debugger_disconnect
"status": "disconnected"
"#;

        let result = reconstruct_test_results_from_protocol_log(log, "go");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["test_run"]["overall_success"], false);
        assert_eq!(json["operations"]["stopped_at_breakpoint"], false);
        assert!(!json["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_reconstruct_session_not_started() {
        let log = "No operations performed";

        let result = reconstruct_test_results_from_protocol_log(log, "nodejs");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["operations"]["session_started"], false);
        assert_eq!(json["operations"]["breakpoint_set"], false);
        assert_eq!(json["test_run"]["overall_success"], false);
    }

    #[test]
    fn test_reconstruct_partial_operations() {
        let log = r#"
debugger_start
"status": "started"
debugger_set_breakpoint
debugger_continue
"status": "continued"
"#;

        let result = reconstruct_test_results_from_protocol_log(log, "ruby");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["operations"]["session_started"], true);
        assert_eq!(json["operations"]["breakpoint_set"], true);
        assert_eq!(json["operations"]["execution_continued"], true);
        assert_eq!(json["operations"]["session_disconnected"], false);
        assert_eq!(json["test_run"]["overall_success"], false);
    }

    #[test]
    fn test_reconstruct_different_languages() {
        let languages = vec!["python", "rust", "go", "nodejs", "ruby"];

        for lang in languages {
            let log = "debugger_start\n\"status\": \"started\"";
            let result = reconstruct_test_results_from_protocol_log(log, lang);
            let json: serde_json::Value = serde_json::from_str(&result).unwrap();

            assert_eq!(json["test_run"]["language"], lang);
            assert_eq!(
                json["test_run"]["reconstructed_from"],
                "mcp_protocol_log.md"
            );
        }
    }

    #[test]
    fn test_reconstruct_contains_timestamp() {
        let log = "debugger_start";
        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(json["test_run"]["timestamp"].is_string());
        let timestamp = json["test_run"]["timestamp"].as_str().unwrap();
        assert!(!timestamp.is_empty());
    }

    #[test]
    fn test_reconstruct_evaluates_with_result_field() {
        let log = r#"debugger_evaluate
"result": "value""#;

        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["operations"]["variable_evaluated"], true);
    }

    #[test]
    fn test_reconstruct_evaluates_with_value_field() {
        let log = r#"debugger_evaluate
"value": "42""#;

        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["operations"]["variable_evaluated"], true);
    }

    // ===== JSON Extraction from Stdout Tests =====

    #[test]
    fn test_extract_valid_json_from_stdout() {
        let stdout = r#"Here's the test result:
```json
{
  "test_run": {
    "language": "python"
  },
  "operations": {}
}
```
Done!"#;

        let result = extract_json_from_stdout(stdout);
        assert!(result.is_some());

        let json_str = result.unwrap();
        assert!(json_str.contains("\"test_run\""));
        assert!(json_str.contains("\"operations\""));
        assert!(json_str.contains("python"));
    }

    #[test]
    fn test_extract_no_json_blocks() {
        let stdout = "No JSON here, just plain text";
        let result = extract_json_from_stdout(stdout);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_json_wrong_format() {
        let stdout = r#"```json
{
  "wrong": "format"
}
```"#;

        let result = extract_json_from_stdout(stdout);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_multiple_json_blocks() {
        let stdout = r#"
First block:
```json
{"other": "data"}
```

Second block (the right one):
```json
{
  "test_run": {"language": "rust"},
  "operations": {}
}
```
"#;

        // The function finds the first valid json block with test_run
        let result = extract_json_from_stdout(stdout);
        // Current implementation finds first json block, which doesn't have test_run
        // So this should be None
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_json_with_whitespace() {
        let stdout = r#"
```json

  {
    "test_run": {
      "language": "go"
    },
    "operations": {
      "session_started": true
    }
  }

```
"#;

        let result = extract_json_from_stdout(stdout);
        assert!(result.is_some());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["test_run"]["language"], "go");
    }

    #[test]
    fn test_extract_incomplete_json_block() {
        let stdout = "```json\n{\"test_run\": {";
        let result = extract_json_from_stdout(stdout);
        assert!(result.is_none());
    }

    // ===== Edge Cases =====

    #[test]
    fn test_reconstruct_empty_log() {
        let log = "";
        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["test_run"]["overall_success"], false);
        assert!(json["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_reconstruct_very_large_log() {
        let mut log = String::from("debugger_start\n\"status\": \"started\"\n");
        log.push_str(&"x".repeat(100000));
        log.push_str("\ndebugger_disconnect\n\"status\": \"disconnected\"");

        let result = reconstruct_test_results_from_protocol_log(&log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["operations"]["session_started"], true);
        assert_eq!(json["operations"]["session_disconnected"], true);
    }

    #[test]
    fn test_reconstruct_special_characters_in_log() {
        let log = r#"
debugger_start "status": "started"
Some 特殊文字 and émojis 🎉
debugger_set_breakpoint
"verified": true
"#;

        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let _json: serde_json::Value = serde_json::from_str(&result).unwrap();
        // Should not panic on special characters
    }

    #[test]
    fn test_extract_json_with_nested_code_blocks() {
        let stdout = r#"
```json
{
  "test_run": {
    "language": "python",
    "code_sample": "```python\nprint('hi')\n```"
  },
  "operations": {}
}
```
"#;

        // This is a limitation - nested code blocks will break the extraction
        // The simple implementation finds the first ``` which is the nested one
        let result = extract_json_from_stdout(stdout);
        // Expected to fail due to nested code block limitation
        assert!(result.is_none());
    }

    // ===== Validation Tests =====

    #[test]
    fn test_reconstruct_produces_valid_json() {
        let log = "debugger_start\n\"status\": \"started\"";
        let result = reconstruct_test_results_from_protocol_log(log, "python");

        // Should parse without error
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should have required fields
        assert!(json.get("test_run").is_some());
        assert!(json.get("operations").is_some());
        assert!(json.get("errors").is_some());
    }

    #[test]
    fn test_reconstructed_json_schema() {
        let log = "debugger_start\n\"status\": \"started\"";
        let result = reconstruct_test_results_from_protocol_log(log, "python");
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Verify schema structure
        assert!(json["test_run"]["language"].is_string());
        assert!(json["test_run"]["timestamp"].is_string());
        assert!(json["test_run"]["overall_success"].is_boolean());
        assert!(json["test_run"]["reconstructed_from"].is_string());

        // Verify all operation flags are booleans
        let ops = &json["operations"];
        assert!(ops["session_started"].is_boolean());
        assert!(ops["breakpoint_set"].is_boolean());
        assert!(ops["breakpoint_verified"].is_boolean());
        assert!(ops["execution_continued"].is_boolean());
        assert!(ops["stopped_at_breakpoint"].is_boolean());
        assert!(ops["stack_trace_retrieved"].is_boolean());
        assert!(ops["variable_evaluated"].is_boolean());
        assert!(ops["session_disconnected"].is_boolean());

        // Verify errors is an array
        assert!(json["errors"].is_array());
    }
}
