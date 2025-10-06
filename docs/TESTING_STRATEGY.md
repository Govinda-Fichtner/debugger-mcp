# Testing Strategy to Achieve 95%+ Code Coverage

## Current State
- **Coverage**: 45.96% (250/544 lines)
- **Tests**: 56 unit tests
- **Fully Covered Modules**: error.rs, debug/state.rs, adapters/python.rs

## Gap Analysis

### Uncovered Code Categories

1. **I/O Transport Layers** (28% of uncovered code)
   - `src/dap/transport.rs`: 0/28 lines - Process stdio communication
   - `src/mcp/transport.rs`: 3/31 lines - Stdin/stdout JSON-RPC

2. **DAP Client** (33% of uncovered code)
   - `src/dap/client.rs`: 0/90 lines - Async process spawning & message handling

3. **Debug Sessions** (25% of uncovered code)
   - `src/debug/session.rs`: 0/67 lines - Session lifecycle management

4. **Entry Points** (14% of uncovered code)
   - `src/main.rs`, `src/lib.rs`, `src/mcp/mod.rs` - Server initialization

---

## Recommended Approach: **Trait-Based Dependency Injection + Mockall**

### Strategy Overview

**Why This Approach?**
- ✅ Non-invasive: Minimal production code changes
- ✅ Industry-standard: Uses `mockall` crate (100M+ downloads)
- ✅ Type-safe: Compile-time mock verification
- ✅ Async-friendly: Full tokio support
- ✅ Maintainable: Clear separation of concerns

### Libraries to Add

```toml
[dev-dependencies]
mockall = "0.13.1"          # Mock generation for traits
tokio-test = "0.4.4"        # Already included - async test helpers
assert_matches = "1.5.0"    # Pattern matching in tests
bytes = "1.10.1"            # For byte stream testing
```

---

## Implementation Plan

### Phase 1: Refactor Transport Layers (Target: +15% coverage)

**Problem**: Transport layers directly use `tokio::io::stdin/stdout` and `Child` processes, making them untestable.

**Solution**: Extract I/O behind traits

#### 1.1 Create Transport Traits

```rust
// src/dap/transport_trait.rs
use crate::{Error, Result};
use super::types::Message;
use async_trait::async_trait;

#[async_trait]
pub trait DapTransportTrait: Send + Sync {
    async fn read_message(&mut self) -> Result<Message>;
    async fn write_message(&mut self, msg: &Message) -> Result<()>;
}

// src/mcp/transport_trait.rs
use crate::{Error, Result};
use super::protocol::JsonRpcMessage;
use async_trait::async_trait;

#[async_trait]
pub trait McpTransportTrait: Send + Sync {
    async fn read_message(&mut self) -> Result<JsonRpcMessage>;
    async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()>;
}
```

#### 1.2 Implement Traits for Existing Code

```rust
// src/dap/transport.rs - Add trait implementation
#[async_trait]
impl DapTransportTrait for DapTransport {
    async fn read_message(&mut self) -> Result<Message> {
        // Existing implementation
    }

    async fn write_message(&mut self, msg: &Message) -> Result<()> {
        // Existing implementation
    }
}
```

#### 1.3 Create Mock Transport for Tests

```rust
// src/dap/transport.rs - Add test module
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use bytes::Bytes;

    mock! {
        pub DapTransport {}

        #[async_trait]
        impl DapTransportTrait for DapTransport {
            async fn read_message(&mut self) -> Result<Message>;
            async fn write_message(&mut self, msg: &Message) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_transport_message_parsing() {
        let mut mock_transport = MockDapTransport::new();

        // Setup expectations
        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| Ok(Message::Response(Response {
                seq: 1,
                request_seq: 1,
                command: "initialize".to_string(),
                success: true,
                message: None,
                body: Some(json!({"capabilities": {}})),
            })));

        let msg = mock_transport.read_message().await.unwrap();
        // Assertions...
    }
}
```

**Expected Coverage Gain**: 28/59 transport lines = +5.1%

---

### Phase 2: Refactor DAP Client with Testable Process Spawning (Target: +20% coverage)

**Problem**: `DapClient::spawn()` creates real child processes with `tokio::process::Command`.

**Solution**: Use dependency injection for process spawning

#### 2.1 Create Process Spawner Trait

```rust
// src/dap/process_spawner.rs
use crate::{Error, Result};
use async_trait::async_trait;
use tokio::process::Child;

#[async_trait]
pub trait ProcessSpawner: Send + Sync {
    async fn spawn(&self, command: &str, args: &[String]) -> Result<SpawnedProcess>;
}

pub struct SpawnedProcess {
    pub stdin: Box<dyn AsyncWrite + Send + Unpin>,
    pub stdout: Box<dyn AsyncRead + Send + Unpin>,
    pub child: Box<dyn ProcessHandle>,
}

pub trait ProcessHandle: Send + Sync {
    // Minimal interface for managing process lifetime
}

// Production implementation
pub struct TokioProcessSpawner;

#[async_trait]
impl ProcessSpawner for TokioProcessSpawner {
    async fn spawn(&self, command: &str, args: &[String]) -> Result<SpawnedProcess> {
        // Existing spawn logic from DapClient::spawn
    }
}
```

