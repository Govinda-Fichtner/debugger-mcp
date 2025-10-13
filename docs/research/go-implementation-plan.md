# Go (Delve) Debug Adapter - Implementation Plan

**Research Date**: October 8, 2025
**Based on**: Comprehensive research of Go/Delve and Java debugging ecosystems
**Estimated Time**: 12-16 hours (1.5-2 days)

---

## Executive Recommendation

**Add Go (Delve) support as the next language adapter**

### Justification

1. **Architectural Fit**: Native DAP support, matches existing TCP Socket pattern (Ruby/Node.js)
2. **Implementation Speed**: 80% code reuse = 1.5-2 days vs 5-6 days for Java
3. **Low Risk**: Independent process, no language server dependency
4. **User Experience**: Simple installation (`go install dlv`) vs complex Java setup
5. **Validation**: Proves adapter pattern works for compiled languages

### Key Findings from Research

- **Delve**: Native DAP support via `dlv dap` command
- **Transport**: TCP Socket (same as Ruby/Node.js)
- **Installation**: Single command, no external dependencies
- **Configuration**: Simple, file-path based (like Python/Ruby)
- **Infrastructure**: 100% reuse of existing `socket_helper` module
- **No Workarounds**: Clean implementation, no known issues

---

## Implementation Roadmap

### Day 1: Core Implementation (6-8 hours)

**Morning (4 hours)**
- Hour 1: Setup and exploration
- Hour 2: Implement adapter struct
- Hour 3: Implement spawn function
- Hour 4: Unit tests

**Afternoon (4 hours)**
- Hour 5: DAP protocol integration
- Hour 6: Launch configuration
- Hour 7: Integration tests
- Hour 8: Bug fixes and refinement

### Day 2: Testing and Documentation (4-8 hours)

**Morning (3 hours)**
- Hour 1: FizzBuzz integration test
- Hour 2: Error handling tests
- Hour 3: Manual testing with real Go programs

**Afternoon (3 hours)**
- Hour 4: Documentation
- Hour 5: Code review preparation
- Hour 6: Final validation

**Buffer (2 hours)**
- Unexpected issues
- Additional testing
- Performance optimization

---

## Detailed Implementation Steps

### Step 1: Environment Setup (30 minutes)

**1.1 Install Delve**
```bash
go install github.com/go-delve/delve/cmd/dlv@latest

# Verify installation
dlv version
# Expected: Delve Debugger, Version: X.X.X
```

**1.2 Manual Testing**
```bash
# Create test file
cat > /tmp/hello.go <<EOF
package main
import "fmt"
func main() {
    fmt.Println("Hello, World!")
}
EOF

# Test dlv dap manually
dlv dap --listen=127.0.0.1:12345 &
nc -v 127.0.0.1 12345
# Should connect successfully

# Kill dlv
killall dlv
```

**1.3 Create Branch**
```bash
cd debugger_mcp
git checkout -b feat/add-go-delve-support
git push -u origin feat/add-go-delve-support
```

**Success Criteria**: Delve installed, manual connection works, branch created.

---

### Step 2: Create Adapter Module (1 hour)

**2.1 Create File Structure**
```bash
touch src/adapters/golang.rs
```

**2.2 Update `src/adapters/mod.rs`**
```rust
// Add to existing modules
pub mod golang;
pub mod logging;
pub mod nodejs;
pub mod python;
pub mod ruby;
pub mod rust;
```

**2.3 Implement Basic Adapter**

**File**: `src/adapters/golang.rs`
```rust
//! Go (Delve) debug adapter implementation
//!
//! This adapter provides debugging support for Go programs using Delve's native DAP support.
//!
//! ## Installation
//!
//! Users must have Go installed and Delve debugger:
//! ```bash
//! go install github.com/go-delve/delve/cmd/dlv@latest
//! ```
//!
//! ## Architecture
//!
//! - **Transport**: TCP Socket (same as Ruby/Node.js)
//! - **Command**: `dlv dap --listen=127.0.0.1:<port>`
//! - **Protocol**: DAP (native support, no translation layer)
//! - **Lifecycle**: Single-use server (exits after debug session)

