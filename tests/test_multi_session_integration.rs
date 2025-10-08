/// Integration tests for multi-session architecture (Node.js debugging)
///
/// These tests verify that the multi-session architecture works correctly
/// for Node.js debugging with vscode-js-debug. The architecture uses a
/// parent-child session model where:
/// - Parent session coordinates (dapDebugServer.js)
/// - Child sessions do actual debugging (pwa-node)
///
/// Tests are organized by functionality:
/// 1. Session creation with multi-session mode
/// 2. Child session spawning and management
/// 3. Operation routing to child sessions
/// 4. Event forwarding from child to parent

use debugger_mcp::debug::{SessionManager, DebugState, SessionMode};
use debugger_mcp::debug::{DebugSession, MultiSessionManager, ChildSession};
use debugger_mcp::dap::client::DapClient;
use debugger_mcp::{Error, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create a mock child session for testing
async fn create_mock_child_session(port: u16) -> ChildSession {
    use debugger_mcp::dap::transport_trait::DapTransportTrait;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<debugger_mcp::dap::types::Message>;
            async fn write_message(&mut self, msg: &debugger_mcp::dap::types::Message) -> Result<()>;
        }
    }

    let mut mock_transport = MockTestTransport::new();
    mock_transport.expect_read_message()
        .returning(|| Err(Error::Dap("Connection closed".to_string())));

    let client = DapClient::new_with_transport(Box::new(mock_transport), None)
        .await
        .unwrap();

    ChildSession {
        id: format!("child-{}", port),
        client: Arc::new(RwLock::new(client)),
        port,
        session_type: "pwa-node".to_string(),
    }
}

// ============================================================================
// Test Group 1: Multi-Session Manager Functionality
// ============================================================================

#[tokio::test]
async fn test_multi_session_manager_creates_successfully() {
    let manager = MultiSessionManager::new("parent-123".to_string());

    assert_eq!(manager.parent_id(), "parent-123");
    assert_eq!(manager.child_count().await, 0);
    assert!(manager.get_active_child().await.is_none());
}

#[tokio::test]
async fn test_adding_child_session_sets_as_active() {
    let manager = MultiSessionManager::new("parent".to_string());
    let child = create_mock_child_session(9000).await;

    manager.add_child(child).await;

    assert_eq!(manager.child_count().await, 1);
    assert!(manager.get_active_child().await.is_some());
    assert_eq!(manager.get_active_child_id().await, Some("child-9000".to_string()));
}

#[tokio::test]
async fn test_multiple_child_sessions() {
    let manager = MultiSessionManager::new("parent".to_string());

    let child1 = create_mock_child_session(9000).await;
    let child2 = create_mock_child_session(9001).await;
    let child3 = create_mock_child_session(9002).await;

    manager.add_child(child1).await;
    manager.add_child(child2).await;
    manager.add_child(child3).await;

    assert_eq!(manager.child_count().await, 3);
    // First child should still be active
    assert_eq!(manager.get_active_child_id().await, Some("child-9000".to_string()));

    // Switch to second child
    manager.set_active_child("child-9001".to_string()).await.unwrap();
    assert_eq!(manager.get_active_child_id().await, Some("child-9001".to_string()));
}

#[tokio::test]
async fn test_removing_active_child_switches_to_next() {
    let manager = MultiSessionManager::new("parent".to_string());

    let child1 = create_mock_child_session(9000).await;
    let child2 = create_mock_child_session(9001).await;

    manager.add_child(child1).await;
    manager.add_child(child2).await;

    // Remove active child (child-9000)
    manager.remove_child("child-9000").await.unwrap();

    // Should switch to child-9001
    assert_eq!(manager.child_count().await, 1);
    assert_eq!(manager.get_active_child_id().await, Some("child-9001".to_string()));
}

// ============================================================================
// Test Group 2: Session Mode Functionality
// ============================================================================

