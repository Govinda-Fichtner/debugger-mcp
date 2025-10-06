# Claude Code Configuration - DAP MCP Server

## Project Overview

This project implements a **Debug Adapter Protocol (DAP) based Model Context
Protocol (MCP) server** in Rust, enabling AI coding agents (like Claude Code,
Gemini CLI, and Codex) to programmatically debug applications across multiple
programming languages.

### Vision

Enable autonomous debugging by AI agents through a unified, language-agnostic
interface that leverages proven standards (DAP for debugging, MCP for AI
integration).

### Key Innovation

**First-of-its-kind MCP server** that bridges the 40+ existing DAP debugger
implementations to AI coding assistants, unlocking a new category of
AI-assisted development: autonomous debugging.

---

## Architecture

### High-Level Design

```
AI Agent (Claude Desktop, etc.)
    ↕ MCP Protocol (JSON-RPC 2.0 over STDIO)
┌─────────────────────────────────────────┐
│     DAP MCP Server (Rust/Tokio)         │
│  ┌────────────────────────────────────┐ │
│  │  MCP Protocol Layer                │ │
│  │  - STDIO transport                 │ │
│  │  - Resources (session state)       │ │
│  │  - Tools (debug operations)        │ │
│  └──────────────┬─────────────────────┘ │
│  ┌──────────────┴─────────────────────┐ │
│  │  Debug Abstraction Layer           │ │
│  │  - SessionManager                  │ │
│  │  - Language-agnostic API           │ │
│  │  - State machine                   │ │
│  └──────────────┬─────────────────────┘ │
│  ┌──────────────┴─────────────────────┐ │
│  │  DAP Client                        │ │
│  │  - Protocol implementation         │ │
│  │  - Request/response correlation    │ │
│  │  - Event processing                │ │
│  └──────────────┬─────────────────────┘ │
│  ┌──────────────┴─────────────────────┐ │
│  │  Process Manager                   │ │
│  │  - Adapter spawning (debugpy, etc) │ │
│  │  - STDIO/TCP transport             │ │
│  │  - Lifecycle management            │ │
│  └────────────────────────────────────┘ │
└─────────────────┼─────────────────────────┘
                  ↕ Debug Adapter Protocol
        ┌─────────┼──────────┐
   debugpy   node-debug   delve  CodeLLDB
   (Python)  (Node.js)    (Go)   (Rust/C++)
```

### Layered Architecture

**Layer 1: MCP Protocol Layer** (`src/mcp/`)
- Handles JSON-RPC 2.0 over STDIO
- Exposes Resources (read-only state) and Tools (actions)
- Routes requests to debug abstraction layer

**Layer 2: Debug Abstraction Layer** (`src/debug/`)
- Provides language-agnostic debugging API
- Manages session lifecycle and state machines
- Coordinates between MCP and DAP

**Layer 3: DAP Client Layer** (`src/dap/`)
- Implements Debug Adapter Protocol
- Async request/response with correlation
- Event stream processing

**Layer 4: Process Management** (`src/process/`)
- Spawns and monitors debugger adapter processes
- Handles STDIO/TCP communication
- Implements retry and recovery logic

### Key Design Principles

1. **Language Agnostic**: No Python/Ruby/etc-specific code in core
2. **Async First**: Tokio throughout for non-blocking I/O
3. **State Machines**: Explicit state transitions for debugging sessions
4. **Actor Model**: Sessions isolated via per-session tasks
5. **Capability Negotiation**: Dynamic feature detection via DAP
6. **Graceful Degradation**: Clear errors when features unsupported

---

## Methodology

### Development Approach: Test-Driven Development (TDD)

**Red-Green-Refactor Cycle:**
1. **Red**: Write failing test first
2. **Green**: Write minimal code to pass
3. **Refactor**: Improve code quality
4. **Repeat**: For each feature

**Testing Strategy:**
- **Unit Tests**: Each component in isolation (80%+ coverage target)
- **Integration Tests**: End-to-end debugging scenarios
- **FizzBuzz Test**: Main validation scenario (Python + Ruby)

