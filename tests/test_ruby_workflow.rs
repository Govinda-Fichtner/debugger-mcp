/// Workflow-level integration tests for Ruby debugging
///
/// These tests verify the HIGH-LEVEL debugging workflow through SessionManager
/// and DebugSession - the actual APIs that the MCP server uses.
///
/// See docs/TEST_COVERAGE_GAP_ANALYSIS.md for why these are critical.
use debugger_mcp::debug::manager::SessionManager;
use debugger_mcp::debug::state::DebugState;
use std::io::Write;
use std::time::Duration;
use tokio::time::sleep;

/// Test 1: Full session lifecycle (CRITICAL - would have caught the bugs!)
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_full_session_lifecycle() {
    // Create test script
    let test_script = "/tmp/test_workflow_lifecycle.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "x = 10").unwrap();
    writeln!(file, "y = 20").unwrap();
    writeln!(file, "puts x + y").unwrap();
    drop(file);

    // Create SessionManager (like MCP server does)
    let manager = SessionManager::new();

    // Create session (like MCP tools do)
    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true, // stop_on_entry
        )
        .await
        .expect("Failed to create session");

    // Wait for state transition from Initializing to Stopped
    // THIS check would have caught the "stuck in Initializing" bug!
    let mut attempts = 0;
    loop {
        let state = manager.get_session_state(&session_id).await.unwrap();

        match state {
            DebugState::Stopped { .. } => {
                // Success!
                break;
            }
            DebugState::Initializing | DebugState::Launching => {
                if attempts > 30 {
                    panic!("Session stuck in {:?} after 3s - THIS IS THE BUG!", state);
                }
                attempts += 1;
                sleep(Duration::from_millis(100)).await;
            }
            DebugState::Failed { error } => {
                panic!("Session failed: {}", error);
            }
            _ => {
                attempts += 1;
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    println!(
        "✅ Session reached Stopped state after {} attempts",
        attempts
    );

    // Get session and verify we can interact
    let session = manager.get_session(&session_id).await.unwrap();

    // Continue execution
    session.continue_execution().await.unwrap();

    // Wait a bit for completion
    sleep(Duration::from_millis(500)).await;

    // Disconnect
    manager.remove_session(&session_id).await.unwrap();

    // Cleanup
    std::fs::remove_file(test_script).ok();
}

/// Test 2: State transitions
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_state_transitions() {
    let test_script = "/tmp/test_workflow_states.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "sleep 0.1").unwrap();
    drop(file);

    let manager = SessionManager::new();

    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    // Track state transitions
    let mut transitions = Vec::new();
    let mut prev_state = manager.get_session_state(&session_id).await.unwrap();
    transitions.push(format!("{:?}", prev_state));

    for _ in 0..20 {
        sleep(Duration::from_millis(100)).await;
        let state = manager.get_session_state(&session_id).await.unwrap();
        if format!("{:?}", state) != format!("{:?}", prev_state) {
            transitions.push(format!("{:?}", state));
            prev_state = state;
        }
    }

    println!("State transitions: {:?}", transitions);

    // Should have transitioned to Stopped
    assert!(
        transitions.iter().any(|s| s.contains("Stopped")),
        "Should reach Stopped state. Transitions: {:?}",
        transitions
    );

    manager.remove_session(&session_id).await.unwrap();
    std::fs::remove_file(test_script).ok();
}

/// Test 3: Breakpoint workflow
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_breakpoint_workflow() {
    let test_script = "/tmp/test_workflow_breakpoint.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "x = 1").unwrap();
    writeln!(file, "y = 2  # Breakpoint here").unwrap();
    writeln!(file, "z = 3").unwrap();
    drop(file);

    let manager = SessionManager::new();
    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    // Wait for Stopped state
    for _ in 0..30 {
        let state = manager.get_session_state(&session_id).await.unwrap();
        if matches!(state, DebugState::Stopped { .. }) {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let session = manager.get_session(&session_id).await.unwrap();

    // Set breakpoint on line 2
    let bp_set = session
        .set_breakpoint(test_script.to_string(), 2)
        .await
        .unwrap();
    assert!(bp_set, "Breakpoint should be set");

    // Continue - should hit breakpoint
    session.continue_execution().await.unwrap();
    sleep(Duration::from_millis(300)).await;

    let state = manager.get_session_state(&session_id).await.unwrap();
    assert!(
        matches!(state, DebugState::Stopped { .. }),
        "Should stop at breakpoint, got: {:?}",
        state
    );

    manager.remove_session(&session_id).await.unwrap();
    std::fs::remove_file(test_script).ok();
}

/// Test 4: Variable evaluation
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_variable_evaluation() {
    let test_script = "/tmp/test_workflow_eval.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "magic_number = 42").unwrap();
    writeln!(file, "sleep 0.1").unwrap();
    drop(file);

    let manager = SessionManager::new();
    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    // Wait for Stopped
    for _ in 0..30 {
        let state = manager.get_session_state(&session_id).await.unwrap();
        if matches!(state, DebugState::Stopped { .. }) {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let session = manager.get_session(&session_id).await.unwrap();

    // Step to execute the assignment
    if let DebugState::Stopped { thread_id, .. } = session.get_state().await {
        session.step_over(thread_id).await.unwrap();
        sleep(Duration::from_millis(200)).await;

        // Evaluate variable
        let result = session.evaluate("magic_number", None).await.unwrap();
        println!("Evaluation result: {}", result);

        assert!(
            result.contains("42"),
            "Should evaluate to 42, got: {}",
            result
        );
    }

    manager.remove_session(&session_id).await.unwrap();
    std::fs::remove_file(test_script).ok();
}

/// Test 5: Step commands
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_step_commands() {
    let test_script = "/tmp/test_workflow_steps.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "def helper").unwrap();
    writeln!(file, "  x = 1").unwrap();
    writeln!(file, "end").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "helper").unwrap();
    writeln!(file, "puts 'done'").unwrap();
    drop(file);

    let manager = SessionManager::new();
    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    // Wait for Stopped
    for _ in 0..30 {
        let state = manager.get_session_state(&session_id).await.unwrap();
        if matches!(state, DebugState::Stopped { .. }) {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let session = manager.get_session(&session_id).await.unwrap();

    if let DebugState::Stopped { thread_id, .. } = session.get_state().await {
        // Step over should work
        session.step_over(thread_id).await.unwrap();
        sleep(Duration::from_millis(200)).await;

        let state = session.get_state().await;
        assert!(
            matches!(state, DebugState::Stopped { .. }),
            "Should stop after step over"
        );

        // Step into should work
        session.step_into(thread_id).await.unwrap();
        sleep(Duration::from_millis(200)).await;

        let state = session.get_state().await;
        assert!(
            matches!(state, DebugState::Stopped { .. }),
            "Should stop after step in"
        );
    }

    manager.remove_session(&session_id).await.unwrap();
    std::fs::remove_file(test_script).ok();
}

/// Test 6: Multiple concurrent sessions
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_multiple_sessions() {
    let test_script1 = "/tmp/test_workflow_multi1.rb";
    let test_script2 = "/tmp/test_workflow_multi2.rb";

    let mut file1 = std::fs::File::create(test_script1).unwrap();
    writeln!(file1, "puts 'Session 1'").unwrap();
    drop(file1);

    let mut file2 = std::fs::File::create(test_script2).unwrap();
    writeln!(file2, "puts 'Session 2'").unwrap();
    drop(file2);

    let manager = SessionManager::new();

    // Start two sessions
    let session_id1 = manager
        .create_session(
            "ruby",
            test_script1.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    let session_id2 = manager
        .create_session(
            "ruby",
            test_script2.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    assert_ne!(session_id1, session_id2, "Session IDs should be unique");

    // Both should reach Stopped
    for _ in 0..30 {
        let state1 = manager.get_session_state(&session_id1).await.unwrap();
        let state2 = manager.get_session_state(&session_id2).await.unwrap();

        if matches!(state1, DebugState::Stopped { .. })
            && matches!(state2, DebugState::Stopped { .. })
        {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Cleanup
    manager.remove_session(&session_id1).await.unwrap();
    manager.remove_session(&session_id2).await.unwrap();

    std::fs::remove_file(test_script1).ok();
    std::fs::remove_file(test_script2).ok();
}

/// Test 7: Error handling - invalid program
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_invalid_program() {
    let manager = SessionManager::new();

    let result = manager
        .create_session(
            "ruby",
            "/nonexistent/script.rb".to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await;

    assert!(result.is_err(), "Should fail for nonexistent script");
}

/// Test 8: Session startup performance
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_session_performance() {
    let test_script = "/tmp/test_workflow_perf.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "x = 1").unwrap();
    drop(file);

    let manager = SessionManager::new();
    let start = std::time::Instant::now();

    let session_id = manager
        .create_session(
            "ruby",
            test_script.to_string(),
            vec![],
            Some("/tmp".to_string()),
            true,
        )
        .await
        .unwrap();

    // Wait for Stopped state
    for _ in 0..30 {
        let state = manager.get_session_state(&session_id).await.unwrap();
        if matches!(state, DebugState::Stopped { .. }) {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let elapsed = start.elapsed();

    // Should reach Stopped within 3 seconds
    assert!(
        elapsed < Duration::from_secs(3),
        "Session startup took too long: {:?}",
        elapsed
    );

    println!("✅ Session startup time: {:?}", elapsed);

    manager.remove_session(&session_id).await.unwrap();
    std::fs::remove_file(test_script).ok();
}