#[tokio::test]
async fn test_session_mode_single_for_python() {
    use debugger_mcp::dap::transport_trait::DapTransportTrait;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<debugger_mcp::dap::types::Message>;
            async fn write_message(&mut self, msg: &debugger_mcp::dap::types::Message) -> Result<()>;
        }
    }

    let mut mock_transport = MockTestTransport::new();
    mock_transport.expect_read_message()
        .returning(|| Err(Error::Dap("Connection closed".to_string())));

    let client = DapClient::new_with_transport(Box::new(mock_transport), None)
        .await
        .unwrap();

    // Python uses Single mode (default constructor)
    let session = DebugSession::new(
        "python".to_string(),
        "test.py".to_string(),
        client,
    )
    .await
    .unwrap();

    // Verify session created successfully
    assert_eq!(session.language, "python");
    assert_eq!(session.program, "test.py");

    // Session mode should be Single (we can't directly check, but operations should work)
    let state = session.get_state().await;
    assert_eq!(state, DebugState::NotStarted);
}

#[tokio::test]
async fn test_session_mode_multi_for_nodejs() {
    use debugger_mcp::dap::transport_trait::DapTransportTrait;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<debugger_mcp::dap::types::Message>;
            async fn write_message(&mut self, msg: &debugger_mcp::dap::types::Message) -> Result<()>;
        }
    }

    let mut mock_transport = MockTestTransport::new();
    mock_transport.expect_read_message()
        .returning(|| Err(Error::Dap("Connection closed".to_string())));

    let client = DapClient::new_with_transport(Box::new(mock_transport), None)
        .await
        .unwrap();

    // Node.js uses MultiSession mode
    let manager = MultiSessionManager::new("session-id".to_string());
    let session_mode = SessionMode::MultiSession {
        parent_client: Arc::new(RwLock::new(client)),
        multi_session_manager: manager,
        vscode_js_debug_port: 12345, // Mock port for testing
    };

    let session = DebugSession::new_with_mode(
        "nodejs".to_string(),
        "test.js".to_string(),
        session_mode,
    )
    .await
    .unwrap();

    // Verify session created successfully
    assert_eq!(session.language, "nodejs");
    assert_eq!(session.program, "test.js");

    let state = session.get_state().await;
    assert_eq!(state, DebugState::NotStarted);
}

// ============================================================================
// Test Group 3: Operation Routing
// ============================================================================

/// Test that operations route to the correct client based on session mode
///
/// Note: This test verifies the routing logic works, but doesn't actually
/// send DAP requests since we're using mock transports.
#[tokio::test]
async fn test_operation_routing_single_mode() {
    use debugger_mcp::dap::transport_trait::DapTransportTrait;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<debugger_mcp::dap::types::Message>;
            async fn write_message(&mut self, msg: &debugger_mcp::dap::types::Message) -> Result<()>;
        }
    }

    let mut mock_transport = MockTestTransport::new();
    mock_transport.expect_read_message()
        .returning(|| Err(Error::Dap("Connection closed".to_string())));

    let client = DapClient::new_with_transport(Box::new(mock_transport), None)
        .await
        .unwrap();

    let session = DebugSession::new(
        "python".to_string(),
        "test.py".to_string(),
        client,
    )
    .await
    .unwrap();

    // Test that get_state works (uses internal routing)
    let state = session.get_state().await;
    assert_eq!(state, DebugState::NotStarted);
}

#[tokio::test]
async fn test_operation_routing_multi_mode_no_child() {
    use debugger_mcp::dap::transport_trait::DapTransportTrait;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<debugger_mcp::dap::types::Message>;
            async fn write_message(&mut self, msg: &debugger_mcp::dap::types::Message) -> Result<()>;
        }
    }

    let mut mock_transport = MockTestTransport::new();
    mock_transport.expect_read_message()
        .returning(|| Err(Error::Dap("Connection closed".to_string())));

    let client = DapClient::new_with_transport(Box::new(mock_transport), None)
        .await
        .unwrap();

    let manager = MultiSessionManager::new("session-id".to_string());
    let session_mode = SessionMode::MultiSession {
        parent_client: Arc::new(RwLock::new(client)),
        multi_session_manager: manager,
        vscode_js_debug_port: 12345, // Mock port for testing
    };

    let session = DebugSession::new_with_mode(
        "nodejs".to_string(),
        "test.js".to_string(),
        session_mode,
    )
    .await
    .unwrap();

    // With no child, operations should fall back to parent client
    let state = session.get_state().await;
    assert_eq!(state, DebugState::NotStarted);
}

