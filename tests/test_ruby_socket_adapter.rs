/// Comprehensive test harness for Ruby socket-based DAP adapter
///
/// This test suite verifies that the Ruby adapter correctly:
/// 1. Finds free ports
/// 2. Spawns rdbg with --open flag
/// 3. Connects to the TCP socket
/// 4. Communicates via DAP protocol
/// 5. Handles timeouts appropriately

use debugger_mcp::adapters::ruby::RubyAdapter;
use debugger_mcp::dap::socket_helper;
use debugger_mcp::dap::transport::DapTransport;
use debugger_mcp::dap::types::{Message, Request};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;

/// Test 1: Socket helper - find free port
#[test]
fn test_socket_helper_find_free_port() {
    let port = socket_helper::find_free_port().unwrap();
    assert!(port > 1024, "Port should be > 1024 (non-privileged)");
    // Port is u16, so it's always < 65536
}

/// Test 2: Socket helper - find multiple unique ports
#[test]
fn test_socket_helper_unique_ports() {
    let port1 = socket_helper::find_free_port().unwrap();
    let port2 = socket_helper::find_free_port().unwrap();
    let port3 = socket_helper::find_free_port().unwrap();

    // All ports should be unique (very high probability)
    assert_ne!(port1, port2);
    assert_ne!(port2, port3);
    assert_ne!(port1, port3);
}

/// Test 3: Socket helper - connect with retry (success case)
#[tokio::test]
async fn test_socket_helper_connect_success() {
    // Start a test TCP server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Accept connection in background
    tokio::spawn(async move {
        let _ = listener.accept().await;
        tokio::time::sleep(Duration::from_secs(10)).await; // Keep alive
    });

    // Connect should succeed quickly
    let start = std::time::Instant::now();
    let result = socket_helper::connect_with_retry(port, Duration::from_secs(2)).await;

    assert!(result.is_ok(), "Should connect successfully");
    assert!(start.elapsed() < Duration::from_millis(500), "Should connect quickly");
}

/// Test 4: Socket helper - connect with retry (timeout case)
#[tokio::test]
async fn test_socket_helper_connect_timeout() {
    // Try to connect to a port that's not listening
    let port = socket_helper::find_free_port().unwrap();

    let start = std::time::Instant::now();
    let result = socket_helper::connect_with_retry(port, Duration::from_millis(500)).await;

    assert!(result.is_err(), "Should timeout");
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(500),
        "Should wait at least 500ms before timeout"
    );
    assert!(
        elapsed < Duration::from_millis(800),
        "Should timeout within reasonable time"
    );
}

/// Test 5: Socket helper - connect with retry (eventual success)
#[tokio::test]
async fn test_socket_helper_connect_eventual_success() {
    let port = socket_helper::find_free_port().unwrap();

    // Start server after a delay (simulating rdbg startup time)
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let _ = listener.accept().await;
        tokio::time::sleep(Duration::from_secs(10)).await; // Keep alive
    });

    // Should retry and eventually connect
    let start = std::time::Instant::now();
    let result = socket_helper::connect_with_retry(port, Duration::from_secs(2)).await;

    assert!(result.is_ok(), "Should eventually connect");
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(300),
        "Should take at least 300ms (server startup delay)"
    );
    assert!(
        elapsed < Duration::from_millis(600),
        "Should connect soon after server starts"
    );
}

/// Test 6: DapTransport - create socket transport
#[tokio::test]
async fn test_dap_transport_socket_creation() {
    // Create a test server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    // Connect and create transport
    let stream = socket_helper::connect_with_retry(port, Duration::from_secs(1))
        .await
        .unwrap();

    let transport = DapTransport::new_socket(stream);

    // Verify it's created (compilation test)
    drop(transport);
}

/// Test 7: DapTransport - socket read/write DAP message
#[tokio::test]
async fn test_dap_transport_socket_read_write() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Spawn echo server
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Read DAP request
        let mut buffer = vec![0u8; 1024];
        let n = socket.read(&mut buffer).await.unwrap();

        // Echo it back
        socket.write_all(&buffer[..n]).await.unwrap();
        socket.flush().await.unwrap();

        tokio::time::sleep(Duration::from_secs(10)).await; // Keep alive
    });

    // Connect
    let stream = socket_helper::connect_with_retry(port, Duration::from_secs(1))
        .await
        .unwrap();

    let mut transport = DapTransport::new_socket(stream);

    // Send a message
    let request = Message::Request(Request {
        seq: 1,
        command: "initialize".to_string(),
        arguments: Some(json!({"clientID": "test"})),
    });

    transport.write_message(&request).await.unwrap();

    // Read it back (echoed)
    let response = transport.read_message().await.unwrap();

    match response {
        Message::Request(req) => {
            assert_eq!(req.seq, 1);
            assert_eq!(req.command, "initialize");
        }
        _ => panic!("Expected Request message"),
    }
}

/// Test 8: Ruby adapter - command and ID
#[test]
fn test_ruby_adapter_metadata() {
    assert_eq!(RubyAdapter::command(), "rdbg");
    assert_eq!(RubyAdapter::adapter_id(), "rdbg");
}

/// Test 9: Ruby adapter - launch args structure
#[test]
fn test_ruby_adapter_launch_args() {
    let program = "/workspace/fizzbuzz.rb";
    let args = vec!["100".to_string()];
    let cwd = Some("/workspace");

    let launch_args = RubyAdapter::launch_args_with_options(program, &args, cwd, true);

    assert_eq!(launch_args["request"], "launch");
    assert_eq!(launch_args["type"], "ruby");
    assert_eq!(launch_args["program"], program);
    assert_eq!(launch_args["args"], json!(args));
    assert_eq!(launch_args["stopOnEntry"], true);
    assert_eq!(launch_args["localfs"], true);
    assert_eq!(launch_args["cwd"], "/workspace");
}

