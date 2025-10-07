/// Integration tests for Rust debugging
///
/// These tests verify that the Rust adapter works correctly end-to-end,
/// including the unique compilation step before debugging.
///
/// Test Coverage:
/// 1. Rust adapter spawning with CodeLLDB
/// 2. Compilation of single-file Rust programs
/// 3. Binary path derivation from source files
/// 4. stopOnEntry flag handling (native LLDB support)
/// 5. DAP communication via stdio (like Python)
/// 6. Basic debugging workflow (compile, start, breakpoint, continue, evaluate)

use debugger_mcp::adapters::rust::RustAdapter;
use debugger_mcp::debug::manager::SessionManager;
use debugger_mcp::mcp::tools::ToolsHandler;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test that Rust adapter command points to CodeLLDB
#[test]
fn test_rust_adapter_command() {
    let command = RustAdapter::command();
    // Should be a CodeLLDB path
    assert!(command.contains("codelldb"), "Command should be codelldb, got: {}", command);
}

/// Test that Rust adapter ID is "codelldb"
#[test]
fn test_rust_adapter_id() {
    assert_eq!(RustAdapter::adapter_id(), "codelldb");
}

/// Test that CodeLLDB args use STDIO mode (empty args)
#[test]
fn test_rust_adapter_args() {
    let args = RustAdapter::args();
    assert_eq!(args, Vec::<String>::new(), "CodeLLDB should use STDIO mode (no args = default)");
}

/// Test that launch args use binary path (not source) and include stopOnEntry
#[test]
fn test_rust_launch_args_structure() {
    let binary_path = "/workspace/fizzbuzz-rust-test/target/debug/fizzbuzz";
    let program_args = vec!["100".to_string()];
    let cwd = Some("/workspace/fizzbuzz-rust-test");
    let launch_args = RustAdapter::launch_args(binary_path, &program_args, cwd, true);

    assert_eq!(launch_args["request"], "launch");
    assert_eq!(launch_args["type"], "lldb");
    assert_eq!(launch_args["program"], binary_path, "Should use compiled binary, not source");
    assert_eq!(launch_args["args"], json!(program_args));
    assert_eq!(launch_args["stopOnEntry"], true);
    assert_eq!(launch_args["cwd"], "/workspace/fizzbuzz-rust-test");
}

/// Test that launch args handle missing cwd
#[test]
fn test_rust_launch_args_no_cwd() {
    let binary_path = "/workspace/target/debug/test";
    let program_args = Vec::<String>::new();
    let launch_args = RustAdapter::launch_args(binary_path, &program_args, None, false);

    assert_eq!(launch_args["program"], binary_path);
    assert_eq!(launch_args["stopOnEntry"], false);
    assert!(launch_args["cwd"].is_null());
}

/// Test that stopOnEntry can be disabled
#[test]
fn test_rust_launch_args_no_stop_on_entry() {
    let binary_path = "/workspace/target/debug/app";
    let launch_args = RustAdapter::launch_args(binary_path, &[], None, false);

    assert_eq!(launch_args["stopOnEntry"], false, "stopOnEntry should be false when disabled");
}

/// Test compilation of a single Rust file (requires rustc)
#[tokio::test]
#[ignore] // Requires rustc and filesystem access
async fn test_rust_compilation_single_file() {
    // Use the FizzBuzz test file
    let source_path = "/workspace/fizzbuzz-rust-test/fizzbuzz.rs";

    // Compile with debug symbols
    let result = RustAdapter::compile_single_file(source_path, false).await;

    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let binary_path = result.unwrap();
    assert!(binary_path.ends_with("target/debug/fizzbuzz"),
            "Binary path should end with target/debug/fizzbuzz, got: {}", binary_path);

    // Verify binary exists (in Docker container)
    // Note: This test requires running in the debugger-mcp-rust Docker container
}

/// Test compilation error handling
#[tokio::test]
#[ignore] // Requires rustc
async fn test_rust_compilation_error() {
    // Try to compile non-existent file
    let result = RustAdapter::compile_single_file("/tmp/nonexistent.rs", false).await;

    assert!(result.is_err(), "Should fail for non-existent file");
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Compilation error"),
            "Error should mention compilation failure: {}", error);
}