// ============================================================================
// Test Group 4: Documentation Tests
// ============================================================================

/// This test documents the expected workflow for multi-session debugging
///
/// Run with: cargo test test_multi_session_workflow_documentation -- --nocapture
#[test]
fn test_multi_session_workflow_documentation() {
    println!("\n=== Multi-Session Debugging Workflow ===\n");
    println!("1. User calls create_session('nodejs', 'fizzbuzz.js', ...)");
    println!("   └─> SessionManager spawns vscode-js-debug parent (dapDebugServer.js)");
    println!("");
    println!("2. SessionManager creates DebugSession with MultiSession mode");
    println!("   ├─> MultiSessionManager tracks parent-child relationships");
    println!("   └─> Registers child session spawn callback on parent client");
    println!("");
    println!("3. SessionManager calls initialize_and_launch() on session");
    println!("   ├─> Parent session sends initialize request");
    println!("   ├─> Parent session sends launch request");
    println!("   └─> Parent session sends configurationDone");
    println!("");
    println!("4. Parent sends 'startDebugging' reverse request");
    println!("   ├─> DapClient message_reader detects reverse request");
    println!("   ├─> Extracts port from configuration.__jsDebugChildServer");
    println!("   └─> Invokes registered callback(port)");
    println!("");
    println!("5. Callback triggers session.spawn_child_session(port)");
    println!("   ├─> Connects to child port via TCP");
    println!("   ├─> Creates DapClient for child");
    println!("   ├─> Initializes child session");
    println!("   ├─> Registers event handlers (stopped, continued, etc.)");
    println!("   └─> Adds child to MultiSessionManager as active");
    println!("");
    println!("6. User calls set_breakpoint(session_id, 'fizzbuzz.js', 5)");
    println!("   ├─> DebugSession.get_debug_client() returns active child");
    println!("   ├─> Breakpoint request sent to child session");
    println!("   └─> Child responds with verified=true");
    println!("");
    println!("7. User calls continue_execution(session_id)");
    println!("   ├─> Continue request sent to child session");
    println!("   └─> Child session runs Node.js program");
    println!("");
    println!("8. Breakpoint hit in Node.js program");
    println!("   ├─> Child session sends 'stopped' event");
    println!("   ├─> Event handler forwards to parent session state");
    println!("   └─> Parent session state set to Stopped");
    println!("");
    println!("9. User calls evaluate(session_id, 'n', frame_id)");
    println!("   ├─> Evaluate request sent to child session");
    println!("   └─> Child returns variable value");
    println!("");
    println!("=== End Workflow ===\n");
}

/// This test documents the key architectural components
#[test]
fn test_architecture_documentation() {
    println!("\n=== Multi-Session Architecture Components ===\n");
    println!("SessionMode enum:");
    println!("  Single {{ client }} - Python, Ruby");
    println!("  MultiSession {{ parent_client, multi_session_manager }} - Node.js");
    println!("");
    println!("MultiSessionManager:");
    println!("  - Tracks child sessions (HashMap<String, ChildSession>)");
    println!("  - Manages active child selection");
    println!("  - Routes operations to correct child");
    println!("");
    println!("ChildSession:");
    println!("  - id: Unique identifier");
    println!("  - client: DapClient for child");
    println!("  - port: TCP port");
    println!("  - session_type: 'pwa-node', 'chrome', etc.");
    println!("");
    println!("DapClient callbacks:");
    println!("  - on_child_session_spawn(callback)");
    println!("  - Callback invoked when startDebugging reverse request received");
    println!("");
    println!("Operation routing:");
    println!("  - get_debug_client() returns appropriate client");
    println!("  - Single mode: returns sole client");
    println!("  - MultiSession: returns active child (fallback to parent)");
    println!("");
    println!("Event forwarding:");
    println!("  - Child event handlers registered in spawn_child_session()");
    println!("  - Events forwarded to parent session state");
    println!("  - Maintains single source of truth");
    println!("");
    println!("=== End Architecture ===\n");
}

// ============================================================================
// Test Group 5: Integration Tests (Require vscode-js-debug)
// ============================================================================

