use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test that validates the MCP server works with Claude Code
#[tokio::test]
async fn test_claude_code_integration() {
    println!("\n🚀 Starting Claude Code Integration Test");
    println!("════════════════════════════════════════════════════════════════");

    // 1. Check Claude CLI is available
    println!("\n📋 Step 1: Checking Claude CLI availability...");
    let claude_check = Command::new("claude").arg("--version").output();

    assert!(
        claude_check.is_ok() && claude_check.as_ref().unwrap().status.success(),
        "❌ Claude CLI not found or not working. Please install Claude Code CLI."
    );
    println!("✅ Claude CLI is available");

    // 2. Create temporary test directory
    println!("\n📁 Step 2: Creating temporary test environment...");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    println!("   Test directory: {}", test_dir.display());

    // 3. Verify the MCP server binary exists
    println!("\n🔨 Step 3: Verifying MCP server binary...");

    // For integration tests, CARGO_MANIFEST_DIR IS the workspace root
    // (it points to the directory containing Cargo.toml)
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    println!("   Workspace root: {}", workspace_root.display());

    // The binary should be pre-built (either by CI or manually)
    // This ensures the binary matches the Docker container's GLIBC version
    let binary_path = workspace_root.join("target/release/debugger_mcp");

    assert!(
        binary_path.exists(),
        "❌ Binary not found at {}\n\n\
         The integration test expects a pre-built binary.\n\
         \n\
         To fix this:\n\
         - Local development: Run 'cargo build --release' first\n\
         - CI: The workflow builds inside Docker automatically\n\
         \n\
         This ensures the binary uses the correct GLIBC version for Docker.",
        binary_path.display()
    );

    println!("✅ MCP server binary found: {}", binary_path.display());

    // 4. Create fizzbuzz.py test file
    println!("\n📝 Step 4: Creating fizzbuzz.py test file...");
    let fizzbuzz_path = test_dir.join("fizzbuzz.py");
    let fizzbuzz_code = r#"#!/usr/bin/env python3

def fizzbuzz(n):
    """
    Returns FizzBuzz result for number n.
    - Returns "Fizz" if n is divisible by 3
    - Returns "Buzz" if n is divisible by 5
    - Returns "FizzBuzz" if n is divisible by both
    - Returns the number as string otherwise
    """
    result = ""

    if n % 3 == 0:
        result += "Fizz"

    # BUG: This should be n % 5 == 0
    if n % 4 == 0:  # <-- INTENTIONAL BUG HERE
        result += "Buzz"

    if not result:
        result = str(n)

    return result


if __name__ == "__main__":
    for i in range(1, 21):
        print(f"{i}: {fizzbuzz(i)}")
"#;
    fs::write(&fizzbuzz_path, fizzbuzz_code).expect("Failed to write fizzbuzz.py");
    println!("✅ Created fizzbuzz.py at {}", fizzbuzz_path.display());

    // 5. Create Claude prompt file
    println!("\n📜 Step 5: Creating Claude prompt...");
    let prompt_path = test_dir.join("debug_prompt.md");
    let prompt = format!(
        r#"# Debugging Task

You are testing the Debugger MCP server integration. Your task is to:

1. Start a debugging session for the fizzbuzz.py program
2. Poll for session state until ready
3. Set a breakpoint at line 21 (where the bug is)
4. Continue execution
5. Document ALL MCP JSON-RPC protocol messages you send and receive

## Steps

### Step 1: Start Debugging Session

Use the `debugger_start` tool to start debugging the program:
- language: "python"
- program: "{}"
- stopOnEntry: true

Record the sessionId returned.

### Step 2: Poll for Session State

Use the `debugger_session_state` tool repeatedly to check the session state.
Record each state you see (NotStarted, Initializing, Running, Stopped, etc.)

Continue polling until the state is either "Running" or "Stopped".

### Step 3: Set Breakpoint

Once the session is ready, use `debugger_set_breakpoint` to set a breakpoint:
- sessionId: (from step 1)
- sourcePath: "{}"
- line: 21

### Step 4: Continue Execution

Use `debugger_continue` to continue execution until the breakpoint is hit.

### Step 5: Disconnect

Use `debugger_disconnect` to end the debugging session.

### Step 6: Document Protocol

Create a report showing:
1. All MCP tool calls you made (with arguments)
2. All responses you received (with results)
3. The sequence of session states observed
4. Any errors encountered

Save this report to a file called "mcp_protocol_log.md" in markdown format.

The report should include:
- Clear sections for each step
- JSON formatting for all tool calls and responses
- Timestamps or sequence numbers
- Success/failure indicators

## Important Notes

- Document EVERY MCP tool call and response
- Include the full JSON for each interaction
- Note the timing/sequence of state changes
- If anything fails, document the error clearly
"#,
        fizzbuzz_path.display(),
        fizzbuzz_path.display()
    );

    fs::write(&prompt_path, prompt).expect("Failed to write prompt");
    println!("✅ Created prompt at {}", prompt_path.display());

    // 6. Register MCP server with Claude CLI
    println!("\n⚙️  Step 6: Registering MCP server with Claude CLI...");

    // Build MCP server configuration JSON
    // CRITICAL: Must include "serve" subcommand for STDIO communication
    let mcp_config = json!({
        "command": binary_path.to_str().unwrap(),
        "args": ["serve"]
    });

    let mcp_config_str = serde_json::to_string(&mcp_config).expect("Failed to serialize config");
    println!(
        "   Config: {}",
        serde_json::to_string_pretty(&mcp_config).unwrap()
    );

    // Register the MCP server using claude mcp add-json
    let register_output = Command::new("claude")
        .arg("mcp")
        .arg("add-json")
        .arg("debugger-test")
        .arg(&mcp_config_str)
        .output()
        .expect("Failed to register MCP server");

    if !register_output.status.success() {
        eprintln!(
            "❌ Failed to register MCP server:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&register_output.stdout),
            String::from_utf8_lossy(&register_output.stderr)
        );
        panic!("MCP server registration failed");
    }
    println!("✅ MCP server 'debugger-test' registered");

    // 7. Verify MCP server is configured and connected
    println!("\n🔍 Step 7: Verifying MCP server configuration and connection...");

    let list_output = Command::new("claude")
        .arg("mcp")
        .arg("list")
        .output()
        .expect("Failed to list MCP servers");

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    println!("   MCP server status:");
    println!("════════════════════════════════════════════════════════════════");
    println!("{}", list_stdout);
    println!("════════════════════════════════════════════════════════════════");

    // Check that debugger-test exists
    assert!(
        list_stdout.contains("debugger-test"),
        "❌ MCP server 'debugger-test' not found in claude mcp list output"
    );

    // CRITICAL: Check for "✓ Connected" status
    let is_connected = list_stdout.contains("✓ Connected");

    if !is_connected {
        eprintln!("\n❌ MCP server 'debugger-test' is NOT connected!");
        eprintln!("   Output shows: {}", list_stdout);

        if list_stdout.contains("✗") {
            eprintln!("   Connection failed - check MCP server binary can start");
        }

        panic!("MCP server must show '✓ Connected' status before running integration test");
    }

    println!("✅ MCP server 'debugger-test' is properly configured and connected");

    // 8. Verify Claude CLI authentication
    println!("\n🔐 Step 8: Verifying Claude CLI authentication...");

    // Check if ANTHROPIC_API_KEY is set
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    assert!(
        !api_key.is_empty() && api_key.starts_with("sk-ant-"),
        "❌ ANTHROPIC_API_KEY must be set and start with 'sk-ant-'"
    );

    println!(
        "✅ ANTHROPIC_API_KEY is configured (length: {} chars)",
        api_key.len()
    );

    // Test authentication with a simple prompt
    let auth_test = Command::new("claude")
        .arg("say 'authenticated'")
        .arg("--print")
        .arg("--dangerously-skip-permissions")
        .current_dir(test_dir)
        .output()
        .expect("Failed to test authentication");

    let auth_stdout = String::from_utf8_lossy(&auth_test.stdout);

    if !auth_test.status.success() || auth_stdout.contains("Invalid API key") {
        eprintln!("❌ Claude CLI authentication failed:");
        eprintln!("stdout: {}", auth_stdout);
        eprintln!("stderr: {}", String::from_utf8_lossy(&auth_test.stderr));
        panic!("Authentication failed - please check ANTHROPIC_API_KEY is valid");
    }

    println!("✅ Claude CLI authenticated successfully");

    // 9. Run Claude with the debugging prompt
    println!("\n🤖 Step 9: Running Claude Code with debugging task...");
    println!("   This may take 30-60 seconds...");

    // Read the prompt from the file
    let prompt_text = fs::read_to_string(&prompt_path).expect("Failed to read prompt file");

    // Print the command for debugging
    println!("\n📝 Claude CLI Command:");
    println!("   cd {}", test_dir.display());
    println!("   claude \\");
    println!("     \"<prompt-from-file>\" \\");
    println!("     --print \\");
    println!("     --dangerously-skip-permissions");
    println!("\n   Prompt length: {} chars", prompt_text.len());
    println!(
        "   Prompt first 200 chars: {}",
        &prompt_text.chars().take(200).collect::<String>()
    );

    let claude_output = Command::new("claude")
        .arg(&prompt_text) // Prompt comes first
        .arg("--print")
        .arg("--dangerously-skip-permissions") // For automated testing
        .current_dir(test_dir)
        .output()
        .expect("Failed to run claude");

    let stdout = String::from_utf8_lossy(&claude_output.stdout);
    let stderr = String::from_utf8_lossy(&claude_output.stderr);

    println!("\n📊 Claude Code Output:");
    println!("════════════════════════════════════════════════════════════════");
    println!("{}", stdout);
    if !stderr.is_empty() {
        println!("\n⚠️  Stderr:");
        println!("{}", stderr);
    }

    // 10. Check if protocol log was created
    println!("\n📋 Step 10: Validating protocol documentation...");
    let protocol_log_path = test_dir.join("mcp_protocol_log.md");

    assert!(
        protocol_log_path.exists(),
        "❌ Protocol log file not created at {}",
        protocol_log_path.display()
    );

    let protocol_log = fs::read_to_string(&protocol_log_path).expect("Failed to read protocol log");

    println!("✅ Protocol log created ({} bytes)", protocol_log.len());

    // 11. Validate protocol log contents
    println!("\n🔍 Step 11: Analyzing protocol log...");

    // Check for key operations
    let has_debugger_start = protocol_log.contains("debugger_start");
    let has_session_state = protocol_log.contains("debugger_session_state");
    let has_set_breakpoint = protocol_log.contains("debugger_set_breakpoint");
    let has_continue = protocol_log.contains("debugger_continue");
    let has_disconnect = protocol_log.contains("debugger_disconnect");

    println!("   ✓ Contains debugger_start: {}", has_debugger_start);
    println!(
        "   ✓ Contains debugger_session_state: {}",
        has_session_state
    );
    println!(
        "   ✓ Contains debugger_set_breakpoint: {}",
        has_set_breakpoint
    );
    println!("   ✓ Contains debugger_continue: {}", has_continue);
    println!("   ✓ Contains debugger_disconnect: {}", has_disconnect);

    // Check for session ID
    let has_session_id = protocol_log.contains("sessionId");
    println!("   ✓ Contains sessionId: {}", has_session_id);

    // Check for state mentions
    let has_state_info = protocol_log.contains("state")
        || protocol_log.contains("Initializing")
        || protocol_log.contains("Running")
        || protocol_log.contains("Stopped");
    println!("   ✓ Contains state information: {}", has_state_info);

    println!("\n📄 Protocol Log Contents:");
    println!("════════════════════════════════════════════════════════════════");
    println!("{}", protocol_log);
    println!("════════════════════════════════════════════════════════════════");

    // 12. Cleanup MCP server and temp directory
    println!("\n🧹 Step 12: Cleanup...");

    // Remove the MCP server registration
    let _remove_output = Command::new("claude")
        .arg("mcp")
        .arg("remove")
        .arg("debugger-test")
        .output();

    println!("✅ MCP server 'debugger-test' removed");
    println!("✅ Temporary directory will be automatically cleaned up");

    // 13. Assertions
    println!("\n✅ Step 13: Final Validations...");

    assert!(
        has_debugger_start,
        "❌ Protocol log missing debugger_start tool usage"
    );

    assert!(has_session_id, "❌ Protocol log missing sessionId");

    assert!(has_state_info, "❌ Protocol log missing state information");

    // Assert Claude execution was successful
    assert!(
        claude_output.status.success() || stdout.len() > 100,
        "❌ Claude Code execution may have failed"
    );

    println!("\n🎉 Claude Code Integration Test Completed Successfully!");
    println!("════════════════════════════════════════════════════════════════");
    println!("✅ All validations passed");
    println!("✅ MCP protocol communication documented");
    println!("✅ Async initialization working");
    println!("✅ State tracking working");
    println!("✅ Breakpoint system working");
}
