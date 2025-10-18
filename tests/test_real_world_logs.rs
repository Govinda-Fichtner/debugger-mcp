/// Real-world integration tests using actual mcp_protocol_log.md files from CI
///
/// These tests validate that the extraction logic works with real logs
/// captured from actual CI runs, not just synthetic test data.
use serde_json::json;
use std::fs;
use std::path::PathBuf;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_real_world_log() {
        let log_path = PathBuf::from("tests/fixtures/rust_mcp_protocol_log.md");
        let log_content = fs::read_to_string(&log_path).expect("Failed to read Rust protocol log");

        let result = reconstruct_test_results_from_protocol_log(&log_content, "rust");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse reconstructed JSON");

        // Validate structure
        assert!(json["test_run"]["language"].is_string());
        assert_eq!(json["test_run"]["language"], "rust");
        assert!(json["operations"].is_object());
        assert!(json["errors"].is_array());

        // Based on actual Rust log, we expect:
        // - Session started: true
        // - Breakpoint set: true
        // - Breakpoint verified: false (debug symbols issue)
        // - Stack trace retrieved: true
        assert_eq!(json["operations"]["session_started"], true);
        assert_eq!(json["operations"]["breakpoint_set"], true);
        assert_eq!(json["operations"]["stack_trace_retrieved"], true);

        println!("✅ Rust real-world log reconstruction:");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_python_real_world_log() {
        let log_path = PathBuf::from("tests/fixtures/python_mcp_protocol_log.md");
        let log_content =
            fs::read_to_string(&log_path).expect("Failed to read Python protocol log");

        let result = reconstruct_test_results_from_protocol_log(&log_content, "python");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse reconstructed JSON");

        assert_eq!(json["test_run"]["language"], "python");
        assert!(json["operations"].is_object());

        println!("✅ Python real-world log reconstruction:");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_go_real_world_log() {
        let log_path = PathBuf::from("tests/fixtures/go_mcp_protocol_log.md");
        let log_content = fs::read_to_string(&log_path).expect("Failed to read Go protocol log");

        let result = reconstruct_test_results_from_protocol_log(&log_content, "go");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse reconstructed JSON");

        assert_eq!(json["test_run"]["language"], "go");

        println!("✅ Go real-world log reconstruction:");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_nodejs_real_world_log() {
        let log_path = PathBuf::from("tests/fixtures/nodejs_mcp_protocol_log.md");
        let log_content =
            fs::read_to_string(&log_path).expect("Failed to read Node.js protocol log");

        let result = reconstruct_test_results_from_protocol_log(&log_content, "nodejs");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse reconstructed JSON");

        assert_eq!(json["test_run"]["language"], "nodejs");

        println!("✅ Node.js real-world log reconstruction:");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_ruby_real_world_log() {
        let log_path = PathBuf::from("tests/fixtures/ruby_mcp_protocol_log.md");
        let log_content = fs::read_to_string(&log_path).expect("Failed to read Ruby protocol log");

        let result = reconstruct_test_results_from_protocol_log(&log_content, "ruby");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse reconstructed JSON");

        assert_eq!(json["test_run"]["language"], "ruby");

        println!("✅ Ruby real-world log reconstruction:");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_all_real_world_logs_produce_valid_json() {
        let languages = vec!["rust", "python", "go", "nodejs", "ruby"];

        for lang in languages {
            let log_path = PathBuf::from(format!("tests/fixtures/{}_mcp_protocol_log.md", lang));

            let log_content = fs::read_to_string(&log_path)
                .unwrap_or_else(|_| panic!("Failed to read {} log", lang));

            let result = reconstruct_test_results_from_protocol_log(&log_content, lang);

            // Should parse without error
            let json: serde_json::Value = serde_json::from_str(&result)
                .unwrap_or_else(|_| panic!("{} produced invalid JSON", lang));

            // Validate required fields exist
            assert!(json.get("test_run").is_some(), "{} missing test_run", lang);
            assert!(
                json.get("operations").is_some(),
                "{} missing operations",
                lang
            );
            assert!(json.get("errors").is_some(), "{} missing errors", lang);

            println!("✅ {} log produces valid JSON", lang);
        }
    }
}