/// Full integration test with vscode-js-debug
///
/// This test requires vscode-js-debug to be installed and available.
/// Run with: cargo test test_nodejs_multi_session_full_workflow -- --ignored
#[tokio::test]
#[ignore]
async fn test_nodejs_multi_session_full_workflow() {
    use std::time::Duration;
    use tokio::time::timeout;

    // Create session manager
    let manager = SessionManager::new();

    // Create Node.js session (will spawn vscode-js-debug)
    let session_id = manager
        .create_session(
            "nodejs",
            "tests/fixtures/fizzbuzz.js".to_string(),
            vec![],
            Some("tests/fixtures".to_string()),
            true, // stopOnEntry
        )
        .await
        .expect("Failed to create Node.js session");

    println!("✅ Session created: {}", session_id);

    // Wait for initialization (parent + child spawn)
    timeout(Duration::from_secs(5), async {
        loop {
            let state = manager.get_session_state(&session_id).await.unwrap();
            match state {
                DebugState::Stopped { .. } => break,
                DebugState::Running => break,
                DebugState::Failed { error } => panic!("Session failed: {}", error),
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    })
    .await
    .expect("Timeout waiting for session initialization");

    println!("✅ Session initialized and child spawned");

    // Get session and verify multi-session mode
    let session = manager.get_session(&session_id).await.unwrap();

    // Verify it's in multi-session mode
    if let SessionMode::MultiSession { multi_session_manager, .. } = &session.session_mode {
        assert_eq!(multi_session_manager.child_count().await, 1, "Should have 1 child session");
        assert!(multi_session_manager.get_active_child().await.is_some(), "Should have active child");
        println!("✅ Multi-session mode verified: 1 child session active");
    } else {
        panic!("Session should be in MultiSession mode");
    }

    // Set breakpoint (should go to child)
    let verified = session
        .set_breakpoint("tests/fixtures/fizzbuzz.js".to_string(), 5)
        .await
        .expect("Failed to set breakpoint");

    assert!(verified, "Breakpoint should be verified by child session");
    println!("✅ Breakpoint set and verified on line 5");

    // Continue execution
    session
        .continue_execution()
        .await
        .expect("Failed to continue execution");
    println!("✅ Continue execution sent");

    // Wait for breakpoint hit
    timeout(Duration::from_secs(3), async {
        loop {
            let state = session.get_state().await;
            if let DebugState::Stopped { reason, .. } = state {
                if reason == "breakpoint" {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Timeout waiting for breakpoint");

    println!("✅ Breakpoint hit");

    // Get stack trace
    let stack_trace = session.stack_trace().await.expect("Failed to get stack trace");
    assert!(!stack_trace.is_empty(), "Stack trace should not be empty");
    println!("✅ Stack trace retrieved: {} frames", stack_trace.len());

    // Evaluate variable
    let frame_id = stack_trace.first().unwrap().id;
    let result = session
        .evaluate("n", Some(frame_id))
        .await
        .expect("Failed to evaluate expression");
    println!("✅ Variable 'n' = {}", result);

    // Step over
    let thread_id = 1;
    session
        .step_over(thread_id)
        .await
        .expect("Failed to step over");
    println!("✅ Step over completed");

    // Disconnect
    session.disconnect().await.expect("Failed to disconnect");
    println!("✅ Session disconnected");

    // Clean up
    manager
        .remove_session(&session_id)
        .await
        .expect("Failed to remove session");
    println!("✅ Session removed");
}

/// Test child session spawning specifically
#[tokio::test]
#[ignore]
async fn test_child_session_spawning() {
    let manager = SessionManager::new();

    let session_id = manager
        .create_session(
            "nodejs",
            "tests/fixtures/fizzbuzz.js".to_string(),
            vec![],
            None,
            false,
        )
        .await
        .expect("Failed to create session");

    // Wait for child to spawn
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let session = manager.get_session(&session_id).await.unwrap();

    if let SessionMode::MultiSession { multi_session_manager, .. } = &session.session_mode {
        let child_count = multi_session_manager.child_count().await;
        assert!(child_count > 0, "At least one child session should be spawned");
        println!("✅ {} child session(s) spawned", child_count);

        let children = multi_session_manager.get_children().await;
        for child_id in children {
            println!("  - Child: {}", child_id);
        }
    } else {
        panic!("Expected MultiSession mode");
    }

    manager.remove_session(&session_id).await.ok();
}