/// Test Rust session creation (requires Docker with rustc and CodeLLDB)
#[tokio::test]
#[ignore] // Requires Docker with rustc and CodeLLDB installed
async fn test_rust_session_creation() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager);

    let args = json!({
        "language": "rust",
        "program": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
        "stopOnEntry": true
    });

    // This should:
    // 1. Compile: rustc fizzbuzz.rs -o target/debug/fizzbuzz
    // 2. Spawn: codelldb --port 0
    // 3. Launch: target/debug/fizzbuzz
    let result = tools_handler
        .handle_tool("debugger_start", args)
        .await;

    assert!(result.is_ok(), "Rust session creation failed: {:?}", result.err());

    let response = result.unwrap();
    assert!(response["sessionId"].is_string());
    assert_eq!(response["status"], "initializing");
}

/// Test Rust session with program arguments (requires Docker)
#[tokio::test]
#[ignore] // Requires Docker with rustc and CodeLLDB
async fn test_rust_session_with_program_args() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager);

    let args = json!({
        "language": "rust",
        "program": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
        "args": ["50"],
        "stopOnEntry": false
    });

    // Should compile, then launch with args
    let result = tools_handler
        .handle_tool("debugger_start", args)
        .await;

    assert!(result.is_ok(), "Rust session with args failed: {:?}", result.err());
}

/// Test full debugging workflow: FizzBuzz bug detection
#[tokio::test]
#[ignore] // Requires Docker with full debugging environment
async fn test_rust_fizzbuzz_debugging_workflow() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager.clone());

    // Step 1: Start debug session with stopOnEntry
    let start_args = json!({
        "language": "rust",
        "program": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
        "stopOnEntry": true
    });

    let start_result = tools_handler
        .handle_tool("debugger_start", start_args)
        .await
        .expect("Failed to start session");

    let session_id = start_result["sessionId"]
        .as_str()
        .expect("No sessionId in response");

    // Step 2: Set breakpoint at line 9 (the buggy line: n % 4 instead of n % 5)
    let bp_args = json!({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
        "line": 9
    });

    let bp_result = tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await
        .expect("Failed to set breakpoint");

    assert_eq!(bp_result["verified"], true, "Breakpoint should be verified");

    // Step 3: Continue execution
    let continue_args = json!({
        "sessionId": session_id
    });

    tools_handler
        .handle_tool("debugger_continue", continue_args.clone())
        .await
        .expect("Failed to continue");

    // Step 4: Wait for breakpoint hit
    let wait_args = json!({
        "sessionId": session_id,
        "timeout": 5000
    });

    let stop_result = tools_handler
        .handle_tool("debugger_wait_for_stop", wait_args)
        .await
        .expect("Failed to wait for stop");

    assert_eq!(stop_result["reason"], "breakpoint", "Should stop at breakpoint");

    // Step 5: Evaluate the bug - check what n is
    let eval_n_args = json!({
        "sessionId": session_id,
        "expression": "n"
    });

    let eval_n_result = tools_handler
        .handle_tool("debugger_evaluate", eval_n_args)
        .await
        .expect("Failed to evaluate n");

    let n_value = eval_n_result["result"]
        .as_str()
        .expect("No result in evaluation");

    // n should be 4 (first number divisible by 4)
    assert_eq!(n_value, "4", "First breakpoint hit should be at n=4");

    // Step 6: Evaluate the buggy condition
    let eval_mod4_args = json!({
        "sessionId": session_id,
        "expression": "n % 4"
    });

    let eval_mod4_result = tools_handler
        .handle_tool("debugger_evaluate", eval_mod4_args)
        .await
        .expect("Failed to evaluate n % 4");

    assert_eq!(eval_mod4_result["result"], "0", "n % 4 should be 0");

    // Step 7: Evaluate what it SHOULD be
    let eval_mod5_args = json!({
        "sessionId": session_id,
        "expression": "n % 5"
    });

    let eval_mod5_result = tools_handler
        .handle_tool("debugger_evaluate", eval_mod5_args)
        .await
        .expect("Failed to evaluate n % 5");

    assert_eq!(eval_mod5_result["result"], "4", "n % 5 should be 4 (not 0)");

    // Bug confirmed: Code checks n % 4 == 0 when it should check n % 5 == 0

    // Step 8: Disconnect cleanly
    let disconnect_args = json!({
        "sessionId": session_id
    });

    tools_handler
        .handle_tool("debugger_disconnect", disconnect_args)
        .await
        .expect("Failed to disconnect");
}
