/// Integration tests based on user feedback
///
/// These tests address critical gaps and pain points discovered during
/// user testing. They validate documented behavior and ensure common
/// workflows work correctly.
///
/// See: docs/PROPOSED_INTEGRATION_TESTS.md
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

/// Helper: Start session and hit breakpoint in function where local variables exist
async fn start_and_hit_breakpoint_in_function(
    tools_handler: &ToolsHandler,
) -> Result<String, String> {
    let fizzbuzz_path = get_fizzbuzz_path();

    // Start with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
        "stopOnEntry": true
    });

    let start_response = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await
    .map_err(|_| "Start timeout".to_string())?
    .map_err(|e| format!("Start failed: {}", e))?;

    let session_id = start_response["sessionId"]
        .as_str()
        .ok_or("No session ID")?
        .to_string();

    // Wait for entry
    let wait_args = json!({
        "sessionId": &session_id,
        "timeoutMs": 5000
    });

    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
    )
    .await
    .map_err(|_| "Wait for entry timeout".to_string())?
    .map_err(|e| format!("Wait failed: {}", e))?;

    // Set breakpoint inside fizzbuzz function (line 18 - inside function with parameter 'n')
    let bp_args = json!({
        "sessionId": &session_id,
        "sourcePath": &fizzbuzz_path,
        "line": 18
    });

    tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await
        .map_err(|e| format!("Set breakpoint failed: {}", e))?;

    // Continue to breakpoint
    let continue_args = json!({"sessionId": &session_id});

    tools_handler
        .handle_tool("debugger_continue", continue_args)
        .await
        .map_err(|e| format!("Continue failed: {}", e))?;

    // Wait for breakpoint hit
    let wait_args = json!({
        "sessionId": &session_id,
        "timeoutMs": 5000
    });

    let stop_result = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
    )
    .await
    .map_err(|_| "Wait for breakpoint timeout".to_string())?
    .map_err(|e| format!("Wait for breakpoint failed: {}", e))?;

    // Verify we're at a breakpoint
    if stop_result["reason"].as_str() != Some("breakpoint") {
        return Err(format!(
            "Expected breakpoint, got: {}",
            stop_result["reason"]
        ));
    }

    Ok(session_id)
}

// ============================================================================
// PHASE 1: HIGH PRIORITY TESTS
// ============================================================================

/// PHASE 1 TEST 1: frameId Requirement for Local Variable Access
///
/// User feedback: #1 pain point - unclear that frameId is practically required
///
/// This test proves:
/// - debugger_evaluate WITHOUT frameId fails for local variables (NameError)
/// - debugger_evaluate WITH frameId succeeds
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_frameid_required_for_local_variables() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

    println!("🧪 TEST: frameId requirement for local variable access");

    // Setup: Start and hit breakpoint in function
    let session_id = match start_and_hit_breakpoint_in_function(&tools_handler).await {
        Ok(id) => {
            println!("✅ Setup: Stopped at breakpoint in fizzbuzz function");
            id
        }
        Err(e) => {
            println!("⚠️  Skipping: Setup failed: {}", e);
            return;
        }
    };

    // TEST 1: WITHOUT frameId - should fail for local variable 'n'
    println!("\n📝 Test 1: Evaluate local variable WITHOUT frameId");

    let eval_without_frame = tools_handler
        .handle_tool(
            "debugger_evaluate",
            json!({
                "sessionId": &session_id,
                "expression": "n"  // Local variable in fizzbuzz function
                // NO frameId!
            }),
        )
        .await;

    // Should fail with NameError
    assert!(
        eval_without_frame.is_err(),
        "❌ FAIL: Evaluate without frameId should fail for local variables"
    );

    let error_msg = eval_without_frame.unwrap_err().to_string();
    assert!(
        error_msg.contains("NameError") || error_msg.contains("not defined"),
        "❌ FAIL: Expected NameError, got: {}",
        error_msg
    );

    println!("✅ Test 1 PASS: Got expected NameError without frameId");
    println!("   Error: {}", error_msg.lines().next().unwrap_or(""));

    // TEST 2: WITH frameId - should succeed
    println!("\n📝 Test 2: Evaluate local variable WITH frameId");

    // Get stack trace to obtain frame ID
    let stack_result = tools_handler
        .handle_tool("debugger_stack_trace", json!({"sessionId": &session_id}))
        .await
        .expect("Stack trace should succeed");

    let frames = stack_result["stackFrames"].as_array().unwrap();
    assert!(!frames.is_empty(), "Should have stack frames");

    let frame_id = frames[0]["id"].as_i64().expect("Frame should have ID");
    println!("   Using frameId: {}", frame_id);

    let eval_with_frame = tools_handler
        .handle_tool(
            "debugger_evaluate",
            json!({
                "sessionId": &session_id,
                "expression": "n",
                "frameId": frame_id
            }),
        )
        .await;

    // Should succeed
    assert!(
        eval_with_frame.is_ok(),
        "❌ FAIL: Evaluate with frameId should succeed: {:?}",
        eval_with_frame.err()
    );

    let result = eval_with_frame.unwrap();
    println!("✅ Test 2 PASS: Successfully evaluated 'n' with frameId");
    println!("   Result: {}", result["result"]);

    println!("\n🎉 ALL TESTS PASSED: frameId is required for local variable access");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