#### 2.2 Refactor DapClient Constructor

```rust
// src/dap/client.rs - Refactored
pub struct DapClient {
    transport: Arc<RwLock<Box<dyn DapTransportTrait>>>,
    seq_counter: Arc<AtomicI32>,
    pending_requests: Arc<RwLock<HashMap<i32, ResponseSender>>>,
    event_tx: mpsc::UnboundedSender<Event>,
}

impl DapClient {
    // Production constructor (unchanged API)
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        let spawner = TokioProcessSpawner;
        Self::spawn_with_spawner(command, args, Box::new(spawner)).await
    }

    // Testable constructor (dependency injection)
    #[cfg(test)]
    pub async fn spawn_with_spawner(
        command: &str,
        args: &[String],
        spawner: Box<dyn ProcessSpawner>,
    ) -> Result<Self> {
        let process = spawner.spawn(command, args).await?;
        // Continue with existing logic using process.stdin/stdout
    }
}
```

#### 2.3 Mock DAP Client Tests

```rust
// src/dap/client.rs - Tests
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use tokio::io::duplex;

    mock! {
        ProcessSpawner {}

        #[async_trait]
        impl ProcessSpawner for ProcessSpawner {
            async fn spawn(&self, command: &str, args: &[String]) -> Result<SpawnedProcess>;
        }
    }

    #[tokio::test]
    async fn test_dap_client_initialize() {
        // Create in-memory duplex channel
        let (client_writer, server_reader) = duplex(1024);
        let (server_writer, client_reader) = duplex(1024);

        let mut mock_spawner = MockProcessSpawner::new();
        mock_spawner
            .expect_spawn()
            .returning(move |_, _| {
                Ok(SpawnedProcess {
                    stdin: Box::new(client_writer),
                    stdout: Box::new(client_reader),
                    child: Box::new(MockProcessHandle),
                })
            });

        // Spawn background task to simulate DAP adapter responses
        tokio::spawn(async move {
            // Write mock DAP initialize response to server_writer
            let response = Message::Response(Response {
                seq: 1,
                request_seq: 1,
                command: "initialize".to_string(),
                success: true,
                body: Some(json!({"capabilities": {}})),
                message: None,
            });
            // Write Content-Length header + JSON
        });

        let client = DapClient::spawn_with_spawner(
            "mock-debugger",
            &[],
            Box::new(mock_spawner),
        ).await.unwrap();

        let caps = client.initialize("mock-adapter").await.unwrap();
        assert!(caps.supports_configuration_done_request.is_some());
    }
}
```

**Expected Coverage Gain**: 70/90 DAP client lines = +12.8%

---

### Phase 3: Integration Tests with Fake DAP Adapter (Target: +15% coverage)

**Problem**: Session and manager code requires end-to-end DAP protocol flow.

**Solution**: Create a minimal fake DAP adapter server for tests

#### 3.1 Create Fake DAP Adapter

```rust
// tests/fixtures/fake_dap_adapter.rs
use debugger_mcp::dap::types::*;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

pub struct FakeDapAdapter {
    child: Child,
}

impl FakeDapAdapter {
    pub async fn start() -> Self {
        // Spawn test helper binary that implements basic DAP protocol
        let child = Command::new(env!("CARGO_BIN_EXE_fake_dap_adapter"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        Self { child }
    }

    pub fn host(&self) -> &str { "localhost" }
    pub fn port(&self) -> u16 { 4711 }
}

// Separate binary: tests/bin/fake_dap_adapter.rs
#[tokio::main]
async fn main() {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();

    // Simple state machine responding to DAP requests
    loop {
        let msg = read_dap_message(&mut stdin).await;
        match msg.command.as_str() {
            "initialize" => {
                write_dap_response(&mut stdout, Response {
                    seq: msg.seq,
                    request_seq: msg.seq,
                    command: "initialize".to_string(),
                    success: true,
                    body: Some(json!({"capabilities": {}})),
                    message: None,
                }).await;
            }
            "launch" => { /* ... */ }
            "setBreakpoints" => { /* ... */ }
            _ => {}
        }
    }
}
```

#### 3.2 Integration Tests Using Fake Adapter

```rust
// tests/integration/session_lifecycle.rs
use debugger_mcp::debug::SessionManager;
mod fixtures;

#[tokio::test]
async fn test_full_debug_session_lifecycle() {
    let fake_adapter = fixtures::FakeDapAdapter::start().await;

    let manager = SessionManager::new();

    // Create session
    let session_id = manager
        .create_session("python", "test.py".to_string(), vec![], None)
        .await
        .unwrap();

    // Get session
    let session = manager.get_session(&session_id).await.unwrap();

    // Set breakpoint
    let verified = session
        .set_breakpoint("test.py".to_string(), 10)
        .await
        .unwrap();
    assert!(verified);

    // Continue execution
    session.continue_execution().await.unwrap();

    // Get stack trace
    let frames = session.stack_trace().await.unwrap();
    assert!(!frames.is_empty());

    // Cleanup
    manager.remove_session(&session_id).await.unwrap();
}
```

