# Go (Delve) Debug Adapter - Testing Strategy

**Research Date**: October 8, 2025
**Purpose**: Incremental validation plan for Go/Delve adapter implementation

## Philosophy: Test-Driven Development with Incremental Validation

This strategy follows a **proof-of-concept → unit test → integration test** progression. Each test validates one assumption before moving to the next layer of complexity.

**Key Principle**: Don't jump to conclusions without real proof that things work as expected.

---

## Phase 0: Environment Setup and Prerequisites

### Prerequisites Validation

**Test 0.1: Verify Go Installation**
```bash
# What we're testing: Go compiler is available
go version

# Expected output: go version go1.21+ <platform>
```

**Test 0.2: Verify Delve Installation**
```bash
# What we're testing: Delve debugger is installed and working
go install github.com/go-delve/delve/cmd/dlv@latest
dlv version

# Expected output: Delve Debugger, Version: X.X.X, Build: ...
```

**Test 0.3: Manual DAP Connection Test**
```bash
# What we're testing: Can we manually connect to dlv dap?

# Terminal 1: Start dlv dap manually
dlv dap --listen=127.0.0.1:12345

# Terminal 2: Test connection
nc -v 127.0.0.1 12345

# Expected: Connection established
# Then send initialize request:
echo 'Content-Length: 123\r\n\r\n{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"test","adapterID":"go"}}' | nc 127.0.0.1 12345

# Expected: JSON response with capabilities
```

**Success Criteria**: All three tests pass before writing any Rust code.

---

## Phase 1: Basic Process Management Tests

### Test 1.1: Spawn Delve Process

**File**: `tests/test_golang_basic.rs`

**What we're testing**: Can we spawn `dlv dap` as a subprocess?

```rust
#[tokio::test]
async fn test_spawn_dlv_process() {
    use std::process::{Command, Stdio};

    let child = Command::new("dlv")
        .args(&["dap", "--listen=127.0.0.1:0"])  // Port 0 = OS assigns port
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    assert!(child.is_ok(), "Failed to spawn dlv process");

    let mut child = child.unwrap();

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify it's still running
    assert!(child.try_wait().unwrap().is_none(), "dlv exited immediately");

    // Clean up
    child.kill().unwrap();
}
```

**Expected Result**: Process spawns successfully and stays alive.

**If this fails**: Check if dlv is in PATH, check permissions.

---

### Test 1.2: Port Allocation

**What we're testing**: Can we allocate a free port for dlv?

```rust
#[test]
fn test_find_free_port_for_dlv() {
    use debugger_mcp::dap::socket_helper;

    let port = socket_helper::find_free_port();
    assert!(port.is_ok(), "Failed to find free port");

    let port = port.unwrap();
    assert!(port > 1024, "Port should be > 1024 (non-privileged)");
    assert!(port < 65535, "Port should be valid u16");
}
```

**Expected Result**: Valid port number returned.

---

### Test 1.3: Spawn on Specific Port

**What we're testing**: Can we spawn dlv on a specific port we control?

```rust
#[tokio::test]
async fn test_spawn_dlv_on_specific_port() {
    use debugger_mcp::dap::socket_helper;
    use std::process::{Command, Stdio};

    let port = socket_helper::find_free_port().unwrap();

    let child = Command::new("dlv")
        .args(&[
            "dap",
            "--listen",
            &format!("127.0.0.1:{}", port),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    assert!(child.is_ok(), "Failed to spawn dlv with specific port");

    let mut child = child.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify port is listening
    let connection = tokio::net::TcpStream::connect(("127.0.0.1", port)).await;
    assert!(connection.is_ok(), "Failed to connect to dlv port");

    child.kill().unwrap();
}
```

**Expected Result**: Can connect to the specified port.

---

## Phase 2: Socket Connection Tests

### Test 2.1: Connect with Retry

**What we're testing**: Can we reliably connect to dlv with retry logic?

