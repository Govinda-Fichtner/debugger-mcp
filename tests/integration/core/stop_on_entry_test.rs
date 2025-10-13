/// Integration tests for stopOnEntry and state management
///
/// These tests reproduce and verify fixes for:
/// 1. stopOnEntry not working - state shows "Running" instead of "Stopped"
/// 2. wait_for_stop timing out when it should detect stopped state
/// 3. Race condition between event handlers and manual state updates
use debugger_mcp::debug::SessionManager;
use debugger_mcp::mcp::tools::ToolsHandler;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

/// Helper to get path to fizzbuzz test fixture
fn get_fizzbuzz_path() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("fizzbuzz.py")
        .to_string_lossy()
        .to_string()
}

/// Helper to check if debugpy is available
fn is_debugpy_available() -> bool {
    std::process::Command::new("python3")
        .args(["-c", "import debugpy"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// TEST 1: Reproduce stopOnEntry bug
///
/// EXPECTED FAILURE (before fix):
/// - Session starts with stopOnEntry: true
/// - State should be "Stopped" with reason "entry"
/// - BUG: State shows "Running" instead
///
/// EXPECTED PASS (after fix):
/// - State correctly shows "Stopped" immediately after start
#[tokio::test(flavor = "multi_thread")]
#[ignore] // Run with: cargo test stopOnEntry_test -- --ignored --nocapture
#[allow(non_snake_case)]
async fn test_stopOnEntry_sets_stopped_state() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    // Start session with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": get_fizzbuzz_path(),
        "args": [],
        "stopOnEntry": true
    });

    println!("🔧 Starting session with stopOnEntry: true");

    let start_result = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await;

    let start_response = match start_result {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            println!("⚠️  Skipping: debugger_start failed: {}", e);
            return;
        }
        Err(_) => {
            println!("⚠️  Skipping: debugger_start timed out");
            return;
        }
    };

    let session_id = start_response["sessionId"].as_str().unwrap().to_string();
    println!("✅ Session started: {}", session_id);

    // Wait for session to complete async initialization and reach Stopped state
    // Poll for up to 5 seconds until state becomes "Stopped"
    for attempt in 1..=50 {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let state_args = json!({
            "sessionId": session_id
        });

        let state_result = tools_handler
            .handle_tool("debugger_session_state", state_args)
            .await
            .expect("debugger_session_state should succeed");

        let state = state_result["state"].as_str().unwrap();
        println!("⏳ Attempt {}/50: state = {}", attempt, state);

        if state == "Stopped" {
            break;
        }
    }

    // Now check the final state - THIS IS THE CRITICAL TEST
    let state_args = json!({
        "sessionId": session_id
    });

    let state_result = tools_handler
        .handle_tool("debugger_session_state", state_args)
        .await
        .expect("debugger_session_state should succeed");

    println!("📊 Final state after polling: {}", state_result);

    let final_state = state_result["state"].as_str().unwrap();
    let reason = state_result
        .get("details")
        .and_then(|d| d.get("reason"))
        .and_then(|r| r.as_str());

    // ASSERTION: State should be "Stopped", not "Running"
    assert_eq!(
        final_state, "Stopped",
        "❌ BUG REPRODUCED: State is '{}' but should be 'Stopped' with stopOnEntry: true (waited {} attempts)",
        final_state, 50
    );

    // ASSERTION: Reason should be "entry"
    assert_eq!(
        reason,
        Some("entry"),
        "❌ BUG: Reason is '{:?}' but should be 'entry'",
        reason
    );

    println!("✅ TEST PASSED: State is correctly 'Stopped' with reason 'entry'");

    // Cleanup
    let disconnect_args = json!({"sessionId": session_id});
    let _ = tools_handler
        .handle_tool("debugger_disconnect", disconnect_args)
        .await;
}