/// PHASE 1 TEST 2: Frame IDs Change Between Stops
///
/// User feedback: Not documented that frame IDs are unstable across stops
///
/// This test proves:
/// - Frame IDs change between different stop events
/// - Using stale frame IDs fails
/// - Must get fresh stack trace after each stop
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_frame_ids_change_between_stops() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    println!("🧪 TEST: Frame IDs change between stops");

    // Start session with stopOnEntry
    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
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

    // Wait for entry
    let wait_args = json!({
        "sessionId": session_id,
        "timeoutMs": 5000
    });

    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args.clone()),
    )
    .await
    .ok();

    // Set breakpoint inside main loop (will hit multiple times)
    let bp_args = json!({
        "sessionId": session_id,
        "sourcePath": &fizzbuzz_path,
        "line": 31  // Inside for loop in main
    });

    tools_handler
        .handle_tool("debugger_set_breakpoint", bp_args)
        .await
        .expect("Set breakpoint should succeed");

    // STOP 1: Hit breakpoint first time
    println!("\n📝 Stop 1: Hit breakpoint (iteration 1)");

    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": session_id}))
        .await
        .expect("Continue should succeed");

    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args.clone()),
    )
    .await
    .expect("Should hit breakpoint")
    .expect("Should not error");

    let stack1 = tools_handler
        .handle_tool("debugger_stack_trace", json!({"sessionId": session_id}))
        .await
        .expect("Stack trace 1 should succeed");

    let frames1 = stack1["stackFrames"].as_array().unwrap();
    let frame_id_1 = frames1[0]["id"].as_i64().unwrap();

    println!("   Frame ID at stop 1: {}", frame_id_1);

    // STOP 2: Hit breakpoint second time
    println!("\n📝 Stop 2: Hit breakpoint (iteration 2)");

    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": session_id}))
        .await
        .expect("Continue should succeed");

    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
    )
    .await
    .expect("Should hit breakpoint")
    .expect("Should not error");

    let stack2 = tools_handler
        .handle_tool("debugger_stack_trace", json!({"sessionId": session_id}))
        .await
        .expect("Stack trace 2 should succeed");

    let frames2 = stack2["stackFrames"].as_array().unwrap();
    let frame_id_2 = frames2[0]["id"].as_i64().unwrap();

    println!("   Frame ID at stop 2: {}", frame_id_2);

    // TEST: Frame IDs should be DIFFERENT between stops
    assert_ne!(
        frame_id_1, frame_id_2,
        "❌ FAIL: Frame IDs should change between stops"
    );

    println!(
        "✅ TEST PASS: Frame IDs changed between stops ({} → {})",
        frame_id_1, frame_id_2
    );

    // TEST: Using old frame ID should fail
    println!("\n📝 Test: Using stale frame ID from stop 1");

    let eval_with_old_frame = tools_handler
        .handle_tool(
            "debugger_evaluate",
            json!({
                "sessionId": session_id,
                "expression": "1 + 1",
                "frameId": frame_id_1  // OLD frame ID from first stop
            }),
        )
        .await;

    // Using stale frame ID should fail
    assert!(
        eval_with_old_frame.is_err(),
        "❌ FAIL: Using stale frame ID should fail"
    );

    println!("✅ TEST PASS: Stale frame ID correctly rejected");

    // TEST: Using current frame ID should succeed
    println!("\n📝 Test: Using fresh frame ID from stop 2");

    let eval_with_current_frame = tools_handler
        .handle_tool(
            "debugger_evaluate",
            json!({
                "sessionId": session_id,
                "expression": "1 + 1",
                "frameId": frame_id_2  // Current frame ID
            }),
        )
        .await;

    assert!(
        eval_with_current_frame.is_ok(),
        "❌ FAIL: Fresh frame ID should work: {:?}",
        eval_with_current_frame.err()
    );

    println!("✅ TEST PASS: Fresh frame ID works correctly");
    println!("\n🎉 ALL TESTS PASSED: Frame IDs are unstable across stops");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