```rust
#[tokio::test]
async fn test_connect_to_dlv_with_retry() {
    use debugger_mcp::dap::socket_helper;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let port = socket_helper::find_free_port().unwrap();

    let _child = Command::new("dlv")
        .args(&["dap", "--listen", &format!("127.0.0.1:{}", port)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Connect with retry (dlv takes a moment to bind)
    let stream = socket_helper::connect_with_retry(port, Duration::from_secs(3)).await;

    assert!(stream.is_ok(), "Failed to connect to dlv with retry");
}
```

**Expected Result**: Connection established within timeout.

---

### Test 2.2: Connection Timeout

**What we're testing**: Does connection timeout work correctly?

```rust
#[tokio::test]
async fn test_connection_timeout_to_nonexistent_dlv() {
    use debugger_mcp::dap::socket_helper;
    use std::time::Duration;

    let port = socket_helper::find_free_port().unwrap();
    // Don't spawn dlv - port should be free

    let result = socket_helper::connect_with_retry(port, Duration::from_millis(500)).await;

    assert!(result.is_err(), "Should timeout when dlv is not running");
}
```

**Expected Result**: Timeout error after 500ms.

---

## Phase 3: DAP Protocol Communication Tests

### Test 3.1: Send Initialize Request

**What we're testing**: Can we send a valid DAP initialize request?

```rust
#[tokio::test]
async fn test_send_initialize_request() {
    use debugger_mcp::dap::{DapClient, InitializeRequestArguments};

    // Spawn dlv and connect...
    let port = spawn_dlv_and_get_port().await;
    let stream = connect_to_dlv(port).await.unwrap();

    let client = DapClient::new(stream);

    let args = InitializeRequestArguments {
        client_id: Some("debugger-mcp-test".to_string()),
        client_name: Some("Debugger MCP Test".to_string()),
        adapter_id: "go".to_string(),
        locale: Some("en-US".to_string()),
        lines_start_at_1: Some(true),
        columns_start_at_1: Some(true),
        path_format: Some("path".to_string()),
        supports_variable_type: Some(true),
        supports_variable_paging: Some(false),
        supports_run_in_terminal_request: Some(false),
        supports_memory_references: Some(false),
        supports_progress_reporting: Some(false),
        supports_invalidated_event: Some(false),
    };

    let response = client.initialize(args).await;

    assert!(response.is_ok(), "Initialize request failed");

    let capabilities = response.unwrap();
    assert!(capabilities.supports_configuration_done_request.unwrap_or(false));
}
```

**Expected Result**: Valid capabilities response from dlv.

---

### Test 3.2: Launch Configuration

**What we're testing**: Can we send a launch request with Go program?

```rust
#[tokio::test]
async fn test_launch_go_program() {
    use debugger_mcp::dap::DapClient;
    use serde_json::json;

    // Setup: Create simple Go program
    let temp_dir = tempfile::tempdir().unwrap();
    let program_path = temp_dir.path().join("hello.go");
    std::fs::write(
        &program_path,
        r#"
package main
import "fmt"
func main() {
    fmt.Println("Hello, World!")
}
"#,
    )
    .unwrap();

    // Spawn dlv and initialize...
    let client = setup_dlv_client().await;

    let launch_args = json!({
        "type": "go",
        "request": "launch",
        "mode": "debug",
        "program": program_path.to_str().unwrap(),
        "stopOnEntry": true,
    });

    let response = client.launch(launch_args).await;

    assert!(response.is_ok(), "Launch request failed");
}
```

**Expected Result**: Launch succeeds, program stops at entry.

---

## Phase 4: Breakpoint and Execution Control Tests

### Test 4.1: Set Breakpoint

**What we're testing**: Can we set a source breakpoint?

