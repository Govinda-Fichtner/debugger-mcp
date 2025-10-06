use debugger_mcp::debug::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Phase 1: Test Ruby language detection
/// This test will FAIL initially (Red phase)
#[tokio::test]
#[ignore]
async fn test_ruby_language_detection() {
    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Try to create a Ruby debug session
    let result = session_manager
        .create_session(
            "ruby",
            "tests/fixtures/fizzbuzz.rb".to_string(),
            vec![],
            None,
            true,
        )
        .await;

    // This should succeed once Ruby adapter is implemented
    assert!(
        result.is_ok(),
        "Ruby language should be supported: {:?}",
        result
    );
}

/// Phase 2: Test Ruby adapter spawning
/// This test will FAIL initially (Red phase)
#[tokio::test]
#[ignore]
async fn test_ruby_adapter_spawning() {
    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Create a Ruby debug session
    let session_id = session_manager
        .create_session(
            "ruby",
            "tests/fixtures/fizzbuzz.rb".to_string(),
            vec![],
            None,
            true,
        )
        .await
        .expect("Should create Ruby session");

    // Wait a bit for initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify session exists
    let session = session_manager.get_session(&session_id).await;
    assert!(session.is_ok(), "Should get Ruby session");

    // Verify session language
    let session = session.unwrap();
    assert_eq!(session.language, "ruby");
    assert_eq!(session.program, "tests/fixtures/fizzbuzz.rb");
}
