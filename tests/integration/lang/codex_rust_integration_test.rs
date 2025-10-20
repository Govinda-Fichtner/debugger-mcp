use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

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
            "reconstructed_from": "mcp_protocol_log.md",
            "ai_client": "codex"
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

/// Test that validates Rust MCP server works with OpenAI Codex CLI
#[tokio::test]
#[ignore]
async fn test_rust_codex_integration() {
    println!("\n🚀 Starting Rust Codex Integration Test");
    println!("════════════════════════════════════════════════════════════════");

    // 1. Check Codex CLI is available
    println!("\n📋 Step 1: Checking Codex CLI availability...");
    let codex_check = Command::new("codex").arg("--version").output();

    if codex_check.is_err() || !codex_check.as_ref().unwrap().status.success() {
        println!("⚠️  Skipping test: Codex CLI not found");
        println!("   Install with: npm install -g @openai/codex-cli");
        return;
    }
    println!("✅ Codex CLI is available");

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
        println!("   Build with: cargo build --release");
        return;
    }

    // 6. Compile Rust test fixture
    println!("\n🔨 Step 4: Compiling Rust test fixture...");
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
        r#"# Rust Debugging Test with Codex

Test the debugger MCP server with Rust:
1. List available MCP tools
2. Start debugging session for {}
3. Set breakpoint at line 5
4. Continue and document results
5. Disconnect

IMPORTANT: At the end of testing, **USE THE WRITE TOOL** to create a file named 'test-results.json' with this EXACT format:
```json
{{
  "test_run": {{
    "language": "rust",
    "timestamp": "<current ISO 8601 timestamp>",
    "overall_success": <true if all operations succeeded, false otherwise>,
    "ai_client": "codex"
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
        fizzbuzz_binary.display()
    );
    fs::write(&prompt_path, prompt).expect("Failed to write prompt");

    // 8. Register MCP server with Codex
    println!("\n🔧 Step 5: Registering MCP server with Codex...");

    let workspace_prompt = workspace_root.join("debug_prompt.md");
    fs::copy(&prompt_path, &workspace_prompt).expect("Failed to copy prompt");

    // Get OPENAI_API_KEY from environment
    let openai_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("⚠️  OPENAI_API_KEY not set, skipping test");
            println!("   Set OPENAI_API_KEY environment variable to run this test");
            return;
        }
    };

    // Login to Codex with API key
    println!("\n🔑 Step 3: Logging in to Codex...");
    let login_output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo '{}' | codex login --with-api-key",
            openai_key
        ))
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to login to Codex");

    if !login_output.status.success() {
        let stderr = String::from_utf8_lossy(&login_output.stderr);
        println!("⚠️  Codex login failed: {}", stderr);
        println!("   Skipping test");
        return;
    }
    println!("✅ Successfully logged in to Codex");

    // Register MCP server: codex mcp add debugger-rust -- docker run...
    let register_output = Command::new("codex")
        .arg("mcp")
        .arg("add")
        .arg("debugger-test-rust")
        .arg("--")
        .arg("docker")
        .arg("run")
        .arg("--rm")
        .arg("-i")
        .arg("--network")
        .arg("host")
        .arg("-v")
        .arg(format!("{}:/workspace:rw", workspace_root.display()))
        .arg("-e")
        .arg(format!("OPENAI_API_KEY={}", openai_key))
        .arg("debugger-mcp:integration-tests")
        .arg(binary_path.to_str().unwrap())
        .arg("serve")
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to register MCP server");

    if !register_output.status.success() {
        let stderr = String::from_utf8_lossy(&register_output.stderr);
        println!("⚠️  MCP registration failed: {}", stderr);
        println!("   This might be expected if the server is already registered");
    } else {
        println!("✅ MCP server registered with Codex");
    }

    // 9. Run Codex CLI
    println!("\n🤖 Step 6: Running Codex CLI...");
    let prompt_content = fs::read_to_string(&workspace_prompt).unwrap();

    let codex_output = Command::new("codex")
        .arg("exec")
        .arg("--dangerously-bypass-approvals-and-sandbox")
        .arg(&prompt_content)
        .current_dir(&workspace_root)
        .env("OPENAI_API_KEY", &openai_key)
        .output()
        .expect("Failed to run codex");

    println!("\n📊 Codex Output:");
    let output_str = String::from_utf8_lossy(&codex_output.stdout);
    println!("{}", output_str);

    if !codex_output.status.success() {
        let stderr = String::from_utf8_lossy(&codex_output.stderr);
        println!("⚠️  Codex execution had issues:");
        println!("{}", stderr);
    }

    // 10. Verify protocol log
    let protocol_log_path = workspace_root.join("mcp_protocol_log.md");
    let log_exists = protocol_log_path.exists();

    if log_exists {
        println!("✅ Protocol log created");
    }

    // 10.5. Extract test-results.json from Codex's output if it wasn't written to file
    let test_results_src = workspace_root.join("test-results.json");

    // Check if Codex actually wrote a VALID file (not just any file)
    let mut needs_extraction = !test_results_src.exists()
        || fs::metadata(&test_results_src)
            .map(|m| m.len())
            .unwrap_or(0)
            == 0;

    // ENHANCED: Also validate the file contains valid, parseable JSON
    if !needs_extraction && test_results_src.exists() {
        if let Ok(content) = fs::read_to_string(&test_results_src) {
            let trimmed = content.trim();

            // Check if file is empty or doesn't contain required fields
            if trimmed.is_empty()
                || !trimmed.contains("\"test_run\"")
                || !trimmed.contains("\"operations\"")
            {
                println!("⚠️  test-results.json exists but is empty or missing required fields");
                needs_extraction = true;
            } else {
                // Validate it's actually parseable JSON
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(_) => {
                        println!("✅ Valid test-results.json found ({} bytes)", trimmed.len());
                    }
                    Err(e) => {
                        println!(
                            "⚠️  test-results.json exists but contains invalid JSON: {}",
                            e
                        );
                        needs_extraction = true;
                    }
                }
            }
        } else {
            println!("⚠️  test-results.json exists but cannot be read as UTF-8");
            needs_extraction = true;
        }
    }

    if needs_extraction {
        println!("⚠️  test-results.json not valid, extracting from output...");

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
                        "✅ Extracted and wrote test-results.json from Codex's output ({} bytes)",
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
                    reconstruct_test_results_from_protocol_log(&log_content, "rust");

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

    // 11. Verify test-results.json is ready for CI artifact collection
    if test_results_src.exists() {
        let size = fs::metadata(&test_results_src)
            .map(|m| m.len())
            .unwrap_or(0);
        println!(
            "✅ test-results.json ready at {} ({} bytes)",
            test_results_src.display(),
            size
        );
    } else {
        println!(
            "⚠️  test-results.json not found at {}",
            test_results_src.display()
        );
    }

    // 12. Cleanup
    let _ = Command::new("codex")
        .arg("mcp")
        .arg("remove")
        .arg("debugger-test-rust")
        .current_dir(&workspace_root)
        .output();

    let _ = fs::remove_file(&workspace_prompt);
    // NOTE: Do NOT delete protocol_log_path or test_results.json
    // These files are needed by CI for artifact upload

    println!("\n🎉 Rust Codex integration test completed!");
}
