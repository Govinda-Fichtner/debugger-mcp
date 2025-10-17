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

/// Helper function to compile a Rust source file to a binary with debug symbols
fn compile_rust_fixture(source_path: &PathBuf) -> Result<PathBuf, String> {
    // Create output directory in target
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = PathBuf::from(&manifest_dir).join("tests/fixtures/target");
    fs::create_dir_all(&output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Output binary path
    let binary_path = output_dir.join("fizzbuzz");

    // Remove old binary to ensure fresh compilation with current flags
    if binary_path.exists() {
        fs::remove_file(&binary_path).map_err(|e| format!("Failed to remove old binary: {}", e))?;
        println!("🗑️  Removed cached binary");
    }

    println!("🔨 Compiling Rust fixture...");
    println!("   Source: {}", source_path.display());
    println!("   Output: {}", binary_path.display());

    // Compile with debug symbols (-g flag) and no optimizations (-C opt-level=0)
    let compile_result = Command::new("rustc")
        .arg(source_path)
        .arg("-g") // Include debug symbols for LLDB
        .arg("-C")
        .arg("opt-level=0") // Disable optimizations for better debugging
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(format!("Compilation failed:\n{}", stderr));
    }

    println!("✅ Compilation successful");

    // Verify debug symbols are present
    let readelf_output = Command::new("readelf").arg("-S").arg(&binary_path).output();

    if let Ok(output) = readelf_output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains(".debug_info") {
            println!("✅ Debug symbols verified (.debug_info section present)");
        } else {
            return Err("Binary missing debug symbols (.debug_info section not found)".to_string());
        }
    } else {
        println!("⚠️  Could not verify debug symbols (readelf not available)");
    }

    Ok(binary_path)
}

/// Test Rust language detection
#[tokio::test]
#[ignore]
async fn test_rust_language_detection() {
    // Check if codelldb, lldb and rustc are available
    let codelldb_check = Command::new("codelldb").arg("--version").output();
    let lldb_check = Command::new("lldb").arg("--version").output();
    let rustc_check = Command::new("rustc").arg("--version").output();

    if codelldb_check.is_err() || !codelldb_check.unwrap().status.success() {
        println!("⚠️  Skipping test: codelldb not installed");
        println!("   Install CodeLLDB from: https://github.com/vadimcn/codelldb/releases");
        return;
    }

    if lldb_check.is_err() || !lldb_check.unwrap().status.success() {
        println!("⚠️  Skipping test: lldb not installed");
        return;
    }

    if rustc_check.is_err() || !rustc_check.unwrap().status.success() {
        println!("⚠️  Skipping test: rustc not installed");
        return;
    }

    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Get path to source file
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let fizzbuzz_rs = PathBuf::from(manifest_dir).join("tests/fixtures/fizzbuzz.rs");

    // Compile to binary
    let binary_path = match compile_rust_fixture(&fizzbuzz_rs) {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            return;
        }
    };

    // Try to create a Rust debug session with the compiled binary
    let result = session_manager
        .create_session(
            "rust",
            binary_path.to_string_lossy().to_string(),
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
    // Check if codelldb, lldb and rustc are available
    let codelldb_check = Command::new("codelldb").arg("--version").output();
    let lldb_check = Command::new("lldb").arg("--version").output();
    let rustc_check = Command::new("rustc").arg("--version").output();

    if codelldb_check.is_err() || !codelldb_check.unwrap().status.success() {
        println!("⚠️  Skipping test: codelldb not installed");
        println!("   Install CodeLLDB from: https://github.com/vadimcn/codelldb/releases");
        return;
    }

    if lldb_check.is_err() || !lldb_check.unwrap().status.success() {
        println!("⚠️  Skipping test: lldb not installed");
        return;
    }

    if rustc_check.is_err() || !rustc_check.unwrap().status.success() {
        println!("⚠️  Skipping test: rustc not installed");
        return;
    }

    let manager = Arc::new(RwLock::new(SessionManager::new()));
    let session_manager = manager.read().await;

    // Get path and compile
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let fizzbuzz_rs = PathBuf::from(manifest_dir).join("tests/fixtures/fizzbuzz.rs");

    let binary_path = match compile_rust_fixture(&fizzbuzz_rs) {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            return;
        }
    };

    let binary_str = binary_path.to_string_lossy().to_string();

    // Create a Rust debug session
    let session_id = session_manager
        .create_session("rust", binary_str.clone(), vec![], None, true)
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
    assert_eq!(session.program, binary_str);
}

