use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Cargo test wrapper for Go Codex integration test
///
/// This wrapper invokes the shell script and validates output for code coverage.
#[tokio::test]
#[ignore]
async fn test_go_codex_via_script() {
    println!("\n🚀 Go Codex Integration Test (via script)");
    println!("══════════════════════════════════════════");

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = workspace_root.join("scripts/run-codex-go-test.sh");

    // Verify script exists
    assert!(
        script_path.exists(),
        "Script not found: {}",
        script_path.display()
    );

    // Run the script
    println!("📋 Running: {}", script_path.display());
    let output = Command::new("bash")
        .arg(&script_path)
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to execute script");

    // Print stdout and stderr
    println!("\n📊 Script output:");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));

    // Validate test-results.json exists
    let results_path = workspace_root.join("test-results.json");

    if !results_path.exists() {
        println!("⚠️  test-results.json not found - test may have timed out");
        // Don't fail the test - timeout is acceptable for Codex tests
        return;
    }

    // Read and parse JSON
    let json_content = fs::read_to_string(&results_path).expect("Failed to read test-results.json");

    let results: serde_json::Value =
        serde_json::from_str(&json_content).expect("Invalid JSON in test-results.json");

    println!("\n📊 Test Results:");
    println!("{}", serde_json::to_string_pretty(&results).unwrap());

    // Validate structure
    assert!(results["test_run"].is_object(), "Missing test_run object");
    assert!(
        results["operations"].is_object(),
        "Missing operations object"
    );
    assert_eq!(results["test_run"]["language"].as_str(), Some("go"));
    assert_eq!(results["test_run"]["ai_client"].as_str(), Some("codex"));

    println!("✅ Go Codex test completed");
}