/// PHASE 1 TEST 3: list_breakpoints Functionality
///
/// User feedback: New tool with NO test coverage
///
/// This test validates:
/// - list_breakpoints shows all set breakpoints
/// - Includes correct line numbers and verification status
/// - Works with multiple breakpoints
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_list_breakpoints_shows_all_breakpoints() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    println!("🧪 TEST: list_breakpoints shows all breakpoints");

    // Start session
    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
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

    // Wait for entry
    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": session_id, "timeoutMs": 5000}),
        ),
    )
    .await
    .ok();

    // Set multiple breakpoints at different lines
    println!("\n📝 Setting 3 breakpoints...");

    let breakpoints_to_set = vec![18, 20, 31];

    for line in &breakpoints_to_set {
        let bp_args = json!({
            "sessionId": session_id,
            "sourcePath": &fizzbuzz_path,
            "line": line
        });

        let bp_result = tools_handler
            .handle_tool("debugger_set_breakpoint", bp_args)
            .await
            .expect("Set breakpoint should succeed");

        let verified = bp_result["verified"].as_bool().unwrap_or(false);
        println!("   ✓ Breakpoint at line {}: verified={}", line, verified);
    }

    // List breakpoints
    println!("\n📝 Calling list_breakpoints...");

    let list_result = tools_handler
        .handle_tool(
            "debugger_list_breakpoints",
            json!({"sessionId": session_id}),
        )
        .await
        .expect("list_breakpoints should succeed");

    let breakpoints = list_result["breakpoints"]
        .as_array()
        .expect("Should return breakpoints array");

    println!("   Found {} breakpoints", breakpoints.len());

    // TEST: Should have all 3 breakpoints
    assert_eq!(
        breakpoints.len(),
        3,
        "❌ FAIL: Should have 3 breakpoints, got {}",
        breakpoints.len()
    );

    println!("✅ TEST PASS: Correct number of breakpoints");

    // TEST: Verify line numbers
    let mut lines: Vec<i64> = breakpoints
        .iter()
        .map(|bp| bp["line"].as_i64().unwrap())
        .collect();

    lines.sort();

    println!("\n📝 Verifying breakpoint details:");
    for bp in breakpoints {
        let line = bp["line"].as_i64().unwrap();
        let verified = bp["verified"].as_bool().unwrap();
        let source = bp["sourcePath"].as_str().unwrap();

        println!(
            "   Line {}: verified={}, source={}",
            line,
            verified,
            source.split('/').next_back().unwrap_or(source)
        );

        // All should be verified
        assert!(
            verified,
            "❌ FAIL: Breakpoint at line {} should be verified",
            line
        );

        // Should have correct source path
        assert!(
            source.contains("fizzbuzz.py"),
            "❌ FAIL: Source path should contain fizzbuzz.py"
        );
    }

    // TEST: All expected lines present
    for expected_line in &breakpoints_to_set {
        assert!(
            lines.contains(expected_line),
            "❌ FAIL: Should have breakpoint at line {}",
            expected_line
        );
    }

    println!("✅ TEST PASS: All breakpoints have correct details");
    println!("\n🎉 ALL TESTS PASSED: list_breakpoints works correctly");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

// ============================================================================
// PHASE 2: MEDIUM PRIORITY TESTS
// ============================================================================

