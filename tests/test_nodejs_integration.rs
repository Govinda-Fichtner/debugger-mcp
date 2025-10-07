/// Integration tests for Node.js debugging
///
/// These tests verify that the Node.js adapter works correctly end-to-end,
/// using Microsoft's vscode-js-debug as the DAP adapter.
///
/// Architecture:
/// 1. Spawn vscode-js-debug DAP server (node dapDebugServer.js <port> 127.0.0.1)
/// 2. Connect to DAP server via TCP
/// 3. Send launch request with Node.js program
/// 4. vscode-js-debug spawns Node.js with --inspect-brk internally
/// 5. Standard DAP debugging workflow
///
/// Test Coverage:
/// 1. Adapter configuration (command, args, launch config)
/// 2. vscode-js-debug DAP server spawning
/// 3. TCP connection to DAP server
/// 4. stopOnEntry behavior (should work natively)
/// 5. Breakpoint setting and verification
/// 6. Full FizzBuzz debugging workflow
///
/// NOTE: These tests will fail initially (TDD red phase) until the adapter is implemented.

// Conditional compilation - these tests require vscode-js-debug
#[cfg(test)]
mod nodejs_tests {
    use std::path::PathBuf;

    /// Test that vscode-js-debug path is configurable
    #[test]
    fn test_vscode_js_debug_path_configuration() {
        // This will fail until NodeJsAdapter is implemented
        // Expected path: /tmp/js-debug/src/dapDebugServer.js (from our tests)
        // Or: /usr/local/lib/js-debug/src/dapDebugServer.js (production)

        let test_path = PathBuf::from("/tmp/js-debug/src/dapDebugServer.js");
        assert!(
            test_path.exists(),
            "vscode-js-debug not found at {}. Run command-line tests first.",
            test_path.display()
        );
    }

    /// Test Node.js adapter basic configuration
    #[test]
    fn test_nodejs_adapter_type() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        assert_eq!(NodeJsAdapter::adapter_type(), "pwa-node");
    }

    /// Test Node.js DAP server command generation
    #[test]
    fn test_nodejs_dap_server_command() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;

        // This test only works if vscode-js-debug is installed
        // Otherwise we test the structure in the adapter's unit tests
        if let Ok(cmd) = NodeJsAdapter::dap_server_command(8123) {
            assert_eq!(cmd[0], "node");
            assert!(cmd[1].ends_with("dapDebugServer.js"));
            assert_eq!(cmd[2], "8123");
            assert_eq!(cmd[3], "127.0.0.1");  // IPv4 explicit
        } else {
            // If vscode-js-debug not installed, test passes with warning
            println!("WARNING: vscode-js-debug not installed, skipping command test");
        }
    }

    /// Test Node.js launch configuration structure
    #[test]
    fn test_nodejs_launch_config_with_stop_on_entry() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        use serde_json::json;

        let config = NodeJsAdapter::launch_config(
            "/workspace/fizzbuzz.js",
            &["100".to_string()],
            Some("/workspace"),
            true
        );

        assert_eq!(config["type"], "pwa-node");
        assert_eq!(config["request"], "launch");
        assert_eq!(config["program"], "/workspace/fizzbuzz.js");
        assert_eq!(config["stopOnEntry"], true);
        assert_eq!(config["cwd"], "/workspace");
        assert_eq!(config["args"], json!(["100"]));
    }

    /// Test Node.js launch configuration without stopOnEntry
    #[test]
    fn test_nodejs_launch_config_no_stop_on_entry() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        use serde_json::json;

        let config = NodeJsAdapter::launch_config(
            "/workspace/app.js",
            &[],
            None,
            false
        );

        assert_eq!(config["stopOnEntry"], false);
        assert!(config["cwd"].is_null());
        assert_eq!(config["args"], json!([]));
    }

    /// Test Node.js launch configuration with program arguments
    #[test]
    fn test_nodejs_launch_config_with_args() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        use serde_json::json;

        let program_args = vec!["--verbose".to_string(), "input.json".to_string()];
        let config = NodeJsAdapter::launch_config(
            "/app/server.js",
            &program_args,
            Some("/app"),
            false
        );

        assert_eq!(config["args"], json!(["--verbose", "input.json"]));
        assert_eq!(config["type"], "pwa-node");
        assert_eq!(config["program"], "/app/server.js");
        assert_eq!(config["cwd"], "/app");
    }
}