use crate::adapters::logging::DebugAdapterLogger;
use crate::dap::socket_helper;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Go debug adapter using Delve
pub struct GoAdapter {
    port: u16,
}

impl GoAdapter {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl DebugAdapterLogger for GoAdapter {
    fn language_name(&self) -> &str {
        "Go"
    }

    fn language_emoji(&self) -> &str {
        "🐹"
    }

    fn transport_type(&self) -> &str {
        "TCP Socket"
    }

    fn adapter_id(&self) -> &str {
        "delve"
    }

    fn command_line(&self) -> String {
        format!("dlv dap --listen=127.0.0.1:{}", self.port)
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        tracing::error!(
            language = self.language_name(),
            adapter = self.adapter_id(),
            error = %error,
            "Failed to spawn {} debugger",
            self.language_name()
        );
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        tracing::error!(
            language = self.language_name(),
            adapter = self.adapter_id(),
            port = self.port,
            error = %error,
            "Failed to connect to {} debugger",
            self.language_name()
        );
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        tracing::error!(
            language = self.language_name(),
            adapter = self.adapter_id(),
            error = %error,
            "Failed to initialize {} debugger",
            self.language_name()
        );
    }
}

/// Go debug session
pub struct GoDebugSession {
    pub process: Child,
    pub socket: TcpStream,
    pub port: u16,
}

impl Drop for GoDebugSession {
    fn drop(&mut self) {
        let adapter = GoAdapter::new(self.port);
        adapter.log_shutdown();

        // Kill the delve process
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Spawn a Go debug session using Delve
///
/// # Arguments
///
/// * `program` - Path to the Go source file or package directory
/// * `program_args` - Arguments to pass to the Go program
/// * `stop_on_entry` - Whether to stop at program entry point
///
/// # Example
///
/// ```rust,no_run
/// use debugger_mcp::adapters::golang;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let session = golang::spawn(
///         "main.go",
///         &["arg1".to_string(), "arg2".to_string()],
///         true,
///     ).await?;
///
///     // Use session.socket for DAP communication
///     Ok(())
/// }
/// ```
pub async fn spawn(
    program: &str,
    program_args: &[String],
    stop_on_entry: bool,
) -> Result<GoDebugSession> {
    // Find a free port
    let port = socket_helper::find_free_port()?;

    let adapter = GoAdapter::new(port);
    adapter.log_selection();
    adapter.log_transport_init();

    // Build delve command
    let mut args = vec![
        "dap".to_string(),
        "--listen".to_string(),
        format!("127.0.0.1:{}", port),
    ];

    debug!(
        "Spawning {} adapter: dlv {}",
        adapter.language_name(),
        args.join(" ")
    );

    adapter.log_spawn_attempt();

    // Spawn delve process
    let child = Command::new("dlv")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            adapter.log_spawn_error(&e);
            Error::Process(format!("Failed to spawn dlv: {}", e))
        })?;

    info!("Spawned dlv process (PID: {:?})", child.id());

    // Connect to delve with retry (it takes a moment to bind)
    let socket = socket_helper::connect_with_retry(port, Duration::from_secs(3))
        .await
        .map_err(|e| {
            adapter.log_connection_error(&e);
            e
        })?;

    adapter.log_connection_success();

    info!(
        "Connected to {} debugger on port {}",
        adapter.language_name(),
        port
    );