/// Test 10: Ruby adapter - spawn with rdbg installed (requires Docker)
#[tokio::test]
#[ignore] // Requires rdbg to be installed
async fn test_ruby_adapter_spawn_real_rdbg() {
    use std::io::Write;

    // Create a minimal test Ruby script
    let test_script = "/tmp/test_ruby_dap.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "puts 'Hello from Ruby'").unwrap();
    writeln!(file, "sleep 1").unwrap();
    drop(file);

    // Spawn rdbg
    let result = RubyAdapter::spawn(test_script, &[], true).await;

    assert!(
        result.is_ok(),
        "Failed to spawn rdbg: {:?}",
        result.err()
    );

    let session = result.unwrap();

    // Verify port is in valid range
    assert!(session.port > 1024);

    // Verify socket is connected
    assert!(session.socket.peer_addr().is_ok());

    // Clean up
    std::fs::remove_file(test_script).ok();
}

/// Test 11: Ruby adapter - spawn timeout (port not listening)
#[tokio::test]
#[ignore] // Requires rdbg, tests failure case
async fn test_ruby_adapter_spawn_timeout() {
    // Try to spawn with a script that doesn't exist
    let result = RubyAdapter::spawn("/nonexistent/script.rb", &[], true).await;

    // Should fail (either spawn fails or socket timeout)
    assert!(result.is_err());
}

/// Test 12: End-to-end DAP communication with rdbg
#[tokio::test]
#[ignore] // Requires rdbg and fizzbuzz.rb
async fn test_ruby_e2e_dap_communication() {
    use debugger_mcp::dap::client::DapClient;
    use std::io::Write;

    // Create test script
    let test_script = "/tmp/test_ruby_e2e.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "def hello").unwrap();
    writeln!(file, "  puts 'Hello'").unwrap();
    writeln!(file, "end").unwrap();
    writeln!(file, "hello").unwrap();
    drop(file);

    // 1. Spawn rdbg
    let session = RubyAdapter::spawn(test_script, &[], true)
        .await
        .expect("Failed to spawn rdbg");

    // 2. Create DAP client from socket
    let client = DapClient::from_socket(session.socket)
        .await
        .expect("Failed to create DAP client");

    // 3. Send initialize request
    let init_response = client
        .send_request("initialize", Some(json!({
            "clientID": "test",
            "clientName": "Test Client",
            "adapterID": "rdbg",
            "linesStartAt1": true,
            "columnsStartAt1": true,
        })))
        .await
        .expect("Initialize request failed");

    assert!(init_response.success, "Initialize should succeed");
    assert!(init_response.body.is_some(), "Initialize should return capabilities");

    // 4. Send launch request
    let launch_response = client
        .send_request("launch", Some(json!({
            "program": test_script,
            "stopOnEntry": true,
            "localfs": true,
        })))
        .await
        .expect("Launch request failed");

    assert!(launch_response.success, "Launch should succeed");

    // Clean up
    std::fs::remove_file(test_script).ok();
}

/// Test 13: Ruby adapter - spawn with arguments
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_adapter_spawn_with_args() {
    use std::io::Write;

    // Create test script that uses ARGV
    let test_script = "/tmp/test_ruby_args.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "puts \"Args: #{{ARGV.inspect}}\"").unwrap();
    writeln!(file, "sleep 0.5").unwrap();
    drop(file);

    // Spawn with arguments
    let args = vec!["arg1".to_string(), "arg2".to_string()];
    let session = RubyAdapter::spawn(test_script, &args, false)
        .await
        .expect("Failed to spawn with args");

    assert!(session.socket.peer_addr().is_ok());

    // Clean up
    std::fs::remove_file(test_script).ok();
}

/// Test 14: Ruby adapter - verify --open flag is used
#[tokio::test]
#[ignore] // Requires rdbg and process inspection
async fn test_ruby_adapter_uses_open_flag() {
    use std::io::Write;

    let test_script = "/tmp/test_ruby_open.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "sleep 2").unwrap();
    drop(file);

    let _session = RubyAdapter::spawn(test_script, &[], true)
        .await
        .expect("Failed to spawn");

    // The spawned process should be listening on the port
    // (This is verified by the fact that we connected successfully)

    // Clean up
    std::fs::remove_file(test_script).ok();
}

/// Test 15: Performance - spawn and connect timing
#[tokio::test]
#[ignore] // Requires rdbg, performance test
async fn test_ruby_adapter_performance() {
    use std::io::Write;

    let test_script = "/tmp/test_ruby_perf.rb";
    let mut file = std::fs::File::create(test_script).unwrap();
    writeln!(file, "sleep 1").unwrap();
    drop(file);

    let start = std::time::Instant::now();
    let session = RubyAdapter::spawn(test_script, &[], true)
        .await
        .expect("Failed to spawn");

    let elapsed = start.elapsed();

    // Should connect within 2 seconds (our timeout)
    assert!(
        elapsed < Duration::from_secs(2),
        "Spawn and connect took too long: {:?}",
        elapsed
    );

    // In practice, should be much faster (< 500ms)
    println!("Spawn + connect time: {:?}", elapsed);

    // Clean up
    std::fs::remove_file(test_script).ok();
}
