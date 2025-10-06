# MVP Implementation Plan - Phase 1

## Overview

**Goal**: Build a minimal viable DAP MCP server with Python support, then validate with Ruby.

**Approach**: Test-Driven Development (TDD) with a concrete integration scenario.

---

## CLI Design with Clap

### Why Clap?

✅ **Yes, clap is an excellent choice** for the MCP server CLI:

1. **Industry Standard**: Most popular Rust CLI framework (derive macros, auto-help)
2. **Subcommands**: Perfect for `debugger_mcp serve` pattern
3. **Type Safety**: Compile-time validation of arguments
4. **Auto-generated Help**: Professional CLI experience
5. **Future Extensibility**: Easy to add more commands (e.g., `list-adapters`, `test`)

### CLI Structure

```rust
// src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "debugger_mcp")]
#[command(about = "DAP-based MCP debugging server", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server (STDIO transport)
    Serve {
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Log level (error, warn, info, debug, trace)
        #[arg(long, default_value = "info")]
        log_level: String,

        /// Enable DAP protocol logging
        #[arg(long)]
        log_dap: bool,
    },

    /// List available debug adapters
    ListAdapters,

    /// Validate adapter configuration
    TestAdapter {
        /// Language to test (python, ruby, etc.)
        language: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { verbose, log_level, log_dap } => {
            setup_logging(&log_level, verbose, log_dap)?;
            serve_mcp().await?;
        }
        Commands::ListAdapters => {
            list_adapters();
        }
        Commands::TestAdapter { language } => {
            test_adapter(&language).await?;
        }
    }

    Ok(())
}
```

### Usage Examples

```bash
# Start MCP server (STDIO)
debugger_mcp serve

# Start with verbose logging
debugger_mcp serve --verbose

# Start with debug-level DAP protocol logging
debugger_mcp serve --log-level debug --log-dap

# List supported languages
debugger_mcp list-adapters

# Test Python adapter installation
debugger_mcp test-adapter python
```

---

## Phased Development Plan

### Phase 1A: Python Support (Weeks 1-3)

**Objective**: Working Python debugger with core features

#### Week 1: Foundation
**Days 1-2: Project Setup**
- [x] Create Rust project structure
- [x] Add dependencies (Cargo.toml)
- [x] Set up clap CLI skeleton
- [x] Configure logging (tracing)
- [x] Write first test (empty server responds to ping)

**Days 3-5: MCP Protocol Layer**
- [ ] Implement STDIO transport (read/write JSON-RPC)
- [ ] Implement MCP protocol handler (routing)
- [ ] Add `initialize` request/response
- [ ] Add error handling
- [ ] **Tests**: STDIO read/write, protocol parsing, error responses

**Day 6-7: Integration Test Setup**
- [ ] Create test fixtures (sample Python scripts)
- [ ] Write integration test harness
- [ ] Test MCP server startup/shutdown

#### Week 2: DAP Client + Python Adapter
**Days 1-3: DAP Transport**
- [ ] Implement DAP wire protocol (Content-Length headers + JSON)
- [ ] Implement request/response correlation (sequence numbers)
- [ ] Implement event stream processing
- [ ] **Tests**: Wire protocol encoding/decoding, request/response matching

**Days 4-5: Python Adapter**
- [ ] Create `AdapterConfig` for debugpy
- [ ] Implement process spawning (tokio::process)
- [ ] Connect DAP transport to debugpy process
- [ ] **Tests**: Spawn debugpy, send initialize, receive response

**Days 6-7: Session Management**
- [ ] Implement `SessionManager`
- [ ] Implement `DebugSession` state machine
- [ ] Connect MCP tools to DAP client
- [ ] **Tests**: Session lifecycle, state transitions

#### Week 3: Core Features + Integration
**Days 1-3: MCP Tools Implementation**
- [ ] `debugger_start` (launch Python script)
- [ ] `debugger_set_breakpoint` (source breakpoints)
- [ ] `debugger_continue` (resume execution)
- [ ] `debugger_evaluate` (evaluate expressions)
- [ ] **Tests**: Each tool independently