```rust
#[tokio::test]
async fn test_set_breakpoint_in_go_code() {
    use debugger_mcp::dap::{DapClient, SourceBreakpoint};

    let temp_dir = tempfile::tempdir().unwrap();
    let program_path = temp_dir.path().join("test.go");
    std::fs::write(
        &program_path,
        r#"
package main
import "fmt"
func main() {
    x := 1      // Line 4
    y := 2      // Line 5
    z := x + y  // Line 6
    fmt.Println(z)
}
"#,
    )
    .unwrap();

    let client = setup_dlv_client().await;
    client.launch_program(&program_path).await.unwrap();

    // Set breakpoint at line 6
    let breakpoints = vec![SourceBreakpoint {
        line: 6,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
    }];

    let response = client
        .set_breakpoints(program_path.to_str().unwrap(), breakpoints)
        .await;

    assert!(response.is_ok(), "Failed to set breakpoint");

    let breakpoints = response.unwrap();
    assert_eq!(breakpoints.len(), 1);
    assert!(breakpoints[0].verified, "Breakpoint not verified");
}
```

**Expected Result**: Breakpoint set and verified.

---

### Test 4.2: Continue to Breakpoint

**What we're testing**: Can we continue execution and hit breakpoint?

```rust
#[tokio::test]
async fn test_continue_to_breakpoint() {
    use debugger_mcp::dap::DapClient;

    let client = setup_program_with_breakpoint().await;

    // Continue execution
    let response = client.continue_execution().await;
    assert!(response.is_ok(), "Continue failed");

    // Wait for stopped event
    let event = client.wait_for_event("stopped", Duration::from_secs(5)).await;
    assert!(event.is_ok(), "Didn't receive stopped event");

    let stopped_event = event.unwrap();
    assert_eq!(stopped_event.reason, "breakpoint");
}
```

**Expected Result**: Program stops at breakpoint.

---

### Test 4.3: Step Operations

**What we're testing**: Can we step over/into/out?

```rust
#[tokio::test]
async fn test_step_operations() {
    let client = setup_at_breakpoint().await;

    // Get current stack frame
    let stack_trace = client.stack_trace().await.unwrap();
    let frame = &stack_trace.stack_frames[0];
    let initial_line = frame.line;

    // Step over
    client.step_over().await.unwrap();
    client.wait_for_stopped().await.unwrap();

    // Verify we moved to next line
    let stack_trace = client.stack_trace().await.unwrap();
    let new_line = stack_trace.stack_frames[0].line;
    assert!(new_line > initial_line, "Step didn't advance line");
}
```

**Expected Result**: Stepping advances through code.

---

## Phase 5: Variable Inspection Tests

### Test 5.1: Inspect Variables

**What we're testing**: Can we read variable values?

```rust
#[tokio::test]
async fn test_inspect_variables() {
    let program = r#"
package main
func main() {
    x := 42
    y := "hello"
    z := 3.14
    _ = x + int(z)
}
"#;

    let client = setup_and_stop_at_line(program, 6).await;

    // Get variables in current frame
    let stack_trace = client.stack_trace().await.unwrap();
    let frame_id = stack_trace.stack_frames[0].id;

    let scopes = client.scopes(frame_id).await.unwrap();
    let locals_scope = scopes.iter().find(|s| s.name == "Local").unwrap();

    let variables = client.variables(locals_scope.variables_reference).await.unwrap();

    // Verify we can see x, y, z
    assert!(variables.iter().any(|v| v.name == "x" && v.value == "42"));
    assert!(variables.iter().any(|v| v.name == "y" && v.value == "\"hello\""));
    assert!(variables.iter().any(|v| v.name == "z" && v.value == "3.14"));
}
```

**Expected Result**: Can inspect variable values.

---

### Test 5.2: Evaluate Expression

**What we're testing**: Can we evaluate expressions in debug context?

```rust
#[tokio::test]
async fn test_evaluate_expression() {
    let client = setup_with_variables().await;

    let result = client.evaluate("x + 10", frame_id, "watch").await;

    assert!(result.is_ok(), "Evaluation failed");
    assert_eq!(result.unwrap().result, "52");
}
```

