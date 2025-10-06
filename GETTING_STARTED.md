# Getting Started - DAP MCP Server Implementation

## Quick Start

### 1. Initialize Project

```bash
# Create new Rust library
cargo new --lib debugger_mcp
cd debugger_mcp

# Add dependencies
cargo add clap --features derive
cargo add tokio --features full
cargo add serde --features derive
cargo add serde_json
cargo add anyhow
cargo add thiserror
cargo add tracing
cargo add tracing-subscriber --features env-filter,json
cargo add flume
cargo add async-trait
cargo add uuid --features v4,serde

# Dev dependencies
cargo add --dev tokio-test
cargo add --dev tempfile
cargo add --dev assert_matches
```

### 2. Create Initial Structure

```bash
# Source structure
mkdir -p src/{mcp,debug,dap,adapters,process}
mkdir -p src/mcp/{resources,tools}
mkdir -p tests/{integration,fixtures}

# Create module files
touch src/mcp/mod.rs src/mcp/transport.rs src/mcp/protocol.rs
touch src/debug/mod.rs src/debug/session.rs src/debug/state.rs
touch src/dap/mod.rs src/dap/client.rs src/dap/transport.rs
touch src/adapters/mod.rs src/adapters/python.rs
touch src/process/mod.rs
touch src/error.rs

# Test files
touch tests/integration/mod.rs
touch tests/integration/fizzbuzz_test.rs
touch tests/fixtures/fizzbuzz.py
```

### 3. Write First Test

```rust
// tests/integration/fizzbuzz_test.rs

use debugger_mcp::*;

#[tokio::test]
async fn test_server_starts() {
    // This test will fail initially - that's TDD!
    let server = McpServer::new().await.unwrap();
    assert!(server.is_running());
}
```

### 4. Run Tests (Expect Failure)

```bash
cargo test
# ❌ Compilation error - McpServer doesn't exist yet
```

### 5. Make It Compile

```rust
// src/lib.rs
pub mod mcp;
pub mod debug;
pub mod dap;
pub mod adapters;
pub mod process;
pub mod error;

pub use mcp::McpServer;

// src/mcp/mod.rs
pub struct McpServer;

impl McpServer {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn is_running(&self) -> bool {
        true
    }
}
```

### 6. Run Tests Again

```bash
cargo test
# ✅ Test passes!
```

**You're now in the TDD cycle! Continue building feature by feature.**

---

## CLI Development

### Create CLI Binary

```rust
// src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "debugger_mcp")]
#[command(about = "DAP-based MCP debugging server")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server (STDIO transport)
    Serve {
        #[arg(short, long)]
        verbose: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { verbose } => {
            if verbose {
                println!("Starting MCP server in verbose mode...");
            }
            debugger_mcp::serve().await?;
        }
    }

    Ok(())
}

// src/lib.rs
pub async fn serve() -> anyhow::Result<()> {
    println!("MCP server started. Listening on STDIO...");
    // TODO: Implement actual server loop
    Ok(())
}
```

### Test CLI

```bash
cargo run -- serve
# MCP server started. Listening on STDIO...

cargo run -- serve --verbose
# Starting MCP server in verbose mode...
# MCP server started. Listening on STDIO...

cargo run -- --help
# DAP-based MCP debugging server
#
# Usage: debugger_mcp <COMMAND>
#
# Commands:
#   serve  Start the MCP server (STDIO transport)
#   help   Print this message or the help of the given subcommand(s)
```

---

## TDD Workflow Example

### Feature: Set Python Breakpoint

#### Step 1: Write Integration Test

```rust
// tests/integration/fizzbuzz_test.rs

#[tokio::test]
async fn test_set_breakpoint_python() {
    // Start server
    let mut client = TestMcpClient::new().await;

    // Start debugger
    let session_id = client.call_tool("debugger_start", json!({
        "mode": "launch",
        "language": "python",
        "program": "tests/fixtures/fizzbuzz.py"
    })).await.unwrap();

    // Set breakpoint
    let response = client.call_tool("debugger_set_breakpoint", json!({
        "sessionId": session_id,
        "source": "tests/fixtures/fizzbuzz.py",
        "line": 3
    })).await.unwrap();

    assert_eq!(response["verified"], true);
    assert_eq!(response["line"], 3);
}
```