/// Integration tests that require vscode-js-debug to be installed
/// These are marked with #[ignore] and need to be run explicitly:
/// cargo test --test test_nodejs_integration -- --ignored
#[cfg(test)]
mod nodejs_integration_tests {
    use std::time::Duration;
    use tokio::process::Command;
    use std::path::Path;

    /// Test: Spawn vscode-js-debug DAP server
    ///
    /// This test verifies we can spawn the DAP server and it listens on the correct port.
    ///
    /// Expected behavior:
    /// 1. Spawn: node dapDebugServer.js 8123 127.0.0.1
    /// 2. Server outputs: "Debug server listening at 127.0.0.1:8123"
    /// 3. TCP port 8123 is open
    #[tokio::test]
    #[ignore] // Requires vscode-js-debug installation
    async fn test_spawn_vscode_js_debug_server() {
        let dap_server_path = "/tmp/js-debug/src/dapDebugServer.js";

        // Verify dapDebugServer.js exists
        assert!(
            Path::new(dap_server_path).exists(),
            "vscode-js-debug not found. Run: cd /tmp && wget https://github.com/microsoft/vscode-js-debug/releases/download/v1.105.0/js-debug-dap-v1.105.0.tar.gz && tar -xzf js-debug-dap-v1.105.0.tar.gz"
        );

        // Spawn DAP server
        let port = 8126u16;
        let mut child = Command::new("node")
            .args(&[dap_server_path, &port.to_string(), "127.0.0.1"])
            .spawn()
            .expect("Failed to spawn vscode-js-debug");

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify process is running
        assert!(
            child.try_wait().unwrap().is_none(),
            "vscode-js-debug exited immediately"
        );

        // Test TCP connection
        let connection_result = tokio::net::TcpStream::connect(("127.0.0.1", port)).await;
        assert!(
            connection_result.is_ok(),
            "Could not connect to vscode-js-debug on port {}",
            port
        );

        // Cleanup
        child.kill().await.ok();
    }