    Ok(GoDebugSession {
        process: child,
        socket,
        port,
    })
}

/// Generate DAP launch arguments for Go debugging
///
/// # Arguments
///
/// * `program` - Path to Go source file or package directory
/// * `args` - Arguments to pass to the Go program
/// * `cwd` - Working directory (optional)
/// * `stop_on_entry` - Whether to stop at program entry
///
/// # Returns
///
/// JSON value suitable for DAP launch request
pub fn launch_args(program: &str, args: &[String], cwd: Option<&str>, stop_on_entry: bool) -> Value {
    let mut launch_config = json!({
        "type": "go",
        "request": "launch",
        "mode": "debug",
        "program": program,
        "args": args,
        "stopOnEntry": stop_on_entry,
    });

    if let Some(cwd) = cwd {
        launch_config["cwd"] = json!(cwd);
    }

    launch_config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_metadata() {
        let adapter = GoAdapter::new(12345);

        assert_eq!(adapter.language_name(), "Go");
        assert_eq!(adapter.language_emoji(), "🐹");
        assert_eq!(adapter.transport_type(), "TCP Socket");
        assert_eq!(adapter.adapter_id(), "delve");
        assert!(!adapter.requires_workaround());
    }

    #[test]
    fn test_command_line() {
        let adapter = GoAdapter::new(12345);
        assert_eq!(
            adapter.command_line(),
            "dlv dap --listen=127.0.0.1:12345"
        );
    }

    #[test]
    fn test_launch_args_minimal() {
        let args = launch_args("main.go", &[], None, false);

        assert_eq!(args["type"], "go");
        assert_eq!(args["request"], "launch");
        assert_eq!(args["mode"], "debug");
        assert_eq!(args["program"], "main.go");
        assert_eq!(args["stopOnEntry"], false);
    }

    #[test]
    fn test_launch_args_full() {
        let program_args = vec!["arg1".to_string(), "arg2".to_string()];
        let args = launch_args("main.go", &program_args, Some("/tmp"), true);

        assert_eq!(args["args"], json!(["arg1", "arg2"]));
        assert_eq!(args["cwd"], "/tmp");
        assert_eq!(args["stopOnEntry"], true);
    }

    #[test]
    fn test_logging_methods() {
        let adapter = GoAdapter::new(12345);

        // These should not panic
        adapter.log_selection();
        adapter.log_transport_init();
        adapter.log_spawn_attempt();
        adapter.log_connection_success();
        adapter.log_shutdown();
    }
}
```

**Success Criteria**: Module compiles, unit tests pass.

```bash
cargo test --lib golang
```

---

### Step 3: Integration Tests (1 hour)

**3.1 Create Test File**

**File**: `tests/test_golang_adapter.rs`
```rust
//! Integration tests for Go (Delve) debug adapter

use debugger_mcp::adapters::golang;
use std::time::Duration;
use tempfile::TempDir;
use tokio;

#[tokio::test]
#[ignore] // Only run if dlv is installed
async fn test_spawn_go_adapter() {
    // Create temporary Go file
    let temp_dir = TempDir::new().unwrap();
    let go_file = temp_dir.path().join("hello.go");
    std::fs::write(
        &go_file,
        r#"
package main
import "fmt"
func main() {
    fmt.Println("Hello from Go!")
}
"#,
    )
    .unwrap();

    // Spawn adapter
    let session = golang::spawn(go_file.to_str().unwrap(), &[], true)
        .await
        .expect("Failed to spawn Go adapter");

    // Verify process is running
    assert!(session.process.id().is_some());
    assert!(session.port > 0);

    // Give it a moment
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cleanup happens via Drop
}

#[tokio::test]
#[ignore]
async fn test_connect_to_delve() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let temp_dir = TempDir::new().unwrap();
    let go_file = temp_dir.path().join("test.go");
    std::fs::write(&go_file, "package main\nfunc main() {}\n").unwrap();

    let mut session = golang::spawn(go_file.to_str().unwrap(), &[], true)
        .await
        .expect("Failed to spawn");

    // Try to write something to the socket
    let test_message = b"test";
    session
        .socket
        .write_all(test_message)
        .await
        .expect("Failed to write to socket");

    // Socket is working
}
```

**3.2 Create FizzBuzz Fixture**

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

**Success Criteria**: Tests pass (if dlv installed).

```bash
cargo test --test test_golang_adapter -- --ignored
```

---

### Step 4: DAP Client Integration (2 hours)

**4.1 Review Existing DAP Client**
```bash
# Understand how Ruby adapter uses DAP client
cat src/adapters/ruby.rs | grep -A 20 "dap::"
```

**4.2 Add Go-Specific DAP Methods** (if needed)

Most DAP operations are language-agnostic, but if we need Go-specific helpers:

**File**: `src/dap/client.rs` (or create `src/dap/go_helpers.rs`)
```rust
// Add to existing DAP client if needed