**Days 4-5: MCP Resources**
- [ ] `debugger://sessions` (list sessions)
- [ ] `debugger://sessions/{id}` (session state)
- [ ] `debugger://sessions/{id}/stackTrace` (call stack)
- [ ] **Tests**: Resource handlers, URI parsing

**Days 6-7: Integration Scenario**
- [ ] Run full integration test (see below)
- [ ] Fix bugs discovered
- [ ] Performance testing
- [ ] Documentation

### Phase 1B: Ruby Validation (Week 4)

**Objective**: Prove architecture is truly language-agnostic

**Days 1-2: Ruby Adapter**
- [ ] Research Ruby debugger (rdbg, ruby-debug-ide)
- [ ] Create `AdapterConfig` for Ruby
- [ ] Implement launch template for Ruby
- [ ] **Tests**: Spawn Ruby debugger, initialize

**Days 3-4: Integration Testing**
- [ ] Create Ruby test fixtures
- [ ] Run same integration scenario with Ruby
- [ ] Document differences/quirks
- [ ] **Tests**: Full Ruby debugging workflow

**Days 5: Validation & Documentation**
- [ ] Compare Python vs Ruby implementations
- [ ] Document abstraction layer effectiveness
- [ ] Identify refactoring opportunities
- [ ] Update architecture docs with learnings

---

## Integration Test Scenario

### The "FizzBuzz Debugger" Test

**Why FizzBuzz?**
- Simple algorithm everyone understands
- Has loops (test stepping)
- Has conditionals (test breakpoints)
- Has function calls (test stack traces)
- Has variables (test inspection)
- Has return values (test evaluation)

### Test Script (Python)

```python
# tests/fixtures/fizzbuzz.py
def fizzbuzz(n):
    """Returns FizzBuzz result for number n"""
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

    print("FizzBuzz results:", results)
    return results

if __name__ == "__main__":
    main()
```

### Test Script (Ruby)

```ruby
# tests/fixtures/fizzbuzz.rb
def fizzbuzz(n)
  if n % 15 == 0
    "FizzBuzz"
  elsif n % 3 == 0
    "Fizz"
  elsif n % 5 == 0
    "Buzz"
  else
    n.to_s
  end
end

def main
  results = []
  (1..15).each do |i|
    result = fizzbuzz(i)
    results << result
  end

  puts "FizzBuzz results: #{results}"
  results
end

main if __FILE__ == $PROGRAM_NAME
```

### Integration Test Specification

```rust
// tests/integration/fizzbuzz_test.rs

#[tokio::test]
async fn test_fizzbuzz_debugging_python() {
    // 1. Start debugger
    let session_id = start_debugger(
        "python",
        "tests/fixtures/fizzbuzz.py"
    ).await.unwrap();

    // 2. Set breakpoint at line 3 (inside fizzbuzz function)
    let bp = set_breakpoint(
        &session_id,
        "tests/fixtures/fizzbuzz.py",
        3
    ).await.unwrap();
    assert!(bp.verified, "Breakpoint should be verified");

    // 3. Continue execution
    continue_execution(&session_id).await.unwrap();

    // 4. Wait for breakpoint hit
    wait_for_stopped(&session_id, "breakpoint").await.unwrap();

    // 5. Get stack trace
    let stack = get_stack_trace(&session_id).await.unwrap();
    assert_eq!(stack.frames.len(), 2); // fizzbuzz + main
    assert_eq!(stack.frames[0].name, "fizzbuzz");

    // 6. Evaluate variable 'n'
    let n_value = evaluate(&session_id, "n").await.unwrap();
    assert_eq!(n_value.result, "1"); // First iteration

    // 7. Step over to line 4
    step_over(&session_id).await.unwrap();

    // 8. Evaluate the condition
    let cond = evaluate(&session_id, "n % 15 == 0").await.unwrap();
    assert_eq!(cond.result, "False");

    // 9. Continue to next breakpoint hit
    continue_execution(&session_id).await.unwrap();
    wait_for_stopped(&session_id, "breakpoint").await.unwrap();

    // 10. Check we're at n=2 now
    let n_value = evaluate(&session_id, "n").await.unwrap();
    assert_eq!(n_value.result, "2");

    // 11. Remove breakpoint and continue to completion
    remove_breakpoint(&session_id, bp.id).await.unwrap();
    continue_execution(&session_id).await.unwrap();

    // 12. Wait for program completion
    wait_for_terminated(&session_id).await.unwrap();

    // 13. Clean up
    stop_debugger(&session_id).await.unwrap();
}

#[tokio::test]
async fn test_fizzbuzz_debugging_ruby() {
    // Same test, but with Ruby script
    // Validates language abstraction works
    // ... (identical structure to Python test)
}
```

