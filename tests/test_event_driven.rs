/// Test the event-driven DAP client implementation
use debugger_mcp::dap::client::DapClient;
use std::path::PathBuf;

#[tokio::test]
#[ignore]
async fn test_event_driven_launch() {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();

    println!("\n=== Testing Event-Driven DAP Client ===\n");

    // Get fizzbuzz path
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let fizzbuzz_path = PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("fizzbuzz.py");

    println!("1. Spawning debugpy adapter...");
    let client = DapClient::spawn("python", &["-m".to_string(), "debugpy.adapter".to_string()])
        .await
        .expect("Failed to spawn adapter");

    println!("2. Preparing launch args...");
    let launch_args = serde_json::json!({
        "request": "launch",
        "type": "python",
        "program": fizzbuzz_path.to_string_lossy(),
        "args": [],
        "console": "internalConsole",
        "stopOnEntry": false,
    });

    println!("3. Calling initialize_and_launch...");
    match client.initialize_and_launch("debugpy", launch_args).await {
        Ok(_) => {
            println!("✅ SUCCESS: initialize_and_launch completed!");
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            panic!("Test failed");
        }
    }

    println!("\n=== Test Completed Successfully ===\n");
}