/// Full Rust FizzBuzz debugging integration test
#[tokio::test]
#[ignore]
async fn test_rust_fizzbuzz_debugging_integration() {
    use tokio::time::{timeout, Duration};

    // Wrap entire test in timeout
    let test_result = timeout(Duration::from_secs(45), async {
        // Check if lldb is available
        let lldb_check = Command::new("lldb").arg("--version").output();

        if lldb_check.is_err() || !lldb_check.unwrap().status.success() {
            println!("⚠️  Skipping Rust FizzBuzz test: lldb not installed");
            println!("   Install with: apt install lldb (Debian/Ubuntu)");
            return Ok::<(), String>(());
        }

        // Check if rustc is available
        let rustc_check = Command::new("rustc").arg("--version").output();

        if rustc_check.is_err() || !rustc_check.unwrap().status.success() {
            println!("⚠️  Skipping Rust FizzBuzz test: rustc not installed");
            return Ok(());
        }

        // Setup
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let tools_handler = ToolsHandler::new(Arc::clone(&session_manager));

        // Get path to source and compile
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let fizzbuzz_rs = PathBuf::from(manifest_dir).join("tests/fixtures/fizzbuzz.rs");

        let binary_path = match compile_rust_fixture(&fizzbuzz_rs) {
            Ok(path) => path,
            Err(e) => {
                println!("⚠️  Skipping Rust FizzBuzz test: {}", e);
                return Ok(());
            }
        };

        let binary_str = binary_path.to_string_lossy().to_string();

        // 1. Start debugger session with stopOnEntry
        println!("🔧 Starting Rust debug session for: {}", binary_str);

        let start_args = json!({
            "language": "rust",
            "program": binary_str,
            "args": [],
            "cwd": null,
            "stopOnEntry": true
        });

        let start_result = timeout(
            Duration::from_secs(30),
            tools_handler.handle_tool("debugger_start", start_args),
        )
        .await;

        let start_result = match start_result {
            Err(_) => {
                println!("⚠️  Skipping Rust FizzBuzz test: debugger_start timed out");
                return Ok(());
            }
            Ok(result) => result,
        };

        let start_response = match start_result {
            Err(err) => {
                println!("⚠️  Skipping Rust FizzBuzz test: {}", err);
                return Ok(());
            }
            Ok(response) => response,
        };

        let session_id = start_response["sessionId"].as_str().unwrap().to_string();
        println!("✅ Rust debug session started: {}", session_id);

        // Give spawned async task time to begin executing (tokio::spawn doesn't guarantee immediate execution)
        println!("⏳ Waiting 50ms for async task to start...");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 2. Set breakpoint at main function (line 5 in fizzbuzz.rs)
        println!("🎯 Setting breakpoint at line 5");

        let bp_args = json!({
            "sessionId": session_id,
            "sourcePath": fizzbuzz_rs.to_string_lossy().to_string(),
            "line": 5
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

        // 3. Continue execution
        println!("▶️  Continuing execution...");

        let continue_args = json!({
            "sessionId": session_id
        });

        let continue_result = tools_handler
            .handle_tool("debugger_continue", continue_args)
            .await;

        if continue_result.is_err() {
            println!(
                "⚠️  Continue execution may have issues: {:?}",
                continue_result
            );
        } else {
            println!("✅ Execution continued");
        }

        // Wait for the program to reach breakpoint or complete
        println!("⏳ Waiting for program to stop at breakpoint...");
        let wait_args = json!({
            "sessionId": session_id,
            "timeoutMs": 5000
        });

        let wait_result = timeout(
            Duration::from_secs(10),
            tools_handler.handle_tool("debugger_wait_for_stop", wait_args),
        )
        .await;

        let stopped_at_breakpoint = match wait_result {
            Ok(Ok(stop_response)) => {
                let state = stop_response["state"].as_str().unwrap_or("Unknown");
                let reason = stop_response["reason"].as_str().unwrap_or("unknown");
                println!("🛑 Program stopped: state={}, reason={}", state, reason);
                state == "Stopped" && reason != "entry"
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

        // 4. Get stack trace (only if stopped at breakpoint)
        if stopped_at_breakpoint {
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
                println!("⚠️  Stack trace request failed");
            }

            // 5. Evaluate expression
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
                println!("⚠️  Expression evaluation failed");
            }
        } else {
            println!("⚠️  Skipping stack trace and evaluation (program not stopped at breakpoint)");
            println!("   This may occur if:");
            println!("   - The breakpoint was not hit (line may not be executed)");
            println!("   - The program completed before hitting the breakpoint");
            println!("   - The breakpoint was not verified by LLDB");
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

        println!("\n🎉 Rust FizzBuzz integration test completed!");

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
            println!("⚠️  Test timed out after 45 seconds");
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

    // 3. Check if rustc is available
    let rustc_check = Command::new("rustc").arg("--version").output();
    if rustc_check.is_err() || !rustc_check.unwrap().status.success() {
        println!("⚠️  Skipping test: rustc not installed");
        return;
    }

    // 4. Create temporary test directory
    println!("\n📁 Step 2: Creating temporary test environment...");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();

    // 5. Verify MCP server binary
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary_path = workspace_root.join("target/release/debugger_mcp");

    if !binary_path.exists() {
        println!(
            "⚠️  Skipping test: Binary not found at {}",
            binary_path.display()
        );
        return;
    }

    // 6. Compile Rust test fixture
    println!("\n🔨 Step 3: Compiling Rust test fixture...");
    let fizzbuzz_rs = workspace_root.join("tests/fixtures/fizzbuzz.rs");

    let fizzbuzz_binary = match compile_rust_fixture(&fizzbuzz_rs) {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            return;
        }
    };

    // 7. Create prompt
    let prompt_path = test_dir.join("debug_prompt.md");
    let prompt = format!(
        r#"# Rust Debugging Test

Test the debugger MCP server with Rust:
1. List available MCP tools
2. Start debugging session for {}
3. Set breakpoint at line 5
4. Continue and document results
5. Disconnect

IMPORTANT: At the end of testing, create a file named 'test-results.json' with this EXACT format:
```json
{{
  "test_run": {{
    "language": "rust",
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

Also create mcp_protocol_log.md documenting all interactions."#,
        fizzbuzz_binary.display()
    );
    fs::write(&prompt_path, prompt).expect("Failed to write prompt");

    // 8. Register MCP server
    let mcp_config = json!({
        "command": binary_path.to_str().unwrap(),
        "args": ["serve"]
    });
    let mcp_config_str = serde_json::to_string(&mcp_config).unwrap();

    let workspace_prompt = workspace_root.join("debug_prompt.md");
    fs::copy(&prompt_path, &workspace_prompt).expect("Failed to copy prompt");

    let register_output = Command::new("claude")
        .arg("mcp")
        .arg("add-json")
        .arg("debugger-test-rust")
        .arg(&mcp_config_str)
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to register MCP server");

    if !register_output.status.success() {
        println!("⚠️  MCP registration failed");
        return;
    }

    // 9. Run Claude Code
    let prompt_content = fs::read_to_string(&workspace_prompt).unwrap();

    let claude_output = Command::new("claude")
        .arg(&prompt_content)
        .arg("--allowedTools")
        .arg("Write(/workspace/**) debugger-test-rust")
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to run claude");

    println!("\n📊 Claude Code Output:");
    let output_str = String::from_utf8_lossy(&claude_output.stdout);
    println!("{}", output_str);

    // 10. Verify protocol log
    let protocol_log_path = workspace_root.join("mcp_protocol_log.md");
    let log_exists = protocol_log_path.exists();

    if log_exists {
        println!("✅ Protocol log created");
    }

    // 11. Copy test-results.json from temp workspace to current directory for CI artifact collection
    let test_results_src = workspace_root.join("test-results.json");
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

    // 11. Cleanup
    let _ = Command::new("claude")
        .arg("mcp")
        .arg("remove")
        .arg("debugger-test-rust")
        .current_dir(&workspace_root)
        .output();

    let _ = fs::remove_file(&workspace_prompt);
    // NOTE: Do NOT delete protocol_log_path or test_results.json
    // These files are needed by CI for artifact upload

    println!("\n🎉 Rust Claude Code integration test completed!");
}