**Run**: `cargo test test_set_breakpoint_python`
**Result**: ❌ Fails (not implemented)

#### Step 2: Write Unit Test for Tool

```rust
// src/mcp/tools/breakpoint.rs

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_set_breakpoint_tool() {
        let manager = SessionManager::new_test();
        let tool = SetBreakpointTool::new(manager);

        let params = json!({
            "sessionId": "test-session",
            "source": "/path/to/file.py",
            "line": 10
        });

        let result = tool.handle(params).await.unwrap();
        assert_eq!(result["verified"], true);
    }
}
```

**Run**: `cargo test test_set_breakpoint_tool`
**Result**: ❌ Fails (not implemented)

#### Step 3: Implement Tool

```rust
// src/mcp/tools/breakpoint.rs

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::Error;

pub struct SetBreakpointTool {
    session_manager: Arc<SessionManager>,
}

#[async_trait]
impl ToolHandler for SetBreakpointTool {
    async fn handle(&self, params: Value) -> Result<Value, Error> {
        let session_id = params["sessionId"]
            .as_str()
            .ok_or(Error::MissingParam("sessionId"))?;

        let source = params["source"]
            .as_str()
            .ok_or(Error::MissingParam("source"))?;

        let line = params["line"]
            .as_u64()
            .ok_or(Error::MissingParam("line"))? as i64;

        let session = self.session_manager
            .get_session(session_id)
            .await?;

        let bp = session.write().await
            .set_breakpoint(source, line)
            .await?;

        Ok(json!({
            "id": bp.id,
            "verified": bp.verified,
            "line": bp.line
        }))
    }
}
```

**Run**: `cargo test test_set_breakpoint_tool`
**Result**: ✅ Passes (if underlying components implemented)

#### Step 4: Implement Session.set_breakpoint

```rust
// src/debug/session.rs

impl DebugSession {
    pub async fn set_breakpoint(
        &mut self,
        source: &str,
        line: i64
    ) -> Result<Breakpoint, Error> {
        // Use DAP client to set breakpoint
        let bp_response = self.dap_client
            .set_breakpoints(source, &[SourceBreakpoint {
                line,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }])
            .await?;

        let bp = bp_response.breakpoints[0].clone();

        // Store in session state
        self.breakpoints.push(bp.clone());

        Ok(bp)
    }
}
```

#### Step 5: Run All Tests

```bash
cargo test
# ✅ All tests pass!
```

---

## Test Fixtures

### Python FizzBuzz (tests/fixtures/fizzbuzz.py)

```python
def fizzbuzz(n):
    if n % 15 == 0:
        return "FizzBuzz"
    elif n % 3 == 0:
        return "Fizz"
    elif n % 5 == 0:
        return "Buzz"
    else:
        return str(n)

def main():
    results = []
    for i in range(1, 16):
        result = fizzbuzz(i)
        results.append(result)
    print("Results:", results)

if __name__ == "__main__":
    main()
```

### Test MCP Client Helper

```rust
// tests/integration/helpers.rs

use serde_json::Value;
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct TestMcpClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    seq: u64,
}

impl TestMcpClient {
    pub async fn new() -> Self {
        let mut process = Command::new("cargo")
            .args(&["run", "--", "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

        Self {
            process,
            stdin,
            stdout,
            seq: 1,
        }
    }

    pub async fn call_tool(
        &mut self,
        tool: &str,
        params: Value
    ) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.seq,
            "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": params
            }
        });

        self.seq += 1;

        // Send request
        let request_str = serde_json::to_string(&request)?;
        self.stdin.write_all(request_str.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;

        let response: Value = serde_json::from_str(&line)?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("MCP error: {}", error);
        }

        Ok(response["result"].clone())
    }
}

impl Drop for TestMcpClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
```

---

## Development Checklist