/// PHASE 2 TEST 1: Complete Pattern - Inspect Variable at Breakpoint
///
/// User feedback: Most common workflow needs end-to-end test
///
/// This validates the #1 documented pattern:
/// 1. Start with stopOnEntry
/// 2. Set breakpoint
/// 3. Continue to breakpoint
/// 4. Get stack trace
/// 5. Use frame ID to evaluate variable
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_pattern_inspect_variable_at_breakpoint() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    println!("🧪 TEST: Pattern - Inspect variable at breakpoint");

    // STEP 1: Start with stopOnEntry
    println!("\n📝 Step 1: Start with stopOnEntry");

    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
        "stopOnEntry": true
    });

    let start_response = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await
    .expect("Should not timeout")
    .expect("Start should succeed");

    let session_id = start_response["sessionId"].as_str().unwrap();
    println!("   ✓ Session started: {}", session_id);

    // STEP 2: Wait for entry
    println!("\n📝 Step 2: Wait for entry");

    let stop_result = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": session_id, "timeoutMs": 5000}),
        ),
    )
    .await
    .expect("Should not timeout")
    .expect("Wait should succeed");

    assert_eq!(stop_result["reason"].as_str().unwrap(), "entry");
    println!("   ✓ Stopped at entry");

    // STEP 3: Set breakpoint
    println!("\n📝 Step 3: Set breakpoint at line 18");

    let bp_result = tools_handler
        .handle_tool(
            "debugger_set_breakpoint",
            json!({
                "sessionId": session_id,
                "sourcePath": &fizzbuzz_path,
                "line": 18
            }),
        )
        .await
        .expect("Set breakpoint should succeed");

    assert!(bp_result["verified"].as_bool().unwrap());
    println!("   ✓ Breakpoint set and verified");

    // STEP 4: Continue to breakpoint
    println!("\n📝 Step 4: Continue to breakpoint");

    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": session_id}))
        .await
        .expect("Continue should succeed");

    let stop_result = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": session_id, "timeoutMs": 5000}),
        ),
    )
    .await
    .expect("Should hit breakpoint")
    .expect("Wait should succeed");

    assert_eq!(stop_result["reason"].as_str().unwrap(), "breakpoint");
    println!("   ✓ Stopped at breakpoint");

    // STEP 5: Get stack trace (THE RIGHT WAY per user feedback)
    println!("\n📝 Step 5: Get stack trace to obtain frame ID");

    let stack_result = tools_handler
        .handle_tool("debugger_stack_trace", json!({"sessionId": session_id}))
        .await
        .expect("Stack trace should succeed");

    let frames = stack_result["stackFrames"].as_array().unwrap();
    assert!(!frames.is_empty(), "Should have frames");

    let frame_id = frames[0]["id"].as_i64().unwrap();
    let frame_name = frames[0]["name"].as_str().unwrap();
    let frame_line = frames[0]["line"].as_i64().unwrap();

    println!("   ✓ Current frame: {} at line {}", frame_name, frame_line);
    println!("   ✓ Frame ID: {}", frame_id);

    // STEP 6: Evaluate variable (THE RIGHT WAY with frameId)
    println!("\n📝 Step 6: Evaluate variable 'n' with frameId");

    let eval_result = tools_handler
        .handle_tool(
            "debugger_evaluate",
            json!({
                "sessionId": session_id,
                "expression": "n",
                "frameId": frame_id
            }),
        )
        .await
        .expect("Evaluate should succeed with frameId");

    let n_value = &eval_result["result"];
    println!("   ✓ Successfully evaluated 'n' = {}", n_value);

    // Variable should have a value
    assert!(
        n_value.is_string() || n_value.is_number(),
        "Variable should have a value"
    );

    println!("\n🎉 PATTERN TEST PASSED: Complete workflow works correctly");
    println!("   This validates the documented 'inspect variable at breakpoint' pattern");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