**Expected Result**: Expression evaluated correctly.

---

## Phase 6: Adapter Integration Tests

### Test 6.1: Adapter Trait Implementation

**What we're testing**: Does our adapter implement the required trait?

```rust
#[test]
fn test_go_adapter_trait_implementation() {
    use debugger_mcp::adapters::golang::GoAdapter;
    use debugger_mcp::adapters::logging::DebugAdapterLogger;

    let adapter = GoAdapter;

    assert_eq!(adapter.language_name(), "Go");
    assert_eq!(adapter.language_emoji(), "🐹");
    assert_eq!(adapter.transport_type(), "TCP Socket");
    assert_eq!(adapter.adapter_id(), "delve");
    assert!(!adapter.requires_workaround());

    // Test logging methods don't panic
    adapter.log_selection();
    adapter.log_transport_init();
    adapter.log_spawn_attempt();
    adapter.log_connection_success();
    adapter.log_shutdown();
}
```

**Expected Result**: Trait methods work correctly.

---

### Test 6.2: Full Spawn Workflow

**What we're testing**: Complete adapter spawn function.

```rust
#[tokio::test]
async fn test_go_adapter_spawn() {
    use debugger_mcp::adapters::golang;

    let temp_dir = tempfile::tempdir().unwrap();
    let program_path = temp_dir.path().join("main.go");
    std::fs::write(&program_path, "package main\nfunc main() {}\n").unwrap();

    let session = golang::spawn(
        program_path.to_str().unwrap(),
        &[],
        true, // stop_on_entry
    )
    .await;

    assert!(session.is_ok(), "Adapter spawn failed");

    let session = session.unwrap();
    assert!(session.process.id().is_some(), "Process not running");
    assert!(session.port > 0, "Invalid port");
}
```

**Expected Result**: Full spawn workflow succeeds.

---

## Phase 7: FizzBuzz Integration Test

### Test 7.1: Complete FizzBuzz Debugging Session

**What we're testing**: End-to-end debugging of real Go program.

**File**: `tests/fixtures/fizzbuzz.go`
```go
package main

import "fmt"

func fizzbuzz(n int) string {
    if n%15 == 0 {
        return "FizzBuzz"
    } else if n%3 == 0 {
        return "Fizz"
    } else if n%5 == 0 {
        return "Buzz"
    } else {
        return fmt.Sprintf("%d", n)
    }
}

func main() {
    for i := 1; i <= 100; i++ {
        result := fizzbuzz(i)
        fmt.Println(result)
    }
}
```

**Test**: `tests/test_golang_fizzbuzz.rs`
```rust
#[tokio::test]
async fn test_fizzbuzz_debugging_go() {
    use debugger_mcp::adapters::golang;
    use debugger_mcp::dap::DapClient;

    // 1. Start debugger
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fizzbuzz.go");
    let session = golang::spawn(fixture_path, &[], false)
        .await
        .expect("Failed to spawn Go debugger");

    let client = DapClient::new(session.socket);

    // 2. Initialize
    client
        .initialize_default()
        .await
        .expect("Failed to initialize");

    // 3. Set breakpoint in fizzbuzz function (line 6)
    let bp = client
        .set_breakpoints(fixture_path, vec![SourceBreakpoint { line: 6, ..Default::default() }])
        .await
        .expect("Failed to set breakpoint");

    assert!(bp[0].verified, "Breakpoint not verified");

    // 4. Launch
    client
        .launch_go_program(fixture_path)
        .await
        .expect("Failed to launch");

    // 5. Continue and wait for breakpoint
    client.continue_execution().await.expect("Continue failed");
    let stopped = client.wait_for_stopped_event().await.expect("Didn't stop");
    assert_eq!(stopped.reason, "breakpoint");

    // 6. Inspect variable 'n'
    let variables = client.get_local_variables().await.expect("Failed to get variables");
    let n = variables.iter().find(|v| v.name == "n").expect("Variable 'n' not found");
    let n_value: i32 = n.value.parse().expect("Invalid value");
    assert!(n_value >= 1 && n_value <= 100, "Unexpected value for n");

    // 7. Step over
    client.step_over().await.expect("Step failed");
    client.wait_for_stopped_event().await.expect("Didn't stop after step");

    // 8. Continue to completion
    client.continue_execution().await.expect("Continue failed");
    let terminated = client.wait_for_terminated_event().await;
    assert!(terminated.is_ok(), "Program didn't terminate cleanly");

    // 9. Cleanup
    drop(client);
    drop(session);
}
```