### Phase 1: Foundation (Days 1-2)

- [ ] `cargo new --lib debugger_mcp`
- [ ] Add all dependencies
- [ ] Create directory structure
- [ ] Implement CLI skeleton with clap
- [ ] Write first test (server starts)
- [ ] Make test pass
- [ ] Set up logging

### Phase 2: MCP Protocol (Days 3-5)

- [ ] Implement STDIO transport (read/write JSON-RPC)
- [ ] Write tests for transport
- [ ] Implement MCP initialize request
- [ ] Implement tool routing
- [ ] Write integration test with TestMcpClient
- [ ] Make integration test pass

### Phase 3: DAP Client (Days 6-10)

- [ ] Research debugpy protocol
- [ ] Implement DAP wire protocol
- [ ] Write tests for wire protocol
- [ ] Implement process spawning
- [ ] Test spawning debugpy manually
- [ ] Implement DAP initialize sequence
- [ ] Write integration test for DAP client

### Phase 4: Session Management (Days 11-14)

- [ ] Implement SessionManager
- [ ] Implement DebugSession state machine
- [ ] Write state machine tests
- [ ] Connect MCP tools to sessions
- [ ] Write tool unit tests

### Phase 5: Core Tools (Days 15-18)

- [ ] Implement `debugger_start`
- [ ] Implement `debugger_set_breakpoint`
- [ ] Implement `debugger_continue`
- [ ] Implement `debugger_evaluate`
- [ ] Write unit tests for each tool
- [ ] Write integration tests

### Phase 6: FizzBuzz Test (Days 19-21)

- [ ] Create fizzbuzz.py fixture
- [ ] Write complete FizzBuzz integration test
- [ ] Run test, fix bugs
- [ ] Verify with manual Claude Desktop testing
- [ ] Document findings

---

## Debugging Tips

### Enable Verbose Logging

```bash
RUST_LOG=debug cargo run -- serve --verbose
```

### Test Individual Components

```bash
# Test only MCP transport
cargo test --lib mcp::transport

# Test only DAP client
cargo test --lib dap::client

# Test only integration
cargo test --test integration
```

### Manual STDIO Testing

```bash
# Start server
cargo run -- serve

# In another terminal, send JSON-RPC:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize"}' | cargo run -- serve
```

### Debug Python Debugger Issues

```bash
# Test debugpy standalone
python -m debugpy --listen 5678 --wait-for-client tests/fixtures/fizzbuzz.py

# In another terminal:
nc localhost 5678
# Send DAP initialize request manually
```

---

## Common Issues & Solutions

### Issue: "debugpy not found"

**Solution**:
```bash
pip install debugpy
```

### Issue: "Process spawning fails"

**Solution**: Check that Python is in PATH
```bash
which python
python --version
```

### Issue: "Tests hang"

**Solution**: Add timeouts
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    tokio::time::timeout(
        Duration::from_secs(5),
        actual_test()
    ).await.unwrap();
}
```

### Issue: "DAP protocol errors"

**Solution**: Enable DAP logging
```bash
cargo run -- serve --log-dap
```

---

## Resources

### Documentation
- Main proposal: `docs/DAP_MCP_SERVER_PROPOSAL.md`
- Components: `docs/architecture/COMPONENTS.md`
- MVP plan: `docs/MVP_IMPLEMENTATION_PLAN.md`

### External References
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy Documentation](https://github.com/microsoft/debugpy)
- [Clap Documentation](https://docs.rs/clap/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Example Code
- [nvim-dap](https://github.com/mfussenegger/nvim-dap) - Neovim DAP client
- [vscode-debugadapter-node](https://github.com/microsoft/vscode-debugadapter-node) - Official DAP SDK

---

## Next Steps

1. **Set up project**: Follow "Quick Start" section above
2. **Read MVP plan**: `docs/MVP_IMPLEMENTATION_PLAN.md`
3. **Start coding**: Begin with first test (server starts)
4. **Follow TDD**: Red → Green → Refactor
5. **Run FizzBuzz test**: Week 3 goal

**Happy coding!** 🦀🚀