### Success Criteria

The integration test passes when:

✅ **Debugger starts** successfully for both Python and Ruby
✅ **Breakpoints set** and are verified
✅ **Execution pauses** at breakpoints
✅ **Stack traces** show correct call hierarchy
✅ **Variable inspection** returns correct values
✅ **Expression evaluation** works in language context
✅ **Stepping** advances execution correctly
✅ **Program completes** and terminates cleanly
✅ **No resource leaks** (processes cleaned up)

---

## TDD Workflow

### Red-Green-Refactor Cycle

**Example: Implementing `debugger_start` tool**

#### 1. Red Phase (Write Failing Test)

```rust
// src/mcp/tools/start.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_python_debugger() {
        let manager = SessionManager::new_test();
        let tool = StartDebuggerTool::new(manager.clone());

        let params = json!({
            "mode": "launch",
            "language": "python",
            "program": "tests/fixtures/simple.py"
        });

        let result = tool.handle(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response["sessionId"].is_string());
        assert_eq!(response["state"], "running");
    }
}
```

**Run**: `cargo test test_start_python_debugger`
**Expected**: ❌ Test fails (not implemented yet)

#### 2. Green Phase (Minimal Implementation)

```rust
// src/mcp/tools/start.rs

pub struct StartDebuggerTool {
    session_manager: Arc<SessionManager>,
}

impl StartDebuggerTool {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }
}

#[async_trait]
impl ToolHandler for StartDebuggerTool {
    async fn handle(&self, params: Value) -> Result<Value, Error> {
        let mode = params["mode"].as_str()
            .ok_or_else(|| Error::InvalidParams("mode required"))?;
        let language = params["language"].as_str()
            .ok_or_else(|| Error::InvalidParams("language required"))?;
        let program = params["program"].as_str()
            .ok_or_else(|| Error::InvalidParams("program required"))?;

        let config = LaunchConfig {
            mode: mode.to_string(),
            program: program.to_string(),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
        };

        let session_id = self.session_manager
            .create_session(language, config)
            .await?;

        Ok(json!({
            "sessionId": session_id,
            "state": "running"
        }))
    }
}
```

**Run**: `cargo test test_start_python_debugger`
**Expected**: ✅ Test passes

#### 3. Refactor Phase (Improve Code Quality)

```rust
// Extract parameter parsing
#[derive(Debug, Deserialize)]
struct StartDebuggerParams {
    mode: String,
    language: String,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[async_trait]
impl ToolHandler for StartDebuggerTool {
    async fn handle(&self, params: Value) -> Result<Value, Error> {
        let params: StartDebuggerParams = serde_json::from_value(params)
            .map_err(|e| Error::InvalidParams(e.to_string()))?;

        let config = LaunchConfig::from(params);

        let session_id = self.session_manager
            .create_session(&params.language, config)
            .await?;

        Ok(json!({
            "sessionId": session_id,
            "state": "running"
        }))
    }
}
```