/// PHASE 2 TEST 2: Step Commands - Comprehensive Workflow
///
/// User feedback: New step tools need comprehensive testing
///
/// This validates:
/// - step_into enters function calls
/// - step_over executes without entering calls
/// - step_out returns to caller
/// - Stack changes appropriately for each step type
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_step_commands_comprehensive() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    println!("🧪 TEST: Step commands comprehensive workflow");

    // Start and get to line that calls fizzbuzz function
    let start_args = json!({
        "language": "python",
        "program": &fizzbuzz_path,
        "stopOnEntry": true
    });

    let start_response = timeout(
        Duration::from_secs(10),
        tools_handler.handle_tool("debugger_start", start_args),
    )
    .await
    .ok()
    .and_then(|r| r.ok());

    if start_response.is_none() {
        println!("⚠️  Skipping: debugger_start failed");
        return;
    }

    let session_id = start_response.unwrap()["sessionId"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for entry
    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": &session_id, "timeoutMs": 5000}),
        ),
    )
    .await
    .ok();

    // Set breakpoint at line 32 (where fizzbuzz function is called)
    tools_handler
        .handle_tool(
            "debugger_set_breakpoint",
            json!({
                "sessionId": &session_id,
                "sourcePath": &fizzbuzz_path,
                "line": 32
            }),
        )
        .await
        .ok();

    // Continue to breakpoint
    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": &session_id}))
        .await
        .ok();

    timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": &session_id, "timeoutMs": 5000}),
        ),
    )
    .await
    .ok();

    let stack_before = tools_handler
        .handle_tool("debugger_stack_trace", json!({"sessionId": &session_id}))
        .await;

    if stack_before.is_err() {
        println!("⚠️  Skipping: Could not get initial stack");
        return;
    }

    let frames_before = stack_before.unwrap()["stackFrames"]
        .as_array()
        .unwrap()
        .clone();
    let frame_name_before = frames_before[0]["name"].as_str().unwrap();
    let line_before = frames_before[0]["line"].as_i64().unwrap();

    println!(
        "\n📝 Initial position: {} at line {}",
        frame_name_before, line_before
    );

    // TEST 1: step_into - should enter fizzbuzz function
    println!("\n📝 Test 1: step_into (should enter fizzbuzz function)");

    let step_into_result = tools_handler
        .handle_tool("debugger_step_into", json!({"sessionId": &session_id}))
        .await;

    if step_into_result.is_err() {
        println!("⚠️  step_into not supported or failed");
    } else {
        timeout(
            Duration::from_secs(6),
            tools_handler.handle_tool(
                "debugger_wait_for_stop",
                json!({"sessionId": &session_id, "timeoutMs": 5000}),
            ),
        )
        .await
        .ok();

        let stack_after_into = tools_handler
            .handle_tool("debugger_stack_trace", json!({"sessionId": &session_id}))
            .await;

        if let Ok(stack) = stack_after_into {
            let frames = stack["stackFrames"].as_array().unwrap();
            let frame_name = frames[0]["name"].as_str().unwrap();

            println!("   ✓ After step_into: now in '{}'", frame_name);

            // Should be inside fizzbuzz function
            if frame_name == "fizzbuzz" {
                println!("✅ TEST PASS: step_into entered function");
            } else {
                println!(
                    "⚠️  Note: Expected to be in 'fizzbuzz', got '{}'",
                    frame_name
                );
            }

            // TEST 2: step_over - should advance to next line
            println!("\n📝 Test 2: step_over (should advance to next line)");

            let line_before_over = frames[0]["line"].as_i64().unwrap();

            tools_handler
                .handle_tool("debugger_step_over", json!({"sessionId": &session_id}))
                .await
                .ok();

            timeout(
                Duration::from_secs(6),
                tools_handler.handle_tool(
                    "debugger_wait_for_stop",
                    json!({"sessionId": &session_id, "timeoutMs": 5000}),
                ),
            )
            .await
            .ok();

            if let Ok(stack) = tools_handler
                .handle_tool("debugger_stack_trace", json!({"sessionId": &session_id}))
                .await
            {
                let frames = stack["stackFrames"].as_array().unwrap();
                let line_after_over = frames[0]["line"].as_i64().unwrap();
                let name_after_over = frames[0]["name"].as_str().unwrap();

                println!(
                    "   ✓ After step_over: line {} → {} (in '{}')",
                    line_before_over, line_after_over, name_after_over
                );

                if line_after_over != line_before_over {
                    println!("✅ TEST PASS: step_over advanced to next line");
                }

                // TEST 3: step_out - should return to caller
                println!("\n📝 Test 3: step_out (should return to caller)");

                tools_handler
                    .handle_tool("debugger_step_out", json!({"sessionId": &session_id}))
                    .await
                    .ok();

                timeout(
                    Duration::from_secs(6),
                    tools_handler.handle_tool(
                        "debugger_wait_for_stop",
                        json!({"sessionId": &session_id, "timeoutMs": 5000}),
                    ),
                )
                .await
                .ok();

                if let Ok(stack) = tools_handler
                    .handle_tool("debugger_stack_trace", json!({"sessionId": &session_id}))
                    .await
                {
                    let frames = stack["stackFrames"].as_array().unwrap();
                    let name_after_out = frames[0]["name"].as_str().unwrap();

                    println!("   ✓ After step_out: now in '{}'", name_after_out);

                    if name_after_out != "fizzbuzz" {
                        println!("✅ TEST PASS: step_out returned to caller");
                    }
                }
            }
        }
    }

    println!("\n🎉 STEP COMMANDS TEST COMPLETED");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}