### Implementation Phases

**Phase 1: MVP - Python Support (Weeks 1-3)**
- Focus: Core functionality with debugpy
- Deliverable: Working Python debugger via MCP
- Success: FizzBuzz integration test passes

**Phase 2: Ruby Validation (Week 4)**
- Focus: Prove language abstraction works
- Deliverable: Ruby debugger with same test code
- Success: Architecture validated as language-agnostic

**Phase 3: Multi-Language (Weeks 5-8)**
- Focus: Node.js, Go, Rust support
- Deliverable: 5 languages working
- Success: Plugin architecture validated

**Phase 4: Production (Weeks 9-12)**
- Focus: Advanced features, performance, security
- Deliverable: Production-ready v1.0
- Success: All acceptance criteria met

### Commit Conventions

Following [Conventional Commits](https://www.conventionalcommits.org/) and
[Tim Pope's guidelines](https://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html):

**Format:**
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, etc)
- `refactor`: Code restructuring
- `perf`: Performance improvements
- `test`: Adding/updating tests
- `chore`: Maintenance tasks

**Example:**
```
feat(dap): implement DAP wire protocol transport

Add support for DAP's Content-Length header format and JSON-RPC
message parsing. Implements both async read and write operations
over STDIO using Tokio.

- Add DapTransport struct with AsyncWrite/AsyncBufRead
- Implement header parsing with proper error handling
- Add tests for encoding/decoding edge cases

Closes #12
```

---

## Technology Stack

### Core Dependencies

| Crate | Purpose | Justification |
|-------|---------|---------------|
| `clap` | CLI framework | Industry standard, derive macros |
| `tokio` | Async runtime | Comprehensive, battle-tested |
| `serde` + `serde_json` | Serialization | De facto standard for JSON |
| `anyhow` + `thiserror` | Error handling | Ergonomic, clear error messages |
| `tracing` | Logging | Structured, async-aware |
| `flume` | Async channels | Fast MPMC for event streaming |
| `async-trait` | Trait async methods | Required for async traits |
| `uuid` | Session IDs | Secure, non-deterministic IDs |

### Why Rust?

1. **Performance**: Zero-cost abstractions, minimal overhead
2. **Safety**: Memory safety prevents entire bug classes
3. **Concurrency**: Tokio enables efficient async I/O
4. **Ecosystem**: Mature crates for all needs
5. **Reliability**: Type system catches errors at compile time

### Why Tokio?

1. **Industry Standard**: Most popular async runtime
2. **Comprehensive**: Process, filesystem, network support
3. **Production Ready**: Used by Discord, AWS, etc.
4. **Rich Ecosystem**: Compatible with many crates

---

## MCP Interface Design

### Resources (Read-Only State)

Resources expose debugging state to AI agents:

1. `debugger://sessions` - List all active debug sessions
2. `debugger://sessions/{id}` - Session details and state
3. `debugger://breakpoints` - All breakpoints across sessions
4. `debugger://sessions/{id}/stackTrace` - Current call stack
5. `debugger://sessions/{id}/frames/{frameId}/variables` - Variables in scope

**Design Rationale:**
- Resources are cacheable and read-only
- AI can query state without side effects
- Server-Sent Events (SSE) for real-time updates

### Tools (Debugging Actions)

Tools enable AI to control debugging:

**Session Management:**
- `debugger_start` - Launch or attach to program
- `debugger_stop` - Terminate debug session

**Execution Control:**
- `debugger_continue` - Resume execution
- `debugger_pause` - Pause execution
- `debugger_step_over` - Step over function calls
- `debugger_step_into` - Step into function calls
- `debugger_step_out` - Step out of current function

**Breakpoints:**
- `debugger_set_breakpoint` - Set source breakpoint
- `debugger_remove_breakpoint` - Remove breakpoint
- `debugger_set_exception_breakpoints` - Configure exception handling

**Inspection:**
- `debugger_evaluate` - Evaluate expression
- `debugger_get_variables` - Get variables in scope

**Design Rationale:**
- Each tool does one thing well
- Composable for complex workflows
- Clear error messages for AI understanding

---

## Multi-Language Abstraction

### The Challenge

Different languages have different:
- Debuggers (GDB, pdb, Delve, etc.)
- Runtime models (compiled vs interpreted)
- Debug capabilities (not all support data breakpoints)
- Launch mechanisms (binary vs interpreter + script)

### The Solution: Adapter Registry Pattern

**Adapter Configuration:**
```rust
pub struct AdapterConfig {
    pub id: String,              // "debugpy", "delve", etc.
    pub language: String,        // "python", "go", etc.
    pub adapter_type: AdapterType,
    pub spawn_config: SpawnConfig,
    pub capabilities: AdapterCapabilities,
}

pub enum AdapterType {
    Executable { command: String, args: Vec<String> },
    Server { host: String, port: u16 },
    Pipe { path: String },
}
```

**Adding New Language:**
1. Create `AdapterConfig` (10 lines of code)
2. Implement launch template (JSON transformation)
3. Register in adapter registry
4. Test with sample program

**No MCP interface changes needed!**

### Capability Negotiation

Not all debuggers support all features. The server handles this via:

1. **Initialization**: Query adapter capabilities via DAP `initialize`
2. **Validation**: Check capabilities before tool execution
3. **Graceful Errors**: Return clear error messages when unsupported
4. **Alternatives**: Suggest alternative approaches when possible

**Example:**
```json
{
  "error": {
    "code": -32004,
    "message": "Data breakpoints not supported by debugpy",
    "data": {
      "feature": "data_breakpoints",
      "alternative": "Use conditional breakpoints instead"
    }
  }
}
```

---

## Concurrency Model

### Actor Pattern for Session Isolation

Each debug session runs as an independent Tokio task (actor):

```rust
// Session manager holds Arc<RwLock<HashMap<SessionId, Session>>>
// Each session spawns actor task
tokio::spawn(async move {
    session_actor(session, event_rx).await
});
```

**Benefits:**
- Sessions don't interfere with each other
- Crash in one session doesn't affect others
- Natural message passing via channels

### Request/Response Correlation

DAP uses sequence numbers to correlate requests with responses:

```rust
// Sender side
let (tx, rx) = oneshot::channel();
let seq = next_seq();
pending_requests.insert(seq, tx);
send_request(Request { seq, ... });
let response = rx.await?;

// Receiver side (event loop)
if msg.type == "response" {
    if let Some(tx) = pending_requests.remove(&msg.request_seq) {
        tx.send(msg.into_response());
    }
}
```

**Benefits:**
- Concurrent requests without blocking
- Type-safe response handling
- Clear timeout handling

### Shared State Management

Use `Arc<RwLock<T>>` for read-heavy shared state:

```rust
// Session map (many readers, few writers)
type SessionMap = Arc<RwLock<HashMap<SessionId, Arc<RwLock<Session>>>>>;

// Read access (concurrent)
let session = sessions.read().await.get(id).cloned();

// Write access (exclusive)
sessions.write().await.insert(id, session);
```

**Why RwLock?**
- Debugging queries are read-heavy (stack traces, variables)
- Only writes on state changes (breakpoint hit, step)
- Better performance than Mutex for this workload

---

## Error Handling Strategy

### Error Types

```rust
pub enum Error {
    SessionNotFound(String),        // -32001
    AdapterNotFound(String),        // -32002
    InvalidState { ... },           // -32003
    UnsupportedFeature(String),     // -32004
    MissingParam(String),           // -32602
    DapProtocol(String),           // -32005
    Timeout(String),               // -32006
    Internal(String),              // -32603
}
```

### Error Propagation

Use `?` operator with `anyhow::Result` in application code:

```rust
pub async fn start_debugger(
    language: &str,
    program: &str,
) -> Result<String, Error> {
    let adapter = registry.get(language)
        .ok_or_else(|| Error::AdapterNotFound(language.into()))?;

    let session = SessionManager::create(adapter).await?;

    Ok(session.id)
}
```

### AI-Friendly Errors

Errors include context and suggestions:

```json
{
  "error": {
    "code": -32003,
    "message": "Cannot step: session is running, not paused",
    "data": {
      "current_state": "running",
      "required_state": "paused",
      "suggestion": "Use debugger_pause first, then debugger_step_over"
    }
  }
}
```

---

## Testing Strategy

### Unit Tests (Per Component)

**Target: 80%+ code coverage**

Each module has comprehensive unit tests:

```rust
// src/dap/transport.rs
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_encode_message() {
        let msg = DapMessage { ... };
        let encoded = encode(&msg).unwrap();
        assert!(encoded.starts_with(b"Content-Length: "));
    }

    #[tokio::test]
    async fn test_decode_message() {
        let input = b"Content-Length: 50\r\n\r\n{\"type\":\"request\"}";
        let msg = decode(input).await.unwrap();
        assert_eq!(msg.message_type, "request");
    }
}
```

### Integration Tests

**FizzBuzz Debugging Scenario:**

The main validation test exercises all core features:

```rust
#[tokio::test]
async fn test_fizzbuzz_debugging_python() {
    // 1. Start debugger
    let session_id = start_debugger("python", "fizzbuzz.py").await?;

    // 2. Set breakpoint
    let bp = set_breakpoint(&session_id, "fizzbuzz.py", 3).await?;
    assert!(bp.verified);

    // 3. Continue and wait for breakpoint
    continue_execution(&session_id).await?;
    wait_for_stopped(&session_id, "breakpoint").await?;

    // 4. Inspect variables
    let n = evaluate(&session_id, "n").await?;
    assert_eq!(n.result, "1");

    // 5. Step through
    step_over(&session_id).await?;

    // 6. Continue to completion
    continue_execution(&session_id).await?;
    wait_for_terminated(&session_id).await?;
}
```

**Why FizzBuzz?**
- Simple algorithm everyone understands
- Exercises loops, conditionals, functions
- Tests breakpoints, stepping, evaluation
- Same test validates Python and Ruby

### Test Utilities

**Mock DAP Client:**
```rust
pub struct MockDapClient {
    responses: HashMap<String, DapResponse>,
}

impl MockDapClient {
    pub fn expect_initialize(mut self, caps: Capabilities) -> Self {
        self.responses.insert("initialize", ...);
        self
    }
}
```

**Test Fixtures:**
```python
# tests/fixtures/fizzbuzz.py
def fizzbuzz(n):
    if n % 15 == 0: return "FizzBuzz"
    elif n % 3 == 0: return "Fizz"
    elif n % 5 == 0: return "Buzz"
    else: return str(n)
```

---

## Performance Targets

### Latency Goals

| Operation | Target (P95) | Rationale |
|-----------|--------------|-----------|
| `debugger_start` | < 500ms | Includes process spawn |
| `debugger_set_breakpoint` | < 50ms | Simple DAP request |
| `debugger_continue` | < 20ms | Just send command |
| `debugger_evaluate` | < 100ms | Expression-dependent |
| Resource read | < 10ms | Cache hit |

### Scalability Goals

- **100+ concurrent sessions**: Support many debugging sessions
- **1000+ breakpoints**: Across all sessions
- **Sub-second startup**: Server ready in < 1s

### Optimization Techniques

1. **Connection Pooling**: Reuse adapter processes when possible
2. **Lazy Loading**: Don't fetch variables until requested
3. **Caching**: Cache stack traces, invalidate on state change
4. **Pagination**: Limit frames/variables per request
5. **Async Throughout**: Non-blocking I/O everywhere

---

## Security Considerations

### Input Validation

All user inputs are validated:

```rust
fn validate_file_path(path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);

    // Prevent directory traversal
    if path.components().any(|c| c == Component::ParentDir) {
        return Err(Error::InvalidPath("No .. allowed"));
    }

    // Must be absolute
    if !path.is_absolute() {
        return Err(Error::InvalidPath("Must be absolute"));
    }

    Ok(path)
}
```

### Resource Limits

Prevent abuse via limits:

```rust
const MAX_SESSIONS: usize = 100;
const MAX_BREAKPOINTS_PER_SESSION: usize = 1000;
const MAX_EXPRESSION_LENGTH: usize = 10_000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
```

### Sandboxing

Future enhancement: Run debuggers in containers for isolation.

---

## Documentation Structure

```
debugger_mcp/
├── CLAUDE.md                      # This file - architecture & methodology
├── README.md                      # Project overview
├── GETTING_STARTED.md             # Developer quick start
├── SUMMARY.md                     # Executive summary
├── MVP_STATUS.md                  # Implementation status
├── PUSH_TO_GITHUB.md              # Git instructions
└── docs/
    ├── README.md                  # Documentation index
    ├── DAP_MCP_SERVER_PROPOSAL.md # Complete architecture (68 pages)
    ├── MVP_IMPLEMENTATION_PLAN.md # Phase 1 development guide
    ├── architecture/
    │   └── COMPONENTS.md          # Component specifications
    └── research/
        ├── dap-client-research.md
        └── rust-mcp-technology-stack.md
```

---

## Development Workflow

### Getting Started

```bash
# Clone repository
git clone https://github.com/Govinda-Fichtner/debugger-mcp.git
cd debugger-mcp

# Build
cargo build

# Run tests
cargo test

# Run server
cargo run -- serve

# Run with verbose logging
cargo run -- serve --verbose
```

### TDD Workflow

1. **Write test** (should fail)
   ```bash
   cargo test test_set_breakpoint -- --nocapture
   ```

2. **Implement feature** (make test pass)
   ```bash
   # Edit src/mcp/tools/breakpoint.rs
   cargo test test_set_breakpoint
   ```

3. **Refactor** (improve code)
   ```bash
   cargo test  # Ensure still passes
   ```

4. **Commit** (following conventions)
   ```bash
   git add src/mcp/tools/breakpoint.rs
   git commit -m "feat(mcp): implement set_breakpoint tool

   Add debugger_set_breakpoint tool for setting source breakpoints.
   Validates session state and coordinates with DAP client.

   - Add SetBreakpointTool struct
   - Implement parameter validation
   - Add unit tests for success and error cases

   Refs #5"
   ```

### Code Review Checklist

- [ ] Tests added/updated
- [ ] Error handling comprehensive
- [ ] Logging at appropriate levels
- [ ] No unsafe code (unless justified)
- [ ] Documentation comments for public APIs
- [ ] Conventional commit message
- [ ] No compiler warnings

---

## Future Enhancements

### Near-Term (3-6 months)
- Remote debugging (SSH tunnels, K8s pods)
- Time-travel debugging (record/replay)
- Enhanced visualization (call graphs, timelines)

### Long-Term (6-12 months)
- Distributed debugging (microservices)
- Machine learning integration (anomaly detection)
- IDE integration (VS Code extension)
- Cloud debugging (serverless, containers)

---

## References

### Specifications
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [Model Context Protocol](https://spec.modelcontextprotocol.io/)
- [Conventional Commits](https://www.conventionalcommits.org/)

### Implementations
- [debugpy](https://github.com/microsoft/debugpy) - Python debug adapter
- [nvim-dap](https://github.com/mfussenegger/nvim-dap) - Neovim DAP client
- [vscode-js-debug](https://github.com/microsoft/vscode-js-debug) - JavaScript adapter

### Technologies
- [Tokio](https://tokio.rs/) - Async runtime
- [Clap](https://docs.rs/clap/) - CLI framework
- [serde](https://serde.rs/) - Serialization

---

## Contributing

### Code Style
- Follow Rust conventions (rustfmt)
- Use clippy for linting
- Document public APIs
- Write tests for new features

### Commit Messages
Use conventional commits format (see above examples).

### Pull Requests
- One feature per PR
- Include tests
- Update documentation
- Pass CI checks

---

**Project**: DAP MCP Server
**Status**: Architecture Complete, Implementation Starting
**Timeline**: 3-4 weeks to MVP, 12 weeks to v1.0
**License**: TBD (MIT or Apache 2.0)

Last Updated: October 5, 2025