impl DapClient {
    /// Launch a Go program with Delve-specific configuration
    pub async fn launch_go_program(
        &self,
        program: &str,
        args: &[String],
        stop_on_entry: bool,
    ) -> Result<()> {
        use crate::adapters::golang;

        let launch_args = golang::launch_args(program, args, None, stop_on_entry);
        self.launch(launch_args).await
    }
}
```

**Success Criteria**: DAP client can launch Go programs.

---

### Step 5: End-to-End FizzBuzz Test (2 hours)

**5.1 Create FizzBuzz Integration Test**

**File**: `tests/test_golang_fizzbuzz.rs`
```rust
//! End-to-end FizzBuzz debugging test for Go

use debugger_mcp::adapters::golang;
use debugger_mcp::dap::{DapClient, SourceBreakpoint};
use std::path::PathBuf;

#[tokio::test]
#[ignore] // Requires dlv installation
async fn test_fizzbuzz_go_debugging() {
    // Get fixture path
    let fixture_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "fizzbuzz.go",
    ]
    .iter()
    .collect();

    println!("Testing with fixture: {:?}", fixture_path);

    // 1. Spawn Go debugger
    let session = golang::spawn(fixture_path.to_str().unwrap(), &[], false)
        .await
        .expect("Failed to spawn Go debugger");

    let mut client = DapClient::new(session.socket);

    // 2. Initialize
    println!("Initializing DAP client...");
    client
        .initialize("debugger-mcp-test", "go")
        .await
        .expect("Failed to initialize");

    // 3. Set breakpoint in fizzbuzz function (line 6: first if statement)
    println!("Setting breakpoint at line 6...");
    let breakpoints = vec![SourceBreakpoint {
        line: 6,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
    }];

    let bp_response = client
        .set_breakpoints(fixture_path.to_str().unwrap(), breakpoints)
        .await
        .expect("Failed to set breakpoints");

    assert!(!bp_response.is_empty(), "No breakpoints returned");
    assert!(bp_response[0].verified, "Breakpoint not verified");
    println!("Breakpoint verified at line {}", bp_response[0].line);

    // 4. Launch program
    println!("Launching Go program...");
    client
        .launch_go_program(fixture_path.to_str().unwrap(), &[], false)
        .await
        .expect("Failed to launch program");

    // 5. Continue execution to breakpoint
    println!("Continuing to breakpoint...");
    client
        .continue_execution()
        .await
        .expect("Failed to continue");

    // 6. Wait for stopped event
    let stopped_event = client
        .wait_for_event("stopped", std::time::Duration::from_secs(5))
        .await
        .expect("Didn't receive stopped event");

    assert_eq!(stopped_event["reason"], "breakpoint");
    println!("Stopped at breakpoint!");

    // 7. Get stack trace
    let stack_trace = client.stack_trace().await.expect("Failed to get stack trace");
    assert!(!stack_trace.stack_frames.is_empty(), "No stack frames");
    println!("Stack trace: {} frames", stack_trace.stack_frames.len());

    // 8. Inspect variables
    let frame_id = stack_trace.stack_frames[0].id;
    let scopes = client.scopes(frame_id).await.expect("Failed to get scopes");
    let locals_scope = scopes.iter().find(|s| s.name == "Local" || s.name == "Locals");

    if let Some(scope) = locals_scope {
        let variables = client
            .variables(scope.variables_reference)
            .await
            .expect("Failed to get variables");

        println!("Variables: {:?}", variables);

        // Should see variable 'n'
        let n_var = variables.iter().find(|v| v.name == "n");
        assert!(n_var.is_some(), "Variable 'n' not found");
        println!("Variable n = {}", n_var.unwrap().value);
    }

    // 9. Step over
    println!("Stepping over...");
    client.step_over().await.expect("Failed to step");

    let stepped_event = client
        .wait_for_event("stopped", std::time::Duration::from_secs(5))
        .await
        .expect("Didn't receive stopped event after step");

    assert_eq!(stepped_event["reason"], "step");
    println!("Stepped successfully!");

    // 10. Continue to end
    println!("Continuing to end...");
    client
        .continue_execution()
        .await
        .expect("Failed to continue");

    // 11. Wait for termination
    let _terminated_event = client
        .wait_for_event("terminated", std::time::Duration::from_secs(10))
        .await
        .expect("Program didn't terminate");

    println!("Program terminated successfully!");

    // Cleanup via Drop
}
```

**5.2 Run Test**
```bash
# Ensure dlv is installed
go install github.com/go-delve/delve/cmd/dlv@latest

