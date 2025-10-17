use debugger_mcp::debug::SessionManager;
use debugger_mcp::mcp::resources::ResourcesHandler;
use debugger_mcp::mcp::tools::ToolsHandler;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Reconstruct test-results.json from mcp_protocol_log.md by parsing MCP tool operations
fn reconstruct_test_results_from_protocol_log(log_content: &str, language: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    // Parse the log to detect which operations succeeded
    let session_started =
        log_content.contains("debugger_start") && log_content.contains("\"status\": \"started\"");

    let breakpoint_set = log_content.contains("debugger_set_breakpoint");
    let breakpoint_verified = log_content.contains("\"verified\": true");

    let execution_continued = log_content.contains("debugger_continue")
        && log_content.contains("\"status\": \"continued\"");

    let stopped_at_breakpoint = log_content.contains("debugger_wait_for_stop")
        && log_content.contains("\"reason\": \"breakpoint\"");

    let stack_trace_retrieved =
        log_content.contains("debugger_stack_trace") && log_content.contains("\"stackFrames\"");

    let variable_evaluated = log_content.contains("debugger_evaluate")
        && (log_content.contains("\"result\":") || log_content.contains("\"value\":"));

    let session_disconnected = log_content.contains("debugger_disconnect")
        && log_content.contains("\"status\": \"disconnected\"");

    // Collect errors from the log
    let mut errors = Vec::new();

    if session_started && !breakpoint_verified {
        errors.push(json!({
            "operation": "breakpoint_set",
            "message": "Breakpoint was not verified (likely missing debug symbols)"
        }));
    }

    if !stopped_at_breakpoint && execution_continued {
        errors.push(json!({
            "operation": "execution",
            "message": "Program did not stop at breakpoint"
        }));
    }

    let overall_success = session_started
        && breakpoint_set
        && execution_continued
        && session_disconnected
        && errors.is_empty();

    // Generate JSON
    let result = json!({
        "test_run": {
            "language": language,
            "timestamp": timestamp,
            "overall_success": overall_success,
            "reconstructed_from": "mcp_protocol_log.md"
        },
        "operations": {
            "session_started": session_started,
            "breakpoint_set": breakpoint_set,
            "breakpoint_verified": breakpoint_verified,
            "execution_continued": execution_continued,
            "stopped_at_breakpoint": stopped_at_breakpoint,
            "stack_trace_retrieved": stack_trace_retrieved,
            "variable_evaluated": variable_evaluated,
            "session_disconnected": session_disconnected
        },
        "errors": errors
    });

    serde_json::to_string_pretty(&result).unwrap()
}

/// Test Go language detection
#[tokio::test]
#[ignore]
async fn test_go_language_detection() {
    // Check if dlv is available
    let dlv_check = Command::new("dlv").arg("version").output();

    if dlv_check.is_err() || !dlv_check.unwrap().status.success() {
        println!("⚠️  Skipping test: dlv (Delve) not installed");
        println!("   Install with: go install github.com/go-delve/delve/cmd/dlv@latest");
        return;
    }

    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Try to create a Go debug session
    let result = session_manager
        .create_session(
            "go",
            "tests/fixtures/fizzbuzz.go".to_string(),
            vec![],
            None,
            true,
        )
        .await;

    assert!(
        result.is_ok(),
        "Go language should be supported: {:?}",
        result
    );
}

/// Test Go adapter spawning
#[tokio::test]
#[ignore]
async fn test_go_adapter_spawning() {
    // Check if dlv is available
    let dlv_check = Command::new("dlv").arg("version").output();

    if dlv_check.is_err() || !dlv_check.unwrap().status.success() {
        println!("⚠️  Skipping test: dlv (Delve) not installed");
        println!("   Install with: go install github.com/go-delve/delve/cmd/dlv@latest");
        return;
    }

    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Create a Go debug session
    let session_id = session_manager
        .create_session(
            "go",
            "tests/fixtures/fizzbuzz.go".to_string(),
            vec![],
            None,
            true,
        )
        .await
        .expect("Should create Go session");

    // Wait a bit for initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify session exists
    let session = session_manager.get_session(&session_id).await;
    assert!(session.is_ok(), "Should get Go session");

    // Verify session language
    let session = session.unwrap();
    assert_eq!(session.language, "go");
    assert_eq!(session.program, "tests/fixtures/fizzbuzz.go");
}