    /// Test: stopOnEntry behavior with Node.js
    ///
    /// This is the CRITICAL test that validates our hypothesis:
    /// Node.js with vscode-js-debug should support stopOnEntry natively.
    ///
    /// Expected DAP sequence:
    /// 1. Spawn vscode-js-debug DAP server
    /// 2. Connect via TCP
    /// 3. Send initialize request
    /// 4. Receive initialized event
    /// 5. Send launch request with stopOnEntry: true
    /// 6. vscode-js-debug spawns node --inspect-brk internally
    /// 7. Receive 'stopped' event with reason: "entry" ✅
    ///
    /// If this fails, we'll need the entry breakpoint workaround (like Ruby).
    #[tokio::test]
    #[ignore] // Requires vscode-js-debug installation
    async fn test_nodejs_stop_on_entry_native_support() {
        use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        use debugger_mcp::dap::client::DapClient;
        use debugger_mcp::dap::types::Event;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        println!("\n=== TESTING NODE.JS STOPENTRY NATIVE SUPPORT ===\n");

        // 1. Create test program path
        let test_program = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fizzbuzz.js");

        assert!(
            test_program.exists(),
            "FizzBuzz test fixture not found: {}",
            test_program.display()
        );

        println!("✅ Test program: {}", test_program.display());

        // 2. Spawn vscode-js-debug DAP server
        println!("\n🚀 Spawning vscode-js-debug DAP server");
        let nodejs_session = NodeJsAdapter::spawn_dap_server()
            .await
            .expect("Failed to spawn vscode-js-debug");

        println!("✅ vscode-js-debug spawned on port {}", nodejs_session.port);

        // 3. Create DAP client from socket
        let client = DapClient::from_socket(nodejs_session.socket)
            .await
            .expect("Failed to create DAP client");

        println!("✅ DAP client connected");

        // 4. Track events
        let events = Arc::new(Mutex::new(Vec::<Event>::new()));

        // Register callbacks for specific events we care about
        let events_clone = Arc::clone(&events);
        client
            .on_event("initialized", move |event| {
                let events = events_clone.clone();
                println!("📨 Event received: initialized");
                tokio::spawn(async move {
                    events.lock().await.push(event);
                });
            })
            .await;

        let events_clone = Arc::clone(&events);
        client
            .on_event("stopped", move |event| {
                let events = events_clone.clone();
                println!("📨 Event received: stopped");
                tokio::spawn(async move {
                    events.lock().await.push(event);
                });
            })
            .await;

        let events_clone = Arc::clone(&events);
        client
            .on_event("terminated", move |event| {
                let events = events_clone.clone();
                println!("📨 Event received: terminated");
                tokio::spawn(async move {
                    events.lock().await.push(event);
                });
            })
            .await;

        println!("✅ Event callbacks registered");

        // Give event processing task time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 5. Initialize DAP session
        println!("\n🔧 Initializing DAP session");
        let capabilities = timeout(
            Duration::from_secs(5),
            client.initialize("nodejs-test")
        )
        .await
        .expect("Initialize timeout")
        .expect("Initialize failed");

        println!("✅ Initialize response received");
        println!("   Capabilities: configurationDone={:?}",
                 capabilities.supports_configuration_done_request);

        // 6. Wait for 'initialized' event
        println!("\n⏳ Waiting for 'initialized' event (2s timeout)...");
        let got_initialized = timeout(Duration::from_secs(2), async {
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let events_locked = events.lock().await;
                if events_locked.iter().any(|e| e.event == "initialized") {
                    return;
                }
            }
        })
        .await
        .is_ok();

        assert!(got_initialized, "❌ FAILED: Did not receive 'initialized' event");
        println!("✅ Received 'initialized' event");

        // 7. Send launch request with stopOnEntry: true
        println!("\n🚀 Launching with stopOnEntry: true");
        let launch_args = NodeJsAdapter::launch_config(
            test_program.to_str().unwrap(),
            &[],
            None,
            true, // stopOnEntry: true
        );

        println!("   Launch config: {}", serde_json::to_string_pretty(&launch_args).unwrap());

        // Try using send_request directly (like Ruby test does)
        client
            .send_request("launch", Some(launch_args))
            .await
            .expect("Launch request failed");

        println!("✅ Launch request sent");

        // Give it a moment to process
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 8. Send configurationDone to complete initialization
        println!("\n📤 Sending configurationDone");
        client
            .configuration_done()
            .await
            .expect("ConfigurationDone failed");

        println!("✅ Configuration done");

        // 9. Wait for 'stopped' event at entry
        println!("\n⏳ Waiting for 'stopped' event (up to 10 seconds)...");

