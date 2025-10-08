/// Integration tests for Ruby debugging
///
/// These tests verify that the Ruby adapter works correctly end-to-end,
/// including the critical command-line argument structure that was previously broken.
///
/// Test Coverage:
/// 1. Ruby adapter spawning with correct command-line args
/// 2. Program path and arguments passed correctly
/// 3. stopOnEntry flag handling (--stop-at-load vs --nonstop)
/// 4. DAP communication via stdio (not socket mode)
/// 5. Basic debugging workflow (start, breakpoint, continue, evaluate)

use debugger_mcp::adapters::ruby::RubyAdapter;
use debugger_mcp::debug::manager::SessionManager;
use debugger_mcp::mcp::tools::ToolsHandler;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test that Ruby adapter command is "rdbg"
#[test]
fn test_ruby_adapter_command() {
    assert_eq!(RubyAdapter::command(), "rdbg");
}

/// Test that Ruby adapter ID is "rdbg"
#[test]
fn test_ruby_adapter_id() {
    assert_eq!(RubyAdapter::adapter_id(), "rdbg");
}

/// Test that launch args include program path and args
#[test]
fn test_ruby_launch_args_structure() {
    let program = "/workspace/fizzbuzz.rb";
    let program_args = vec!["100".to_string()];
    let cwd = Some("/workspace");
    let launch_args = RubyAdapter::launch_args_with_options(program, &program_args, cwd, true);

    assert_eq!(launch_args["request"], "launch");
    assert_eq!(launch_args["type"], "ruby");
    assert_eq!(launch_args["program"], program);
    assert_eq!(launch_args["args"], json!(program_args));
    assert_eq!(launch_args["stopOnEntry"], true);
    assert_eq!(launch_args["localfs"], true);
    assert_eq!(launch_args["cwd"], "/workspace");
}

/// Test that launch args handle missing cwd
#[test]
fn test_ruby_launch_args_no_cwd() {
    let program = "/workspace/test.rb";
    let program_args = Vec::<String>::new();
    let launch_args = RubyAdapter::launch_args_with_options(program, &program_args, None, false);

    assert_eq!(launch_args["program"], program);
    assert_eq!(launch_args["stopOnEntry"], false);
    assert!(launch_args["cwd"].is_null());
}

/// Test Ruby session creation (requires Docker)
#[tokio::test]
#[ignore] // Requires Docker and rdbg installed
async fn test_ruby_session_creation() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager);

    let args = json!({
        "language": "ruby",
        "program": "/workspace/fizzbuzz.rb",
        "stopOnEntry": true
    });

    // This should spawn: rdbg --stop-at-load /workspace/fizzbuzz.rb
    let result = tools_handler
        .handle_tool("debugger_start", args)
        .await;

    assert!(result.is_ok(), "Ruby session creation failed: {:?}", result.err());

    let response = result.unwrap();
    assert!(response["sessionId"].is_string());
    assert_eq!(response["status"], "initializing");
}

/// Test Ruby session with program arguments (requires Docker)
#[tokio::test]
#[ignore] // Requires Docker and rdbg installed
async fn test_ruby_session_with_program_args() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager);

    let args = json!({
        "language": "ruby",
        "program": "/workspace/fizzbuzz.rb",
        "args": ["50"],
        "stopOnEntry": false
    });

    // This should spawn: rdbg --nonstop /workspace/fizzbuzz.rb 50
    let result = tools_handler
        .handle_tool("debugger_start", args)
        .await;

    assert!(result.is_ok(), "Ruby session with args failed: {:?}", result.err());
}

/// Test that Ruby sessions can set breakpoints (requires Docker)
#[tokio::test]
#[ignore] // Requires Docker and rdbg installed
async fn test_ruby_breakpoint_setting() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    // Start session
    let start_args = json!({
        "language": "ruby",
        "program": "/workspace/fizzbuzz.rb",
        "stopOnEntry": true
    });

    let start_result = tools_handler
        .handle_tool("debugger_start", start_args)
        .await
        .expect("Failed to start Ruby session");

    let session_id = start_result["sessionId"].as_str().unwrap();

    // Wait for initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Set breakpoint at the buggy line (line 9: n % 4 should be n % 5)
    let bp_args = json!({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz.rb",
        "line": 9
    });

    let bp_result = tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await;

    assert!(bp_result.is_ok(), "Ruby breakpoint setting failed: {:?}", bp_result.err());

    let bp_response = bp_result.unwrap();
    assert_eq!(bp_response["verified"], true, "Breakpoint not verified");
    assert_eq!(bp_response["line"], 9);
}

/// Integration test: Full Ruby debugging workflow (requires Docker)
#[tokio::test]
#[ignore] // Requires Docker and rdbg installed
async fn test_ruby_full_debugging_workflow() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    // 1. Start session with stopOnEntry
    let start_args = json!({
        "language": "ruby",
        "program": "/workspace/fizzbuzz.rb",
        "stopOnEntry": true
    });

    let start_result = tools_handler
        .handle_tool("debugger_start", start_args)
        .await
        .expect("Failed to start Ruby session");

    let session_id = start_result["sessionId"].as_str().unwrap().to_string();

    // 2. Wait for entry point
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 3. Wait for stop at entry
    let wait_args = json!({
        "sessionId": session_id,
        "timeout": 5000
    });

    let wait_result = tools_handler
        .handle_tool("debugger_wait_for_stop", wait_args)
        .await
        .expect("Failed to wait for stop");

    assert_eq!(wait_result["stopped"], true);

    // 4. Set breakpoint at line 9
    let bp_args = json!({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz.rb",
        "line": 9
    });

    tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await
        .expect("Failed to set breakpoint");

    // 5. Continue execution
    let continue_args = json!({
        "sessionId": session_id
    });

    tools_handler
        .handle_tool("debugger_continue", continue_args.clone())
        .await
        .expect("Failed to continue");

    // 6. Wait for breakpoint hit
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let wait2_args = json!({
        "sessionId": session_id,
        "timeout": 5000
    });

    let wait2_result = tools_handler
        .handle_tool("debugger_wait_for_stop", wait2_args)
        .await
        .expect("Failed to wait for breakpoint");

    assert_eq!(wait2_result["stopped"], true);

    // 7. Get stack trace
    let stack_args = json!({
        "sessionId": session_id
    });

    let stack_result = tools_handler
        .handle_tool("debugger_stack_trace", stack_args)
        .await
        .expect("Failed to get stack trace");

    let frames = stack_result["stackFrames"].as_array().unwrap();
    assert!(!frames.is_empty());

    // 8. Evaluate variable at breakpoint
    let frame_id = frames[0]["id"].as_i64().unwrap();
    let eval_args = json!({
        "sessionId": session_id,
        "expression": "n",
        "frameId": frame_id
    });

    let eval_result = tools_handler
        .handle_tool("debugger_evaluate", eval_args)
        .await
        .expect("Failed to evaluate variable");

    assert!(eval_result["result"].is_string() || eval_result["result"].is_number());
}