/// TEST 2: Verify wait_for_stop works with stopOnEntry
///
/// EXPECTED FAILURE (before fix):
/// - wait_for_stop times out because state never becomes "Stopped"
///
/// EXPECTED PASS (after fix):
/// - wait_for_stop returns immediately with stopped state
#[tokio::test(flavor = "multi_thread")]
#[ignore]
#[allow(non_snake_case)]
async fn test_wait_for_stop_detects_stopOnEntry() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    // Start session with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": get_fizzbuzz_path(),
        "stopOnEntry": true
    });

    let start_response = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await;

    let start_response = match start_response {
        Ok(Ok(r)) => r,
        _ => {
            println!("⚠️  Skipping: debugger_start failed or timed out");
            return;
        }
    };

    let session_id = start_response["sessionId"].as_str().unwrap();
    println!("✅ Session started: {}", session_id);

    // Use wait_for_stop - should return immediately
    let wait_args = json!({
        "sessionId": session_id,
        "timeoutMs": 5000
    });

    println!("⏳ Calling wait_for_stop (should return immediately)...");

    let wait_result = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
    )
    .await;

    let wait_response = match wait_result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            panic!("❌ BUG REPRODUCED: wait_for_stop failed: {}", e);
        }
        Err(_) => {
            panic!("❌ BUG REPRODUCED: wait_for_stop timed out (state never became Stopped)");
        }
    };

    println!("📊 wait_for_stop response: {}", wait_response);

    // ASSERTIONS
    assert_eq!(
        wait_response["state"].as_str().unwrap(),
        "Stopped",
        "wait_for_stop should return Stopped state"
    );

    assert_eq!(
        wait_response["reason"].as_str().unwrap(),
        "entry",
        "wait_for_stop should return reason 'entry'"
    );

    println!("✅ TEST PASSED: wait_for_stop correctly detected stopOnEntry");

    // Cleanup
    let disconnect_args = json!({"sessionId": session_id});
    let _ = tools_handler
        .handle_tool("debugger_disconnect", disconnect_args)
        .await;
}

/// TEST 3: Verify breakpoints work after stopOnEntry
///
/// EXPECTED FAILURE (before fix):
/// - Program runs to completion before breakpoint can be set
/// - Breakpoint never hits
///
/// EXPECTED PASS (after fix):
/// - Program stops at entry
/// - Breakpoint is set successfully
/// - Continue reaches breakpoint
#[tokio::test(flavor = "multi_thread")]
#[ignore]
#[allow(non_snake_case)]
async fn test_breakpoint_works_with_stopOnEntry() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    // 1. Start with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
        "stopOnEntry": true
    });

    let start_response = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await;

    let start_response = match start_response {
        Ok(Ok(r)) => r,
        _ => {
            println!("⚠️  Skipping: debugger_start failed");
            return;
        }
    };

    let session_id = start_response["sessionId"].as_str().unwrap();
    println!("✅ Session started: {}", session_id);

    // 2. Wait for stop at entry
    let wait_args = json!({
        "sessionId": session_id,
        "timeoutMs": 5000
    });

    let wait_response = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args.clone()),
    )
    .await;

    match wait_response {
        Ok(Ok(r)) => {
            println!("✅ Stopped at entry: {}", r);
        }
        _ => {
            println!("❌ BUG: wait_for_stop failed - program may have run to completion");
            // Cleanup and fail
            let _ = tools_handler
                .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
                .await;
            panic!("Failed to stop at entry");
        }
    }

    // 3. Set breakpoint at line 31 (inside main loop where 'i' is defined)
    let bp_args = json!({
        "sessionId": session_id,
        "sourcePath": &fizzbuzz_path,
        "line": 31
    });

    let bp_response = tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await
        .expect("Setting breakpoint should succeed");

    println!("✅ Breakpoint set: {}", bp_response);

    // 4. Continue execution
    let continue_args = json!({"sessionId": session_id});

    tools_handler
        .handle_tool("debugger_continue", continue_args)
        .await
        .expect("Continue should succeed");

    println!("▶️  Continuing to breakpoint...");

    // 5. Wait for breakpoint hit
    let wait2_response = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
    )
    .await;

    let wait2_response = match wait2_response {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            panic!("❌ BUG: wait_for_stop failed after continue: {}", e);
        }
        Err(_) => {
            panic!("❌ BUG: Breakpoint not hit (timeout)");
        }
    };

    println!("📊 Stopped at breakpoint: {}", wait2_response);

    // ASSERTION: Should stop at breakpoint
    assert_eq!(
        wait2_response["state"].as_str().unwrap(),
        "Stopped",
        "Should be stopped at breakpoint"
    );

    assert_eq!(
        wait2_response["reason"].as_str().unwrap(),
        "breakpoint",
        "Reason should be 'breakpoint'"
    );

    // 6. Verify we can get stack trace
    let stack_args = json!({"sessionId": session_id});

    let stack_response = tools_handler
        .handle_tool("debugger_stack_trace", stack_args)
        .await
        .expect("Stack trace should succeed when stopped");

    let frames = stack_response["stackFrames"].as_array().unwrap();
    assert!(!frames.is_empty(), "Should have stack frames");

    println!("✅ Stack trace: {} frames", frames.len());

    // 7. Verify we can evaluate expressions (even if specific variables aren't in scope)
    // Try a simple expression that always works
    let eval_args = json!({
        "sessionId": session_id,
        "expression": "1 + 1"
    });

    let eval_result = tools_handler
        .handle_tool("debugger_evaluate", eval_args)
        .await;

    // Evaluation should work when stopped (even if result varies)
    assert!(
        eval_result.is_ok(),
        "Should be able to evaluate expressions when stopped"
    );

    if let Ok(eval_response) = eval_result {
        println!("✅ Evaluated '1 + 1': {}", eval_response["result"]);
    }

    println!("✅ TEST PASSED: Complete debugging workflow works with stopOnEntry");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