**Expected Result**: Complete debugging session from launch to termination.

---

## Phase 8: Comparison with Existing Adapters

### Test 8.1: Ruby vs Go Adapter Pattern

**What we're testing**: Go adapter follows same pattern as Ruby.

```rust
#[tokio::test]
async fn test_go_adapter_matches_ruby_pattern() {
    use debugger_mcp::adapters::{golang, ruby};

    // Both should use TCP Socket
    assert_eq!(golang::GoAdapter.transport_type(), ruby::RubyAdapter.transport_type());

    // Both should spawn successfully
    let go_result = golang::spawn("test.go", &[], true).await;
    let ruby_result = ruby::spawn("test.rb", &[], true).await;

    // Pattern is identical (both succeed or both fail due to missing files)
    assert_eq!(go_result.is_err(), ruby_result.is_err());
}
```

---

## Phase 9: Error Handling Tests

### Test 9.1: Missing Delve Binary

**What we're testing**: Graceful error when dlv not installed.

```rust
#[tokio::test]
async fn test_error_when_dlv_not_found() {
    use debugger_mcp::adapters::golang;
    use std::env;

    // Temporarily clear PATH
    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", "");

    let result = golang::spawn("test.go", &[], true).await;

    // Restore PATH
    env::set_var("PATH", &original_path);

    assert!(result.is_err(), "Should error when dlv not in PATH");
    assert!(result.unwrap_err().to_string().contains("dlv"));
}
```

---

### Test 9.2: Invalid Go File

**What we're testing**: Error handling for syntax errors.

```rust
#[tokio::test]
async fn test_error_with_invalid_go_syntax() {
    let temp_dir = tempfile::tempdir().unwrap();
    let program_path = temp_dir.path().join("invalid.go");
    std::fs::write(&program_path, "this is not valid Go code").unwrap();

    let result = golang::spawn(program_path.to_str().unwrap(), &[], true).await;

    // Should either fail to spawn or fail at launch
    // Either way, we should get a clear error
    if result.is_ok() {
        let session = result.unwrap();
        let client = DapClient::new(session.socket);
        let launch_result = client.launch_go_program(program_path.to_str().unwrap()).await;
        assert!(launch_result.is_err(), "Should fail to launch invalid code");
    }
}
```

---

## Phase 10: Performance and Stress Tests

### Test 10.1: Multiple Breakpoints

**What we're testing**: Can handle many breakpoints.

```rust
#[tokio::test]
async fn test_many_breakpoints() {
    let program = create_program_with_many_lines(1000);
    let client = setup_client(program).await;

    // Set breakpoints at every 10th line
    let breakpoints: Vec<_> = (0..100).map(|i| SourceBreakpoint {
        line: i * 10,
        ..Default::default()
    }).collect();

    let result = client.set_breakpoints("test.go", breakpoints).await;
    assert!(result.is_ok(), "Failed to set many breakpoints");
}
```

---

### Test 10.2: Deep Stack Traces

**What we're testing**: Can handle recursive functions.

```rust
#[tokio::test]
async fn test_deep_recursion_stack_trace() {
    let program = r#"
package main
func recursive(n int) int {
    if n <= 0 {
        return 0
    }
    return n + recursive(n-1)
}
func main() {
    recursive(100)
}
"#;

    let client = setup_and_break_in_function(program, "recursive").await;
    let stack_trace = client.stack_trace().await.unwrap();

    assert!(stack_trace.stack_frames.len() > 10, "Should have deep stack");
}
```