# Run test
cargo test --test test_golang_fizzbuzz -- --ignored --nocapture
```

**Success Criteria**: FizzBuzz test passes end-to-end.

---

### Step 6: Documentation (1 hour)

**6.1 Update README.md**
```markdown
## Supported Languages

- ✅ **Python** (debugpy) - STDIO transport
- ✅ **Ruby** (rdbg) - TCP Socket transport
- ✅ **Node.js** (node-inspect) - TCP Socket transport
- ✅ **Rust** (CodeLLDB) - STDIO transport, requires compilation
- ✅ **Go** (Delve) - TCP Socket transport  <!-- ADD THIS LINE -->

### Go Debugging

**Installation**:
```bash
go install github.com/go-delve/delve/cmd/dlv@latest
```

**Features**:
- Native DAP support
- Goroutine debugging
- Supports `.go` files and packages
- Breakpoints, stepping, variable inspection

**Example**:
```rust
use debugger_mcp::adapters::golang;

let session = golang::spawn("main.go", &[], true).await?;
// Use session.socket for DAP communication
```
```

**6.2 Create Go-Specific Documentation**

**File**: `docs/adapters/go.md`
```markdown
# Go (Delve) Debug Adapter

## Overview

Supports debugging Go programs using Delve's native DAP implementation.

## Installation

Users must install Delve:
```bash
go install github.com/go-delve/delve/cmd/dlv@latest
```

Verify installation:
```bash
dlv version
```

## Architecture

- **Debugger**: Delve (`dlv`)
- **Transport**: TCP Socket
- **Protocol**: DAP (native)
- **Command**: `dlv dap --listen=127.0.0.1:<port>`

## Usage

### Basic Debugging

```rust
use debugger_mcp::adapters::golang;

let session = golang::spawn("main.go", &[], true).await?;
let mut client = DapClient::new(session.socket);

// Initialize
client.initialize("my-client", "go").await?;

// Set breakpoint
client.set_breakpoints("main.go", vec![
    SourceBreakpoint { line: 10, ..Default::default() }
]).await?;

// Launch
client.launch_go_program("main.go", &[], false).await?;

// Continue
client.continue_execution().await?;
```

## Launch Configuration

```json
{
  "type": "go",
  "request": "launch",
  "mode": "debug",
  "program": "path/to/main.go",
  "args": ["arg1", "arg2"],
  "cwd": "/working/directory",
  "stopOnEntry": false
}
```

### Fields

- `program`: Path to `.go` file or package directory
- `args`: Command-line arguments
- `cwd`: Working directory (optional)
- `stopOnEntry`: Stop at program entry (default: false)
- `mode`: Debug mode (default: "debug")

## Supported Modes

- `debug`: Debug a Go program
- `test`: Debug Go tests
- `exec`: Debug pre-built binary

## Go-Specific Features

### Goroutine Debugging

Delve provides goroutine inspection via DAP's thread support:

```rust
// List all goroutines
let threads = client.threads().await?;

for thread in threads {
    println!("Goroutine {}: {}", thread.id, thread.name);
}

// Get stack trace for specific goroutine
let stack_trace = client.stack_trace_for_thread(thread_id).await?;
```

### Configuration Options

Launch args support additional options:

```json
{
  "hideSystemGoroutines": true,  // Hide system goroutines from thread list
  "showGlobalVariables": true,   // Show global variables in scope
  "substitutePath": [            // Path mapping for remote debugging
    {
      "from": "/src",
      "to": "/local/src"
    }
  ]
}
```