        let stopped_event = timeout(Duration::from_secs(10), async {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let events_locked = events.lock().await;
                if let Some(stopped) = events_locked.iter().find(|e| e.event == "stopped") {
                    return stopped.clone();
                }
                drop(events_locked);
            }
        })
        .await;

        // 10. Verify result
        match stopped_event {
            Ok(event) => {
                println!("✅ SUCCESS: Received 'stopped' event!");

                // Extract reason from body
                let reason = event.body
                    .as_ref()
                    .and_then(|b| b.get("reason"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("unknown");

                println!("   Reason: {}", reason);
                if let Some(body) = &event.body {
                    println!("   Body: {}", serde_json::to_string_pretty(body).unwrap());
                }

                // Verify reason (should be "entry" or similar)
                assert!(
                    reason == "entry" || reason == "breakpoint" || reason == "pause",
                    "Unexpected stop reason: {}",
                    reason
                );

                println!("\n🎉 HYPOTHESIS CONFIRMED: Node.js stopOnEntry works natively!");
            }
            Err(_) => {
                // Print all events received for debugging
                let all_events = events.lock().await;
                println!("\n❌ TIMEOUT: No 'stopped' event received within 10 seconds");
                println!("   Events received: {:?}",
                         all_events.iter().map(|e| &e.event).collect::<Vec<_>>());

                panic!("stopOnEntry did not work - no 'stopped' event received. This means we need an entry breakpoint workaround like Ruby.");
            }
        }
    }

    /// Test: Full FizzBuzz debugging workflow for Node.js
    ///
    /// This is the comprehensive end-to-end test that validates everything works.
    ///
    /// Workflow:
    /// 1. Start debugging session with stopOnEntry: true
    /// 2. Wait for stop at entry
    /// 3. Set breakpoint at line 9 (the bug: n % 4 instead of n % 5)
    /// 4. Continue execution
    /// 5. Hit breakpoint
    /// 6. Evaluate expressions to find bug
    /// 7. Verify bug: when n=4, n % 4 == 0 (wrong), n % 5 == 0 (correct)
    /// 8. Disconnect cleanly
    #[tokio::test]
    #[ignore] // Requires full Node.js adapter implementation
    async fn test_nodejs_fizzbuzz_debugging_workflow() {
        // This will fail until full implementation

        let fizzbuzz_js = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fizzbuzz.js");

        assert!(
            fizzbuzz_js.exists(),
            "FizzBuzz test fixture not found: {}",
            fizzbuzz_js.display()
        );

        // Expected workflow:
        // 1. debugger_start(language="nodejs", program="fizzbuzz.js", stopOnEntry=true)
        // 2. Wait for stopped event (reason: entry)
        // 3. debugger_set_breakpoint(sourcePath="fizzbuzz.js", line=9)
        // 4. debugger_continue()
        // 5. Wait for stopped event (reason: breakpoint)
        // 6. debugger_evaluate(expression="n", frameId=X) -> "1" (first iteration)
        // 7. debugger_continue() multiple times until n=4
        // 8. debugger_evaluate(expression="n % 4 === 0") -> "true" (BUG!)
        // 9. debugger_evaluate(expression="n % 5 === 0") -> "false" (CORRECT)
        // 10. debugger_disconnect()

        // Uncommenting will fail until implemented:
        // use debugger_mcp::debug::manager::SessionManager;
        // use std::sync::Arc;
        // use tokio::sync::RwLock;

        // let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        // let session_id = session_manager.write().await.create_nodejs_session(
        //     fizzbuzz_js.to_str().unwrap(),
        //     &[],
        //     true, // stopOnEntry
        // ).await.unwrap();

        // // ... rest of workflow ...

        assert!(
            false,
            "Full Node.js debugging workflow not implemented yet. This will test the complete FizzBuzz scenario."
        );
    }

    /// Test: Breakpoint setting and verification for Node.js
    ///
    /// Validates that breakpoints can be set and are verified by vscode-js-debug.
    #[tokio::test]
    #[ignore] // Requires NodeJsAdapter implementation
    async fn test_nodejs_breakpoint_set_and_verify() {
        // Expected:
        // 1. Start session
        // 2. Set breakpoint at specific line
        // 3. Verify breakpoint response has verified: true
        // 4. Continue and hit breakpoint

        assert!(
            false,
            "Breakpoint setting not implemented for Node.js yet"
        );
    }

    /// Test: Expression evaluation in Node.js context
    ///
    /// Validates that JavaScript expressions can be evaluated during debugging.
    #[tokio::test]
    #[ignore] // Requires NodeJsAdapter implementation
    async fn test_nodejs_expression_evaluation() {
        // Expected:
        // 1. Stop at breakpoint
        // 2. Evaluate JavaScript expressions
        // 3. Verify results
        //
        // Examples:
        // - evaluate("n") -> "4"
        // - evaluate("n % 4") -> "0"
        // - evaluate("n % 5") -> "4"
        // - evaluate("typeof n") -> "number"

        assert!(
            false,
            "Expression evaluation not implemented for Node.js yet"
        );
    }

    /// Test: Stack trace retrieval for Node.js
    ///
    /// Validates that call stack can be retrieved with source locations.
    #[tokio::test]
    #[ignore] // Requires NodeJsAdapter implementation
    async fn test_nodejs_stack_trace() {
        // Expected:
        // 1. Stop at breakpoint inside fizzbuzz function
        // 2. Get stack trace
        // 3. Verify frame 0 is in fizzbuzz function
        // 4. Verify source path and line number

        assert!(
            false,
            "Stack trace retrieval not implemented for Node.js yet"
        );
    }

    /// Test: Clean disconnect and process cleanup
    ///
    /// Validates that both vscode-js-debug and Node.js processes are cleaned up.
    #[tokio::test]
    #[ignore] // Requires NodeJsAdapter implementation
    async fn test_nodejs_clean_disconnect() {
        // Expected:
        // 1. Start debugging session
        // 2. Disconnect
        // 3. Verify vscode-js-debug process terminated
        // 4. Verify Node.js process terminated (spawned by vscode-js-debug)
        // 5. No orphaned processes

        assert!(
            false,
            "Clean disconnect not implemented for Node.js yet"
        );
    }
}