**Expected Coverage Gain**: 67/67 session + 20/33 manager = +16.0%

---

### Phase 4: MCP Transport Testing (Target: +5% coverage)

**Problem**: `StdioTransport` reads from actual stdin/stdout.

**Solution**: Use `tokio_test::io` for in-memory I/O testing

```rust
// src/mcp/transport.rs - Tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_read_json_rpc_message() {
        // Create mock stdin with Content-Length header
        let mock_stdin = Builder::new()
            .read(b"Content-Length: 47\r\n\r\n")
            .read(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\"}")
            .build();

        let mock_stdout = Builder::new().build();

        let mut transport = StdioTransport {
            stdin: BufReader::new(mock_stdin),
            stdout: mock_stdout,
        };

        let msg = transport.read_message().await.unwrap();
        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "test");
                assert_eq!(req.id, json!(1));
            }
            _ => panic!("Expected request"),
        }
    }

    #[tokio::test]
    async fn test_write_json_rpc_message() {
        let mock_stdin = Builder::new().build();
        let mock_stdout = Builder::new()
            .write(b"Content-Length: 47\r\n\r\n")
            .write(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\"}")
            .build();

        let mut transport = StdioTransport {
            stdin: BufReader::new(mock_stdin),
            stdout: mock_stdout,
        };

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test".to_string(),
            params: None,
        };

        transport.write_message(&JsonRpcMessage::Request(req)).await.unwrap();
    }
}
```

**Expected Coverage Gain**: 28/31 MCP transport lines = +5.1%

---

## Summary: Expected Total Coverage

| Phase | Target Area | Lines to Cover | Estimated Coverage Gain |
|-------|-------------|----------------|-------------------------|
| Current | - | - | **45.96%** |
| Phase 1 | Transport traits & mocks | 59 | +5.1% → **51.06%** |
| Phase 2 | DAP Client with DI | 90 | +12.8% → **63.86%** |
| Phase 3 | Integration tests | 87 | +16.0% → **79.86%** |
| Phase 4 | MCP transport tests | 31 | +5.7% → **85.56%** |
| Stretch | Entry points & remaining | 50 | +9.2% → **94.76%** |

---

## Implementation Timeline

### Week 1: Foundation (Phase 1)
- Add `mockall` and dependencies
- Create transport traits
- Write transport layer tests
- **Milestone**: 51% coverage

### Week 2: Core Testing (Phase 2)
- Refactor DapClient with dependency injection
- Implement process spawner mocks
- Write comprehensive DAP client tests
- **Milestone**: 64% coverage

### Week 3: Integration (Phase 3)
- Build fake DAP adapter binary
- Write end-to-end session tests
- Test manager lifecycle
- **Milestone**: 80% coverage

### Week 4: Polish (Phase 4 + Stretch)
- MCP transport in-memory tests
- Edge cases and error paths
- Documentation and examples
- **Milestone**: 95%+ coverage

---

## Alternative Approaches Considered

### ❌ Option 1: Full E2E with Real Debuggers
**Pros**: Tests actual integrations
**Cons**: Slow, flaky, platform-dependent, requires Python/debugpy installation
**Verdict**: Too fragile for CI/CD

### ❌ Option 2: Async-process Mocking Only
**Pros**: Simpler, fewer abstractions
**Cons**: Can't test protocol logic, misses transport bugs
**Verdict**: Insufficient coverage gains

### ✅ Option 3: Trait-Based DI + Mockall (RECOMMENDED)
**Pros**: Type-safe, maintainable, comprehensive testing
**Cons**: Requires modest refactoring
**Verdict**: Best balance of coverage, maintainability, and reliability

---

## Additional Recommendations

1. **CI/CD Integration**
   ```yaml
   # .github/workflows/coverage.yml
   - name: Run coverage
     run: cargo tarpaulin --out Xml
   - name: Upload to Codecov
     uses: codecov/codecov-action@v3
   - name: Enforce 95% threshold
     run: |
       coverage=$(grep -oP 'line-rate="\K[^"]+' cobertura.xml | head -1)
       if (( $(echo "$coverage < 0.95" | bc -l) )); then
         echo "Coverage $coverage below 95% threshold"
         exit 1
       fi
   ```

2. **Documentation**
   - Add testing guide to README
   - Document mock usage patterns
   - Provide test examples for contributors

3. **Continuous Improvement**
   - Run coverage reports on every PR
   - Set coverage diff threshold (no regressions)
   - Track coverage trends over time

---

## Conclusion

By implementing trait-based dependency injection with the `mockall` library, we can achieve **95%+ code coverage** while maintaining:
- Clean, testable architecture
- Fast, reliable test suite
- Type safety and compile-time guarantees
- Minimal impact on production code

The phased approach allows incremental progress with measurable milestones at each stage.