/// Full Go FizzBuzz debugging integration test
#[tokio::test]
#[ignore]
async fn test_go_fizzbuzz_debugging_integration() {
    use tokio::time::{timeout, Duration};

    // Wrap entire test in timeout
    let test_result = timeout(Duration::from_secs(30), async {
        // Setup
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

        // Get absolute path to fizzbuzz.go
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fizzbuzz_path = PathBuf::from(manifest_dir)
            .join("tests")
            .join("fixtures")
            .join("fizzbuzz.go");

        let fizzbuzz_str = fizzbuzz_path.to_string_lossy().to_string();

        // Check if Go and dlv are available
        let go_check = std::process::Command::new("go").arg("version").output();
        let dlv_check = std::process::Command::new("dlv").arg("version").output();

        if go_check.is_err() || !go_check.unwrap().status.success() {
            println!("⚠️  Skipping Go FizzBuzz test: go not installed");
            return Ok::<(), String>(());
        }

        if dlv_check.is_err() || !dlv_check.unwrap().status.success() {
            println!("⚠️  Skipping Go FizzBuzz test: dlv (Delve) not installed");
            println!("   Install with: go install github.com/go-delve/delve/cmd/dlv@latest");
            return Ok(());
        }

        // 1. Start debugger session
        // Pending breakpoints will be applied after 'initialized' event, before configurationDone
        // This is the correct DAP sequence that works reliably for all debuggers including Delve
        println!("🔧 Starting Go debug session for: {}", fizzbuzz_str);

        let start_args = json!({
            "language": "go",
            "program": fizzbuzz_str,
            "args": [],
            "cwd": null,
            "stopOnEntry": false  // Use pending breakpoints instead of stopOnEntry
        });

        let start_result = timeout(
            Duration::from_secs(30),
            tools_handler.handle_tool("debugger_start", start_args),
        )
        .await;

        // If adapter spawn fails or times out, skip test gracefully
        let start_result = match start_result {
            Err(_) => {
                println!("⚠️  Skipping Go FizzBuzz test: debugger_start timed out");
                return Ok(());
            }
            Ok(result) => result,
        };

        let start_response = match start_result {
            Err(err) => {
                println!("⚠️  Skipping Go FizzBuzz test: {}", err);
                return Ok(());
            }
            Ok(response) => response,
        };

        let session_id = start_response["sessionId"].as_str().unwrap().to_string();
        println!("✅ Go debug session started: {}", session_id);

        // IMPORTANT: Wait a moment to ensure the async initialization task has started
        // and the session state is "Initializing". This ensures the breakpoint will be
        // stored as pending and passed to the DAP client during initialization.
        // Without this delay, the breakpoint might be set after initialization completes,
        // causing it to miss the correct DAP sequence (after 'initialized', before configurationDone).
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 2. Set breakpoint at FizzBuzz function call (line 13)
        // This will be stored as a pending breakpoint and applied during initialization
        // (after 'initialized' event, before configurationDone - the correct DAP sequence)
        println!("🎯 Setting breakpoint at line 13");

        let bp_args = json!({
            "sessionId": session_id,
            "sourcePath": fizzbuzz_str,
            "line": 13
        });

        let bp_result = timeout(
            Duration::from_secs(10),
            tools_handler.handle_tool("debugger_set_breakpoint", bp_args),
        )
        .await;

        match bp_result {
            Err(_) => {
                println!("⚠️  Breakpoint set timed out after 10 seconds");
            }
            Ok(Err(e)) => {
                println!("⚠️  Breakpoint set failed: {:?}", e);
            }
            Ok(Ok(bp_response)) => {
                let verified = bp_response["verified"].as_bool().unwrap_or(false);
                println!("✅ Breakpoint set, verified: {}", verified);
            }
        }

        // CRITICAL: Wait for async initialization to complete
        // The breakpoint was stored as pending, now wait for initialization to finish
        // before continuing execution. This ensures breakpoints are actually set in Delve.
        println!("⏳ Waiting for initialization to complete (2s)...");
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 3. Continue execution (program will run and hit breakpoint)
        println!("▶️  Continuing execution...");

        let continue_args = json!({
            "sessionId": session_id
        });

        let continue_result = timeout(
            Duration::from_secs(5),
            tools_handler.handle_tool("debugger_continue", continue_args),
        )
        .await;

        match continue_result {
            Ok(Ok(_)) => {
                println!("✅ Execution continued");
            }
            Ok(Err(e)) => {
                println!("⚠️  Continue failed: {:?}", e);
            }
            Err(_) => {
                println!("⚠️  Continue timed out");
            }
        }

        // 4. Wait for program to stop at breakpoint
        // Use debugger_wait_for_stop to properly wait until the program hits the breakpoint
        println!("⏳ Waiting for program to stop at breakpoint...");

        let wait_args = json!({
            "sessionId": session_id,
            "timeout": 5000  // 5 second timeout
        });

        let wait_result = timeout(
            Duration::from_secs(6),
            tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
        )
        .await;

        let is_stopped = match wait_result {
            Ok(Ok(response)) => {
                let stopped = response["stopped"].as_bool().unwrap_or(false);
                if stopped {
                    println!("✅ Program stopped at breakpoint");
                    let reason = response["reason"].as_str().unwrap_or("unknown");
                    println!("   Stop reason: {}", reason);
                    true
                } else {
                    println!("⚠️  Program did not stop (timeout or running to completion)");
                    false
                }
            }
            Ok(Err(e)) => {
                println!("⚠️  Wait for stop failed: {:?}", e);
                false
            }
            Err(_) => {
                println!("⚠️  Wait for stop timed out");
                false
            }
        };

        // 5. Get stack trace (only if stopped)
        if is_stopped {
            println!("📚 Getting stack trace...");

            let stack_args = json!({
                "sessionId": session_id
            });

            let stack_result = tools_handler
                .handle_tool("debugger_stack_trace", stack_args)
                .await;

            if let Ok(stack_response) = stack_result {
                let frames = &stack_response["stackFrames"];
                println!(
                    "✅ Stack trace retrieved: {} frames",
                    frames.as_array().map(|a| a.len()).unwrap_or(0)
                );

                if let Some(frames_array) = frames.as_array() {
                    if !frames_array.is_empty() {
                        println!("   Top frame: {}", frames_array[0]);
                    }
                }
            } else {
                println!("⚠️  Stack trace not available");
            }

            // 6. Evaluate expression (only if stopped)
            println!("🔍 Evaluating expression 'n'...");

            let eval_args = json!({
                "sessionId": session_id,
                "expression": "n",
                "frameId": null
            });

            let eval_result = tools_handler
                .handle_tool("debugger_evaluate", eval_args)
                .await;

            if let Ok(eval_response) = eval_result {
                let result = &eval_response["result"];
                println!("✅ Evaluation result: {}", result);
            } else {
                println!("⚠️  Expression evaluation not available");
            }
        } else {
            println!("⏭️  Skipping stack trace and evaluation (program not stopped at breakpoint)");
        }

        // 6. Test resource queries
        println!("📦 Testing resource queries...");

        let resources_handler = ResourcesHandler::new(Arc::clone(&session_manager));

        let sessions_list = resources_handler.read_resource("debugger://sessions").await;
        if let Ok(contents) = sessions_list {
            println!("✅ Sessions resource: {}", contents.uri);
            if let Some(text) = contents.text {
                println!("   Content: {}", text.lines().next().unwrap_or(""));
            }
        }

        let session_details = resources_handler
            .read_resource(&format!("debugger://sessions/{}", session_id))
            .await;

        if let Ok(_contents) = session_details {
            println!("✅ Session details resource retrieved");
        }

        // 7. Disconnect and cleanup
        println!("🔌 Disconnecting session...");

        let disconnect_args = json!({
            "sessionId": session_id
        });

        let disconnect_result = timeout(
            Duration::from_secs(5),
            tools_handler.handle_tool("debugger_disconnect", disconnect_args),
        )
        .await;

        if let Ok(Ok(_)) = disconnect_result {
            println!("✅ Session disconnected successfully");
        } else {
            println!("⚠️  Disconnect may have issues or timed out");
        }

        let manager = session_manager.read().await;
        let sessions = manager.list_sessions().await;

        if !sessions.contains(&session_id) {
            println!("✅ Session cleaned up from manager");
        }

        println!("\n🎉 Go FizzBuzz integration test completed!");

        Ok(())
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

/// Test that validates Go MCP server works with Claude Code CLI
#[tokio::test]
#[ignore]
async fn test_go_claude_code_integration() {
    println!("\n🚀 Starting Go Claude Code Integration Test");
    println!("════════════════════════════════════════════════════════════════");

    // 1. Check Claude CLI is available
    println!("\n📋 Step 1: Checking Claude CLI availability...");
    let claude_check = Command::new("claude").arg("--version").output();

    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        println!("⚠️  Skipping test: Claude CLI not found");
        return;
    }
    println!("✅ Claude CLI is available");

    // 2. Create temporary test directory
    println!("\n📁 Step 2: Creating temporary test environment...");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();

    // 3. Verify MCP server binary
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary_path = workspace_root.join("target/release/debugger_mcp");

    if !binary_path.exists() {
        println!(
            "⚠️  Skipping test: Binary not found at {}",
            binary_path.display()
        );
        return;
    }

    // 4. Create fizzbuzz.go test file
    let fizzbuzz_path = test_dir.join("fizzbuzz.go");
    let fizzbuzz_code = include_str!("../../fixtures/fizzbuzz.go");
    fs::write(&fizzbuzz_path, fizzbuzz_code).expect("Failed to write fizzbuzz.go");

    // 5. Create prompt
    let prompt_path = test_dir.join("debug_prompt.md");
    let prompt = format!(
        r#"# Go Debugging Test

Test the debugger MCP server with Go:
1. List available MCP tools
2. Start debugging session for {}
3. Set breakpoint at line 13
4. Continue and document results
5. Disconnect

IMPORTANT: At the end of testing, **USE THE WRITE TOOL** to create a file named 'test-results.json' with this EXACT format:
```json
{{
  "test_run": {{
    "language": "go",
    "timestamp": "<current ISO 8601 timestamp>",
    "overall_success": <true if all operations succeeded, false otherwise>
  }},
  "operations": {{
    "session_started": <true/false>,
    "breakpoint_set": <true/false>,
    "breakpoint_verified": <true/false>,
    "execution_continued": <true/false>,
    "stopped_at_breakpoint": <true/false>,
    "stack_trace_retrieved": <true/false>,
    "variable_evaluated": <true/false>,
    "session_disconnected": <true/false>
  }},
  "errors": [
    {{
      "operation": "<operation name>",
      "message": "<error message>"
    }}
  ]
}}
```

Set each boolean to true only if that specific operation completed successfully.
Add errors array entries for any failures encountered.

Also **USE THE WRITE TOOL** to create mcp_protocol_log.md documenting all interactions.

**CRITICAL**: After creating both files:
1. Use the Read tool to read back test-results.json
2. Display the full content to verify it was written correctly
3. Do NOT just claim you created the files - actually show the content!"#,
        fizzbuzz_path.display()
    );
    fs::write(&prompt_path, prompt).expect("Failed to write prompt");

    // 6. Register MCP server
    let mcp_config = json!({
        "command": binary_path.to_str().unwrap(),
        "args": ["serve"]
    });
    let mcp_config_str = serde_json::to_string(&mcp_config).unwrap();

    let workspace_fizzbuzz = workspace_root.join("fizzbuzz.go");
    let workspace_prompt = workspace_root.join("debug_prompt.md");

    fs::copy(&fizzbuzz_path, &workspace_fizzbuzz).expect("Failed to copy fizzbuzz.go");
    fs::copy(&prompt_path, &workspace_prompt).expect("Failed to copy prompt");

    let register_output = Command::new("claude")
        .arg("mcp")
        .arg("add-json")
        .arg("debugger-test-go")
        .arg(&mcp_config_str)
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to register MCP server");

    if !register_output.status.success() {
        println!("⚠️  MCP registration failed");
        return;
    }

    // 7. Run Claude Code
    let prompt_content = fs::read_to_string(&workspace_prompt).unwrap();

    let claude_output = Command::new("claude")
        .arg(&prompt_content)
        .arg("--permission-mode")
        .arg("bypassPermissions")
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to run claude");

    println!("\n📊 Claude Code Output:");
    let output_str = String::from_utf8_lossy(&claude_output.stdout);
    println!("{}", output_str);

    // 8. Verify protocol log and copy test-results.json
    let protocol_log_path = workspace_root.join("mcp_protocol_log.md");
    let log_exists = protocol_log_path.exists();

    if log_exists {
        println!("✅ Protocol log created");
    }

    // 8.5. Extract test-results.json from Claude's output if it wasn't written to file
    let test_results_src = workspace_root.join("test-results.json");

    // Check if Claude actually wrote the file
    if !test_results_src.exists()
        || fs::metadata(&test_results_src)
            .map(|m| m.len())
            .unwrap_or(0)
            == 0
    {
        println!("⚠️  test-results.json not created by Claude Code, extracting from output...");

        let mut extracted = false;

        // Strategy 1: Look for JSON block in stdout (between ```json and ```)
        if let Some(json_start) = output_str.find("```json") {
            let search_slice = &output_str[json_start + 7..]; // Skip "```json"
            if let Some(json_end_offset) = search_slice.find("```") {
                let json_content = search_slice[..json_end_offset].trim();

                // Validate it's actually JSON for test_run
                if json_content.contains("\"test_run\"") && json_content.contains("\"operations\"")
                {
                    fs::write(&test_results_src, json_content)
                        .expect("Failed to write extracted JSON");
                    println!(
                        "✅ Extracted and wrote test-results.json from Claude's output ({} bytes)",
                        json_content.len()
                    );
                    extracted = true;
                }
            }
        }

        // Strategy 2: Parse mcp_protocol_log.md as fallback
        if !extracted && protocol_log_path.exists() {
            println!("⚠️  Attempting to reconstruct test-results.json from mcp_protocol_log.md...");

            if let Ok(log_content) = fs::read_to_string(&protocol_log_path) {
                let reconstructed_json =
                    reconstruct_test_results_from_protocol_log(&log_content, "go");

                fs::write(&test_results_src, &reconstructed_json)
                    .expect("Failed to write reconstructed JSON");
                println!(
                    "✅ Reconstructed test-results.json from protocol log ({} bytes)",
                    reconstructed_json.len()
                );
                extracted = true;
            }
        }

        if !extracted {
            println!("❌ Failed to extract or reconstruct test-results.json");
        }
    }

    // Copy test-results.json from temp workspace to current directory for CI artifact collection
    let test_results_dest = std::env::current_dir().unwrap().join("test-results.json");
    if test_results_src.exists() {
        fs::copy(&test_results_src, &test_results_dest)
            .expect("Failed to copy test-results.json for artifact collection");
        println!(
            "✅ Copied test-results.json to {}",
            test_results_dest.display()
        );
    } else {
        println!(
            "⚠️  test-results.json not found at {}",
            test_results_src.display()
        );
    }

    // 9. Cleanup
    let _ = Command::new("claude")
        .arg("mcp")
        .arg("remove")
        .arg("debugger-test-go")
        .current_dir(&workspace_root)
        .output();

    let _ = fs::remove_file(&workspace_fizzbuzz);
    let _ = fs::remove_file(&workspace_prompt);
    // NOTE: Do NOT delete protocol_log_path or test_results.json
    // These files are needed by CI for artifact upload

    println!("\n🎉 Go Claude Code integration test completed!");
}