---

## Test Execution Order

Run tests in this order to validate assumptions incrementally:

```bash
# Phase 0: Prerequisites
go version
dlv version

# Phase 1: Basic spawning
cargo test test_spawn_dlv_process
cargo test test_find_free_port_for_dlv
cargo test test_spawn_dlv_on_specific_port

# Phase 2: Connections
cargo test test_connect_to_dlv_with_retry
cargo test test_connection_timeout

# Phase 3: DAP Protocol
cargo test test_send_initialize_request
cargo test test_launch_go_program

# Phase 4: Debugging Operations
cargo test test_set_breakpoint_in_go_code
cargo test test_continue_to_breakpoint
cargo test test_step_operations

# Phase 5: Variable Inspection
cargo test test_inspect_variables
cargo test test_evaluate_expression

# Phase 6: Adapter Integration
cargo test test_go_adapter_trait_implementation
cargo test test_go_adapter_spawn

# Phase 7: Full Integration
cargo test test_fizzbuzz_debugging_go

# Phase 8: Comparison
cargo test test_go_adapter_matches_ruby_pattern

# Phase 9: Error Handling
cargo test test_error_when_dlv_not_found
cargo test test_error_with_invalid_go_syntax

# Phase 10: Performance
cargo test test_many_breakpoints
cargo test test_deep_recursion_stack_trace
```

---

## Success Criteria

✅ **Phase 0**: All manual tests pass
✅ **Phase 1**: Process spawns and stays alive
✅ **Phase 2**: Connection established reliably
✅ **Phase 3**: DAP initialize and launch succeed
✅ **Phase 4**: Breakpoints work, stepping works
✅ **Phase 5**: Variables readable, expressions evaluate
✅ **Phase 6**: Adapter trait fully implemented
✅ **Phase 7**: FizzBuzz test passes end-to-end
✅ **Phase 8**: Pattern matches existing adapters
✅ **Phase 9**: Errors handled gracefully
✅ **Phase 10**: Performance acceptable

---

## Debugging Tips

### If tests fail in Phase 1:
- Check `dlv` is in PATH: `which dlv`
- Check permissions: `ls -l $(which dlv)`
- Try running dlv manually: `dlv version`

### If tests fail in Phase 2:
- Check port isn't already in use: `lsof -i :$PORT`
- Increase retry timeout
- Check firewall settings

### If tests fail in Phase 3:
- Enable DAP logging: `dlv dap --log --log-output=dap`
- Check request/response JSON format
- Verify capabilities match expectations

### If tests fail in Phase 4:
- Check source file paths are absolute
- Verify line numbers are correct (1-indexed)
- Check if Go file compiles: `go build test.go`

### If tests fail in Phase 7:
- Run each step manually to isolate failure
- Check dlv logs for errors
- Verify fixture file is valid Go

---

## CI/CD Integration

**GitHub Actions Workflow**:

```yaml
name: Go Adapter Tests

on: [push, pull_request]

jobs:
  test-go-adapter:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Go
        uses: actions/setup-go@v4
        with:
          go-version: '1.21'

      - name: Install Delve
        run: go install github.com/go-delve/delve/cmd/dlv@latest

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Go Adapter Tests
        run: cargo test --test test_golang_*

      - name: Run FizzBuzz Integration Test
        run: cargo test test_fizzbuzz_debugging_go
```

---

## Documentation Artifacts

After testing, produce:

1. **Test Results Report**: Which tests passed/failed
2. **Performance Benchmarks**: Timing data for each phase
3. **Known Issues**: Any quirks or limitations discovered
4. **Usage Examples**: Sample code for users

---

## Next Phase

After all tests pass, proceed to **Phase 6: Write Recommendation and Implementation Plan** with concrete evidence that Go/Delve adapter is feasible and well-understood.