**Run**: `cargo test test_start_python_debugger`
**Expected**: ✅ Test still passes (refactoring didn't break anything)

#### 4. Add More Tests (Edge Cases)

```rust
#[tokio::test]
async fn test_start_debugger_missing_language() {
    let tool = StartDebuggerTool::new_test();
    let params = json!({
        "mode": "launch",
        "program": "test.py"
    });

    let result = tool.handle(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_start_debugger_unsupported_language() {
    let tool = StartDebuggerTool::new_test();
    let params = json!({
        "mode": "launch",
        "language": "cobol",
        "program": "test.cob"
    });

    let result = tool.handle(params).await;
    assert!(matches!(result, Err(Error::AdapterNotFound(_))));
}
```

### TDD Development Order

1. **Start with integration test** (failing) - defines what success looks like
2. **Work backwards** to implement components:
   - MCP tool handler (failing unit test)
   - Session manager (failing unit test)
   - DAP client (failing unit test)
   - DAP transport (failing unit test)
3. **Make each test pass** with minimal code
4. **Refactor** for clarity and maintainability
5. **Run integration test** - should pass when all units work together

---

## Updated Cargo.toml

```toml
[package]
name = "debugger_mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive"] }

# Async Runtime
tokio = { version = "1", features = ["full"] }

# MCP Protocol
# (Using official SDK when available, or implement JSON-RPC manually)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# DAP Protocol
# (May need to implement types manually initially)

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Async Channels
flume = "0.11"

# Process Management
async-trait = "0.1"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
assert_matches = "1.5"
```

---

## Project Structure

```
debugger_mcp/
├── Cargo.toml
├── README.md
├── docs/                          # Architecture docs (from earlier)
├── src/
│   ├── main.rs                   # CLI with clap
│   ├── lib.rs                    # Library root
│   ├── mcp/
│   │   ├── mod.rs               # MCP server
│   │   ├── transport.rs         # STDIO transport
│   │   ├── protocol.rs          # JSON-RPC handling
│   │   ├── resources/
│   │   │   ├── mod.rs
│   │   │   ├── sessions.rs      # Session resources
│   │   │   └── breakpoints.rs   # Breakpoint resources
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── start.rs         # debugger_start
│   │       ├── stop.rs          # debugger_stop
│   │       ├── breakpoint.rs    # debugger_set_breakpoint
│   │       ├── control.rs       # continue, pause, step
│   │       └── inspect.rs       # evaluate, get_variables
│   ├── debug/
│   │   ├── mod.rs               # SessionManager
│   │   ├── session.rs           # DebugSession
│   │   └── state.rs             # SessionState enum
│   ├── dap/
│   │   ├── mod.rs               # DAP client
│   │   ├── client.rs            # DapClient
│   │   ├── transport.rs         # Wire protocol
│   │   ├── types.rs             # DAP message types
│   │   └── events.rs            # Event processing
│   ├── adapters/
│   │   ├── mod.rs               # AdapterRegistry
│   │   ├── config.rs            # AdapterConfig
│   │   ├── python.rs            # Python/debugpy
│   │   └── ruby.rs              # Ruby/rdbg
│   ├── process/
│   │   ├── mod.rs
│   │   └── manager.rs           # Process spawning
│   └── error.rs                 # Error types
├── tests/
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── fizzbuzz_test.rs    # Main integration test
│   │   └── helpers.rs           # Test utilities
│   └── fixtures/
│       ├── fizzbuzz.py          # Python test script
│       ├── fizzbuzz.rb          # Ruby test script
│       └── simple.py            # Minimal test script
└── .github/
    └── workflows/
        └── ci.yml               # GitHub Actions CI
```

---

## Development Checklist

### Week 1: Foundation
- [ ] Initialize Rust project (`cargo init --lib`)
- [ ] Add clap dependency
- [ ] Implement CLI skeleton with `serve` subcommand
- [ ] Add logging with tracing
- [ ] Write first integration test (server starts)
- [ ] Implement STDIO transport
- [ ] Implement basic MCP protocol handler
- [ ] Test with manual JSON-RPC messages

### Week 2: DAP Integration
- [ ] Implement DAP wire protocol
- [ ] Implement process spawning
- [ ] Test spawning debugpy manually
- [ ] Implement request/response correlation
- [ ] Implement event processing
- [ ] Create Python adapter configuration
- [ ] Test DAP initialize sequence

### Week 3: MCP Tools
- [ ] Implement `debugger_start` with tests
- [ ] Implement `debugger_set_breakpoint` with tests
- [ ] Implement `debugger_continue` with tests
- [ ] Implement `debugger_evaluate` with tests
- [ ] Implement session resources
- [ ] **Run FizzBuzz integration test (Python)**
- [ ] Fix bugs, optimize

### Week 4: Ruby Validation
- [ ] Research Ruby debugger (rdbg)
- [ ] Create Ruby adapter configuration
- [ ] Implement Ruby launch template
- [ ] Create Ruby test fixtures
- [ ] **Run FizzBuzz integration test (Ruby)**
- [ ] Document findings
- [ ] Refactor abstraction layer if needed

---

## Testing Strategy

### Unit Tests (Per Component)

**Coverage Target**: 80%+

- `src/mcp/transport.rs` - STDIO read/write
- `src/mcp/protocol.rs` - JSON-RPC parsing
- `src/dap/transport.rs` - Wire protocol encoding/decoding
- `src/dap/client.rs` - Request/response correlation
- `src/debug/session.rs` - State machine transitions
- `src/adapters/config.rs` - Adapter configuration
- Each MCP tool handler

### Integration Tests

**Main Test**: FizzBuzz debugging scenario (Python + Ruby)

**Additional Tests**:
- Simple script (no breakpoints, just run to completion)
- Exception handling (crash at line X)
- Multi-threaded program (if debugger supports it)
- Invalid breakpoint (line doesn't exist)
- Evaluate invalid expression

### Manual Testing with Claude Desktop

1. Add to `claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "debugger": {
         "command": "/path/to/debugger_mcp",
         "args": ["serve", "--verbose"]
       }
     }
   }
   ```

2. Restart Claude Desktop

3. Test conversation:
   ```
   User: "Can you debug this Python script for me?"
   [Attach fizzbuzz.py]

   Claude: [Uses debugger_start, sets breakpoints, inspects variables]
   ```

---

## Success Metrics

### MVP Completion (Week 3)

✅ FizzBuzz integration test passes for Python
✅ All unit tests pass
✅ Can start debugger via Claude Desktop
✅ Can set breakpoints and inspect variables
✅ No memory leaks (processes cleaned up)
✅ CLI `debugger_mcp serve` works reliably

### Ruby Validation (Week 4)

✅ FizzBuzz integration test passes for Ruby
✅ Same test code works for both languages (proof of abstraction)
✅ Documented any language-specific quirks
✅ Confidence to add more languages

---

## Next Steps After MVP

1. **Week 5**: Add Node.js support
2. **Week 6**: Implement stepping (step over/into/out)
3. **Week 7**: Implement stack trace and variable resources
4. **Week 8**: Performance testing and optimization
5. **Week 9+**: Advanced features (conditional breakpoints, attach mode, etc.)

---

## Questions & Answers

### Q: Why FizzBuzz for testing?
**A**: Simple, familiar, exercises all debugging primitives (breakpoints, stepping, variables, evaluation), works in any language.

### Q: Why start with Python + Ruby?
**A**: Python has excellent debugger (debugpy), Ruby validates abstraction works, both are interpreted languages (simpler to debug than compiled).

### Q: Can we test without Claude Desktop?
**A**: Yes! Integration tests use programmatic MCP client (send JSON-RPC over STDIO).

### Q: How do we test DAP protocol without real debuggers?
**A**: Unit tests use mock transports. Integration tests use real debuggers (debugpy, rdbg).

### Q: What if a test fails?
**A**: TDD process - fix the minimal code to make it pass, don't over-engineer.

---

**Ready to start coding!** 🚀

Start with:
```bash
cargo new --lib debugger_mcp
cd debugger_mcp
# Add clap to Cargo.toml
cargo add clap --features derive
cargo add tokio --features full
# Write first test
# Make it pass
# Repeat
```
