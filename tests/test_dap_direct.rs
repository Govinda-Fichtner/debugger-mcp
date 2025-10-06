/// Direct test of DAP client initialization with real debugpy adapter.
/// This test helps diagnose why the integration test times out.
use debugger_mcp::dap::client::DapClient;
use tokio::time::{timeout, Duration};

#[tokio::test]
#[ignore]
async fn test_dap_client_with_real_debugpy() {
    println!("Starting test...");

    // Spawn debugpy adapter
    let command = "python";
    let args = vec!["-m".to_string(), "debugpy.adapter".to_string()];

    println!("Spawning DAP client: {} {:?}", command, args);

    let client_result = timeout(
        Duration::from_secs(5),
        DapClient::spawn(command, &args)
    ).await;

    let client = match client_result {
        Ok(Ok(c)) => {
            println!("✅ Client spawned successfully");
            c
        }
        Ok(Err(e)) => {
            println!("❌ Failed to spawn client: {}", e);
            panic!("Spawn failed");
        }
        Err(_) => {
            println!("❌ Spawn timed out after 5s");
            panic!("Spawn timeout");
        }
    };

    // Try to initialize
    println!("Sending initialize request...");

    let init_result = timeout(
        Duration::from_secs(5),
        client.initialize("debugpy")
    ).await;

    match init_result {
        Ok(Ok(caps)) => {
            println!("✅ Initialize SUCCESS!");
            println!("   Capabilities: supportsConfigurationDoneRequest={:?}",
                     caps.supports_configuration_done_request);
        }
        Ok(Err(e)) => {
            println!("❌ Initialize failed: {}", e);
            panic!("Initialize error");
        }
        Err(_) => {
            println!("❌ Initialize timed out after 5s");
            println!("   This means the adapter process is running but not responding");
            println!("   Likely cause: message_handler not processing events before response");
            panic!("Initialize timeout");
        }
    }

    // Send configurationDone (DAP spec requires this after initialize)
    println!("Sending configurationDone...");

    let config_result = timeout(
        Duration::from_secs(5),
        client.configuration_done()
    ).await;

    match config_result {
        Ok(Ok(())) => {
            println!("✅ ConfigurationDone SUCCESS!");
        }
        Ok(Err(e)) => {
            println!("❌ ConfigurationDone failed: {}", e);
            panic!("ConfigurationDone error");
        }
        Err(_) => {
            println!("❌ ConfigurationDone timed out after 5s");
            panic!("ConfigurationDone timeout");
        }
    }

    // Try to launch a program
    println!("Sending launch request...");

    let fizzbuzz_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fizzbuzz.py");

    let launch_args = serde_json::json!({
        "request": "launch",
        "type": "python",
        "program": fizzbuzz_path.to_string_lossy(),
        "args": [],
        "console": "internalConsole",
        "stopOnEntry": false,
    });

    let launch_result = timeout(
        Duration::from_secs(5),
        client.launch(launch_args)
    ).await;

    match launch_result {
        Ok(Ok(())) => {
            println!("✅ Launch SUCCESS!");
        }
        Ok(Err(e)) => {
            println!("❌ Launch failed: {}", e);
            panic!("Launch error");
        }
        Err(_) => {
            println!("❌ Launch timed out after 5s");
            panic!("Launch timeout");
        }
    }

    println!("✅ Test completed successfully");
}
