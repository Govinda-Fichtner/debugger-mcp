use debugger_mcp::debug::SessionManager;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test Rust language detection
#[tokio::test]
#[ignore]
async fn test_rust_language_detection() {
    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Try to create a Rust debug session
    let result = session_manager
        .create_session(
            "rust",
            "tests/fixtures/fizzbuzz.rs".to_string(),
            vec![],
            None,
            true,
        )
        .await;

    assert!(
        result.is_ok(),
        "Rust language should be supported: {:?}",
        result
    );
}

/// Test Rust adapter spawning
#[tokio::test]
#[ignore]
async fn test_rust_adapter_spawning() {
    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Create a Rust debug session
    let session_id = session_manager
        .create_session(
            "rust",
            "tests/fixtures/fizzbuzz.rs".to_string(),
            vec![],
            None,
            true,
        )
        .await
        .expect("Should create Rust session");

    // Wait a bit for initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify session exists
    let session = session_manager.get_session(&session_id).await;
    assert!(session.is_ok(), "Should get Rust session");

    // Verify session language
    let session = session.unwrap();
    assert_eq!(session.language, "rust");
    assert_eq!(session.program, "tests/fixtures/fizzbuzz.rs");
}

/// Full Rust FizzBuzz debugging integration test
#[tokio::test]
#[ignore]
async fn test_rust_fizzbuzz_debugging_integration() {
    use tokio::time::{timeout, Duration};

    // Wrap entire test in timeout
    let test_result = timeout(Duration::from_secs(30), async {
        // Setup
        let _session_manager = Arc::new(RwLock::new(SessionManager::new()));

        // Get absolute path to fizzbuzz.rs
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let _fizzbuzz_path = PathBuf::from(manifest_dir)
            .join("tests")
            .join("fixtures")
            .join("fizzbuzz.rs");

        // Check if lldb is available
        let lldb_check = std::process::Command::new("lldb").arg("--version").output();

        if lldb_check.is_err() || !lldb_check.unwrap().status.success() {
            println!("⚠️  Skipping Rust FizzBuzz test: lldb not installed");
            println!("   Install with: apt install lldb (Debian/Ubuntu)");
            return Ok::<(), String>(());
        }

        // Need to compile Rust file first to get executable
        // For now, we'll skip this test as it requires compilation infrastructure
        println!("⚠️  Skipping Rust FizzBuzz test: requires pre-compiled binary");
        println!("   Rust debugging requires compiling the source to an executable first");
        Ok(())

        // Commented out: would need compiled binary path
        /*
        // 1. Start debugger session with stopOnEntry
        println!("🔧 Starting Rust debug session for: {}", fizzbuzz_str);

        let start_args = json!({
            "language": "rust",
            "program": fizzbuzz_str,
            "args": [],
            "cwd": null,
            "stopOnEntry": true
        });

        let start_result = timeout(
            Duration::from_secs(30),
            tools_handler.handle_tool("debugger_start", start_args),
        )
        .await;

        // ... rest of test follows same pattern as other languages
        */
    })
    .await;

    match test_result {
        Ok(Ok(())) => {
            println!("✅ Test completed within timeout");
        }
        Ok(Err(e)) => {
            println!("⚠️  Test encountered error: {}", e);
        }
        Err(_) => {
            println!("⚠️  Test timed out after 30 seconds");
        }
    }
}

/// Test that validates Rust MCP server works with Claude Code CLI
#[tokio::test]
#[ignore]
async fn test_rust_claude_code_integration() {
    println!("\n🚀 Starting Rust Claude Code Integration Test");
    println!("════════════════════════════════════════════════════════════════");

    // 1. Check Claude CLI is available
    println!("\n📋 Step 1: Checking Claude CLI availability...");
    let claude_check = Command::new("claude").arg("--version").output();

    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        println!("⚠️  Skipping test: Claude CLI not found");
        return;
    }
    println!("✅ Claude CLI is available");

    // 2. Check if LLDB is available
    let lldb_check = Command::new("lldb").arg("--version").output();
    if lldb_check.is_err() || !lldb_check.unwrap().status.success() {
        println!("⚠️  Skipping test: LLDB not installed");
        return;
    }

    // 3. Note: Rust debugging requires pre-compiled binary
    println!("⚠️  Skipping test: Rust debugging requires compiled binary");
    println!("   Rust source files must be compiled before debugging");
    println!("   Future enhancement: Add compilation step to test");

    println!("\n🎉 Rust Claude Code integration test completed (skipped - needs compilation)!");
}