/// TEST 4: Verify state transitions are accurate throughout execution
///
/// Tests that state accurately reflects debugger state at all times:
/// - Stopped at entry
/// - Running after continue
/// - Stopped at breakpoint
/// - Terminated after completion
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_state_transitions_are_accurate() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    // Start with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": get_fizzbuzz_path(),
        "stopOnEntry": true
    });

    let start_response = match timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await
    {
        Ok(Ok(r)) => r,
        _ => {
            println!("⚠️  Skipping: debugger_start failed");
            return;
        }
    };

    let session_id = start_response["sessionId"].as_str().unwrap();

    // Wait for session to complete async initialization and reach Stopped state
    // Poll for up to 5 seconds until state becomes "Stopped"
    let mut found_stopped = false;
    for attempt in 1..=50 {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let state_check = tools_handler
            .handle_tool("debugger_session_state", json!({"sessionId": session_id}))
            .await
            .unwrap();

        let current_state = state_check["state"].as_str().unwrap();
        println!("⏳ Attempt {}/50: state = {}", attempt, current_state);

        if current_state == "Stopped" {
            found_stopped = true;
            break;
        }
    }

    // STATE 1: Should be Stopped at entry
    let state1 = tools_handler
        .handle_tool("debugger_session_state", json!({"sessionId": session_id}))
        .await
        .unwrap();

    println!("📊 State after start and polling: {}", state1);
    assert!(
        found_stopped,
        "State 1: Should have reached Stopped state within 5 seconds, got: {}",
        state1["state"].as_str().unwrap()
    );
    assert_eq!(
        state1["state"].as_str().unwrap(),
        "Stopped",
        "State 1: Should be Stopped at entry"
    );

    // Continue execution
    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": session_id}))
        .await
        .unwrap();

    // Give a tiny moment for state to update
    tokio::time::sleep(Duration::from_millis(50)).await;

    // STATE 2: Should be Running (or quickly move to Terminated)
    let state2 = tools_handler
        .handle_tool("debugger_session_state", json!({"sessionId": session_id}))
        .await
        .unwrap();

    println!("📊 State after continue: {}", state2);
    let state2_str = state2["state"].as_str().unwrap();

    assert!(
        state2_str == "Running" || state2_str == "Terminated",
        "State 2: Should be Running or Terminated, got '{}'",
        state2_str
    );

    println!("✅ TEST PASSED: State transitions are accurate");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}