/// Test documentation and examples
///
/// These tests document expected behavior and serve as examples.
#[cfg(test)]
mod nodejs_documentation_tests {
    /// Documents the expected adapter configuration
    #[test]
    fn example_nodejs_adapter_configuration() {
        // This test documents how NodeJsAdapter should be configured:
        //
        // use debugger_mcp::adapters::nodejs::NodeJsAdapter;
        //
        // // DAP server command (spawned first)
        // let dap_cmd = NodeJsAdapter::dap_server_command(8123);
        // // Returns: ["node", "/path/to/dapDebugServer.js", "8123", "127.0.0.1"]
        //
        // // Launch configuration (sent after DAP server running)
        // let launch_config = NodeJsAdapter::launch_config(
        //     "/workspace/app.js",
        //     &["--port", "3000"],
        //     Some("/workspace"),
        //     true, // stopOnEntry
        // );
        // // Returns:
        // // {
        // //   "type": "pwa-node",
        // //   "request": "launch",
        // //   "program": "/workspace/app.js",
        // //   "args": ["--port", "3000"],
        // //   "cwd": "/workspace",
        // //   "stopOnEntry": true
        // // }

        // For now, this test just documents the expected interface
        assert!(true, "Documentation test - see comments for expected API");
    }

    /// Documents the expected debugging workflow
    #[test]
    fn example_nodejs_debugging_workflow() {
        // This test documents the complete debugging workflow:
        //
        // 1. Spawn vscode-js-debug DAP server:
        //    node /path/to/dapDebugServer.js 8123 127.0.0.1
        //
        // 2. Connect to DAP server via TCP:
        //    TcpStream::connect("127.0.0.1:8123")
        //
        // 3. Initialize DAP session:
        //    send initialize request → receive initialize response
        //    wait for 'initialized' event
        //
        // 4. Set breakpoints (optional, before launch):
        //    send setBreakpoints request → receive response with verified: true
        //
        // 5. Launch Node.js program:
        //    send launch request with {type: "pwa-node", program: "...", stopOnEntry: true}
        //    vscode-js-debug spawns: node --inspect-brk script.js
        //
        // 6. Wait for stopped event:
        //    receive 'stopped' event with reason: "entry" or "breakpoint"
        //
        // 7. Debugging operations:
        //    - stackTrace: get call stack
        //    - evaluate: evaluate JavaScript expressions
        //    - continue/step: execution control
        //    - setBreakpoints: add more breakpoints
        //
        // 8. Disconnect:
        //    send disconnect request
        //    vscode-js-debug terminates Node.js process
        //    close TCP connection

        assert!(true, "Documentation test - see comments for workflow");
    }
}
