/// Simplified Codex Rust Integration Test
///
/// This test is a minimal wrapper around the shell script that performs the actual testing.
/// The shell script handles all the complexity of:
/// - Compiling the Rust fixture
/// - Logging into Codex
/// - Registering the MCP server
/// - Running Codex with the debugging prompt
/// - Validating output files
///
/// This wrapper exists primarily to:
/// - Allow `cargo test` to run the integration test
/// - Enable `cargo tarpaulin` to collect coverage from the MCP server
/// - Maintain compatibility with the existing CI workflow
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[tokio::test]
#[ignore]
async fn test_rust_codex_integration() {
    println!("\n🚀 Rust Codex Integration Test (Simplified Wrapper)");
    println!("════════════════════════════════════════════════════");

    // Get workspace root
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = workspace_root.join("scripts/run-codex-rust-test.sh");

    // Check if script exists
    if !script_path.exists() {
        println!(
            "⚠️  Skipping test: Script not found at {}",
            script_path.display()
        );
        println!("   Expected: scripts/run-codex-rust-test.sh");
        return;
    }

    // Check if OPENAI_API_KEY is set
    if std::env::var("OPENAI_API_KEY").is_err() {
        println!("⚠️  Skipping test: OPENAI_API_KEY not set");
        println!("   Set OPENAI_API_KEY environment variable to run this test");
        return;
    }

    println!("✅ Prerequisites checked");
    println!();

    // Run the shell script
    println!("📝 Running shell script: {}", script_path.display());
    println!();

    let output = Command::new(&script_path)
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to execute shell script");

    // Print stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        println!("📊 Script output:");
        println!("{}", stdout);
    }

    if !stderr.is_empty() {
        println!("⚠️  Script stderr:");
        println!("{}", stderr);
    }

    println!();
    println!("📊 Script exit code: {}", output.status);

    // Validate output files exist
    let test_results_path = workspace_root.join("test-results.json");
    let protocol_log_path = workspace_root.join("mcp_protocol_log.md");

    let results_exist = test_results_path.exists();
    let log_exists = protocol_log_path.exists();

    if results_exist {
        let size = fs::metadata(&test_results_path)
            .map(|m| m.len())
            .unwrap_or(0);
        println!("✅ test-results.json exists ({} bytes)", size);

        // Read and validate JSON
        if let Ok(content) = fs::read_to_string(&test_results_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                println!("✅ test-results.json is valid JSON");

                // Check for required fields
                if let Some(test_run) = json.get("test_run") {
                    if let Some(success) = test_run.get("overall_success") {
                        println!("   overall_success: {}", success);
                    }
                }
            } else {
                println!("⚠️  test-results.json contains invalid JSON");
            }
        }
    } else {
        println!("❌ test-results.json not found");
    }

    if log_exists {
        let size = fs::metadata(&protocol_log_path)
            .map(|m| m.len())
            .unwrap_or(0);
        println!("✅ mcp_protocol_log.md exists ({} bytes)", size);
    } else {
        println!("⚠️  mcp_protocol_log.md not found");
    }

    println!();

    // Assert test passed
    assert!(
        output.status.success(),
        "Shell script failed with exit code: {}",
        output.status
    );

    assert!(
        results_exist,
        "test-results.json was not created by the script"
    );

    println!("🎉 Rust Codex integration test completed successfully!");
}