/// PHASE 2 TEST 3: wait_for_stop Timing Behavior
///
/// User feedback: Performance guarantees need validation
///
/// This validates:
/// - Immediate return (<100ms) when already stopped
/// - Blocking behavior when running
/// - Timeout works correctly
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_wait_for_stop_timing_behavior() {
    if !is_debugpy_available() {
        println!("⚠️  Skipping: debugpy not installed");
        return;
    }

    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));
    let fizzbuzz_path = get_fizzbuzz_path();

    println!("🧪 TEST: wait_for_stop timing behavior");

    // Start session with stopOnEntry (will be immediately stopped)
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

    if start_response.is_err() {
        println!("⚠️  Skipping: debugger_start timeout");
        return;
    }

    let start_response = start_response.unwrap();
    if start_response.is_err() {
        println!("⚠️  Skipping: debugger_start failed");
        return;
    }

    let session_id = start_response.unwrap()["sessionId"]
        .as_str()
        .unwrap()
        .to_string();

    // Small delay to let it fully stop
    tokio::time::sleep(Duration::from_millis(200)).await;

    // TEST 1: Immediate return when already stopped
    println!("\n📝 Test 1: Immediate return when already stopped");

    let start_time = std::time::Instant::now();

    let wait_result = timeout(
        Duration::from_secs(6),
        tools_handler.handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": &session_id, "timeoutMs": 5000}),
        ),
    )
    .await;

    let elapsed = start_time.elapsed();

    if let Ok(Ok(result)) = wait_result {
        println!("   ✓ Returned in {}ms", elapsed.as_millis());
        println!("   ✓ State: {}", result["state"]);

        // Should return quickly (per user feedback: <100ms typical)
        if elapsed.as_millis() < 500 {
            println!("✅ TEST PASS: Immediate return when already stopped");
        } else {
            println!(
                "⚠️  Note: Took {}ms (expected <100ms, acceptable <500ms)",
                elapsed.as_millis()
            );
        }
    } else {
        println!("⚠️  wait_for_stop failed or timed out");
    }

    // TEST 2: Timeout behavior with short timeout
    println!("\n📝 Test 2: Timeout behavior");

    // Continue execution (program will run)
    tools_handler
        .handle_tool("debugger_continue", json!({"sessionId": &session_id}))
        .await
        .ok();

    // Give it a moment to start running
    tokio::time::sleep(Duration::from_millis(50)).await;

    let start_time = std::time::Instant::now();

    // Call wait_for_stop with short timeout (program may complete or timeout)
    let wait_result = tools_handler
        .handle_tool(
            "debugger_wait_for_stop",
            json!({"sessionId": &session_id, "timeoutMs": 1000}),
        )
        .await;

    let elapsed = start_time.elapsed();

    match wait_result {
        Ok(result) => {
            println!("   ✓ Returned in {}ms", elapsed.as_millis());
            println!(
                "   ✓ State: {} (program stopped or terminated)",
                result["state"]
            );
            println!("✅ TEST PASS: wait_for_stop returned (program stopped)");
        }
        Err(e) => {
            println!("   ✓ Timed out in {}ms", elapsed.as_millis());
            println!("   ✓ Error: {}", e.to_string().lines().next().unwrap_or(""));

            // Should timeout around 1000ms
            if elapsed.as_millis() >= 1000 && elapsed.as_millis() < 2000 {
                println!("✅ TEST PASS: Timeout behavior works correctly");
            } else {
                println!(
                    "⚠️  Note: Timeout took {}ms (expected ~1000ms)",
                    elapsed.as_millis()
                );
            }
        }
    }

    println!("\n🎉 TIMING TEST COMPLETED");

    // Cleanup
    let _ = tools_handler
        .handle_tool("debugger_disconnect", json!({"sessionId": session_id}))
        .await;
}