## Known Limitations

1. **Single-use Server**: Delve exits after debug session ends (by design)
2. **Go Installation Required**: Users must have Go toolchain installed
3. **Compilation**: Source files must be compilable

## Troubleshooting

### dlv: command not found

Ensure Delve is installed and in PATH:
```bash
go install github.com/go-delve/delve/cmd/dlv@latest
export PATH="$PATH:$(go env GOPATH)/bin"
```

### Connection Timeout

Increase retry timeout if spawning is slow:
```rust
let socket = socket_helper::connect_with_retry(port, Duration::from_secs(5)).await?;
```

### Breakpoint Not Verified

Ensure:
- Go file is syntactically correct
- Line number is valid (1-indexed)
- File path is absolute

### Goroutines Not Showing

Enable in launch config:
```json
{
  "hideSystemGoroutines": false
}
```

## Comparison to Other Adapters

| Feature | Go (Delve) | Python (debugpy) | Ruby (rdbg) |
|---------|------------|------------------|-------------|
| Transport | TCP Socket | STDIO | TCP Socket |
| DAP Support | Native | Native | Native |
| Installation | `go install` | `pip install` | `gem install` |
| Compilation | No | No | No |
| Concurrency | Goroutines | Threads | Threads |

## References

- [Delve Documentation](https://github.com/go-delve/delve)
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [VS Code Go Debugging](https://code.visualstudio.com/docs/languages/go#_debugging)
```

**Success Criteria**: Documentation complete and accurate.

---

### Step 7: Code Review and Cleanup (1 hour)

**7.1 Run All Tests**
```bash
# Unit tests
cargo test --lib golang

# Integration tests
cargo test --test test_golang_adapter -- --ignored

# FizzBuzz test
cargo test --test test_golang_fizzbuzz -- --ignored --nocapture

# All tests
cargo test
```

**7.2 Run Linters**
```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check documentation
cargo doc --no-deps --document-private-items
```

**7.3 Manual Testing**
```bash
# Test with real Go program
cat > /tmp/test.go <<EOF
package main
import "fmt"
func main() {
    for i := 0; i < 5; i++ {
        fmt.Println("Count:", i)
    }
}
EOF

# Run debugger manually to verify
cargo run -- debug /tmp/test.go
```

**Success Criteria**: All tests pass, no lint warnings, manual testing works.

---

### Step 8: Git Commit and PR (30 minutes)

**8.1 Review Changes**
```bash
git status
git diff
```

**8.2 Commit**
```bash
git add src/adapters/golang.rs
git add src/adapters/mod.rs
git add tests/test_golang_adapter.rs
git add tests/test_golang_fizzbuzz.rs
git add tests/fixtures/fizzbuzz.go
git add docs/adapters/go.md
git add README.md

git commit -m "feat(adapters): add Go (Delve) debug adapter support

Add native DAP support for Go debugging using Delve debugger.

Implementation details:
- TCP Socket transport (same pattern as Ruby/Node.js)
- Native DAP support via 'dlv dap' command
- 100% reuse of socket_helper module
- Simple installation: go install dlv

Features:
- Breakpoints, stepping, variable inspection
- Goroutine debugging support
- FizzBuzz integration test validates all features
- Clean adapter pattern implementation

Testing:
- Unit tests for adapter trait implementation
- Integration tests for spawn and connection
- End-to-end FizzBuzz debugging scenario
- All tests pass with dlv installed

Documentation:
- README updated with Go support
- New docs/adapters/go.md with usage guide
- Inline code documentation

Refs #<issue-number>"
```

**8.3 Push and Create PR**
```bash
git push origin feat/add-go-delve-support

# Create PR via GitHub CLI or web interface
gh pr create \
  --title "feat: Add Go (Delve) debug adapter support" \
  --body "$(cat <<EOF
## Summary

Adds Go debugging support using Delve's native DAP implementation.

## Changes

- ✅ New \`src/adapters/golang.rs\` module
- ✅ TCP Socket transport (matches Ruby/Node.js pattern)
- ✅ Complete test coverage
- ✅ FizzBuzz integration test
- ✅ Documentation

## Testing

All tests pass with Delve installed:
\`\`\`bash
cargo test --test test_golang_* -- --ignored
\`\`\`

## Installation

Users need Go and Delve:
\`\`\`bash
go install github.com/go-delve/delve/cmd/dlv@latest
\`\`\`

## Reviewer Notes

- 80% code reuse from Ruby adapter
- Clean implementation, no workarounds needed
- Validates adapter pattern for compiled languages

## Next Steps

After this lands, can add Java support with lessons learned.
EOF
)"
```

**Success Criteria**: PR created with clear description.

---

## Testing Checklist

Before marking complete, verify:

- [ ] Unit tests pass: `cargo test --lib golang`
- [ ] Integration tests pass: `cargo test --test test_golang_adapter -- --ignored`
- [ ] FizzBuzz test passes: `cargo test --test test_golang_fizzbuzz -- --ignored`
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] Manual testing with real Go program works
- [ ] README updated
- [ ] New documentation created
- [ ] Git commit follows conventions
- [ ] PR created with good description

---

## Risk Mitigation

### Risk: Delve not installed on user machine

**Mitigation**:
- Clear error message: "dlv not found. Install with: go install github.com/go-delve/delve/cmd/dlv@latest"
- Documentation explains installation
- CI checks verify dlv is available

### Risk: Port already in use

**Mitigation**:
- Use `find_free_port()` to get OS-assigned port
- Retry logic in `connect_with_retry()`
- Clear error messages

### Risk: Go compilation errors

**Mitigation**:
- Launch request will fail gracefully
- Error message includes Go compiler output
- User can fix syntax errors and retry

### Risk: Performance issues with large programs

**Mitigation**:
- Delve handles this natively
- DAP protocol supports pagination
- Can add caching if needed

---

## Post-Implementation Tasks

After Go support lands:

1. **Announce to users** via changelog/blog post
2. **Update examples** with Go programs
3. **Monitor issues** for edge cases
4. **Gather feedback** on UX
5. **Consider improvements**:
   - Remote debugging support
   - Advanced goroutine filtering
   - Performance profiling integration

---

## Success Metrics

**Definition of Done**:
- ✅ Code merged to main branch
- ✅ All tests passing in CI
- ✅ Documentation complete
- ✅ At least one user successfully debugs Go program
- ✅ No critical bugs reported in first week

**Performance Targets**:
- Spawn time: < 500ms
- Connection time: < 200ms
- Breakpoint set: < 50ms
- Step operation: < 100ms

---

## Future Enhancements (Not in Scope)

Ideas for later:
- **Remote Debugging**: Connect to dlv on remote host
- **Core Dump Analysis**: Debug from core files
- **Replay Debugging**: Time-travel debugging
- **Test Debugging**: Specialized support for `go test`
- **Module Awareness**: Smart breakpoint resolution in modules

---

## Java Implementation (Future Work)

After Go is stable, apply lessons learned to Java:

### Key Differences to Handle

1. **LSP Dependency**: Must manage jdt.ls lifecycle
2. **Complex Setup**: Multi-step initialization
3. **Classpath Resolution**: Requires project metadata
4. **Longer Timeline**: 5-6 days vs 1-2 days

### Suggested Approach

1. Research jdt.ls integration patterns
2. Build LSP client infrastructure
3. Create JdtLsManager for lifecycle
4. Implement Java adapter on top
5. Extensive testing due to complexity

---

## Conclusion

This plan provides a clear, incremental path to adding Go support with minimal risk and maximum code reuse.

**Key Success Factors**:
- Follow existing patterns (Ruby TCP Socket adapter)
- Comprehensive testing at each step
- Clear documentation for users
- Gradual validation (not big bang)

**Timeline**: 12-16 hours (1.5-2 days) for complete implementation.

**Next Language**: After Go is stable and proven, tackle Java with confidence.

---

**Questions? Contact**: [maintainer]
**Implementation Branch**: `feat/add-go-delve-support`
**Related Issues**: #[issue-number]
