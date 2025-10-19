# Component Architecture Details

This document provides detailed specifications for each component in the DAP MCP Server.

## Component Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                         main.rs                                  │
│                    (Application Entry)                           │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                     mcp::McpServer                               │
│  Dependencies:                                                   │
│  • Arc<SessionManager>                                          │
│  • HashMap<String, Box<dyn ResourceHandler>>                    │
│  • HashMap<String, Box<dyn ToolHandler>>                        │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                debug::SessionManager                             │
│  Dependencies:                                                   │
│  • Arc<RwLock<HashMap<SessionId, Arc<RwLock<DebugSession>>>>>  │
│  • Arc<AdapterRegistry>                                         │
└────────────────────────┬────────────────────────────────────────┘
                         │
         ┌───────────────┴────────────────┐
         ▼                                ▼
┌──────────────────────┐      ┌──────────────────────────┐
│ debug::DebugSession  │      │ adapters::AdapterRegistry│
│  Dependencies:       │      │  Dependencies:           │
│  • Arc<DapClient>    │      │  • HashMap<String,       │
│  • State machine     │      │    AdapterConfig>        │
└──────┬───────────────┘      └──────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│                     dap::DapClient                               │
│  Dependencies:                                                   │
│  • Arc<DapTransport>                                            │
│  • flume::Sender/Receiver<DapEvent>                            │
│  • Arc<RwLock<HashMap<RequestId, oneshot::Sender<Response>>>>  │
└────────────────────────┬────────────────────────────────────────┘
                         │
         ┌───────────────┴────────────────┐
         ▼                                ▼
┌──────────────────────┐      ┌──────────────────────────┐
│ dap::DapTransport    │      │ process::ProcessManager  │
│  Dependencies:       │      │  Dependencies:           │
│  • AsyncWrite/Read   │      │  • tokio::process::Child │
└──────────────────────┘      └──────────────────────────┘
```

## Module Structure

```
src/
├── main.rs                         # Entry point, Tokio runtime
├── mcp/
│   ├── mod.rs                      # MCP server
│   ├── resources.rs                # Resource handlers
│   ├── tools.rs                    # Tool handlers
│   └── transport.rs                # STDIO/HTTP transport
├── debug/
│   ├── mod.rs                      # Session manager
│   ├── session.rs                  # DebugSession type
│   ├── state.rs                    # State machine
│   └── templates.rs                # Launch config templates
├── dap/
│   ├── mod.rs                      # DAP client
│   ├── client.rs                   # Request/response handling
│   ├── events.rs                   # Event processing
│   ├── transport.rs                # DAP wire protocol
│   └── types.rs                    # DAP message types
├── adapters/
│   ├── mod.rs                      # Adapter registry
│   ├── python.rs                   # Python (debugpy) config
│   ├── nodejs.rs                   # Node.js config
│   ├── go.rs                       # Go (delve) config
│   └── rust.rs                     # Rust (CodeLLDB) config
├── process/
│   ├── mod.rs                      # Process manager
│   └── manager.rs                  # Adapter process spawning
├── error.rs                        # Error types
└── utils.rs                        # Utilities
```

## Component Specifications

### 1. MCP Server (`src/mcp/mod.rs`)

**Purpose**: Handle MCP protocol, route requests to debug layer

**Key Types**:
```rust
pub struct McpServer {
    session_manager: Arc<SessionManager>,
    resource_handlers: HashMap<String, Box<dyn ResourceHandler>>,
    tool_handlers: HashMap<String, Box<dyn ToolHandler>>,
}

impl McpServer {
    pub async fn run(&self) -> Result<(), Error>;
    fn register_resources(&mut self);
    fn register_tools(&mut self);
}

#[async_trait]
pub trait ResourceHandler: Send + Sync {
    async fn handle(&self, uri: &str) -> Result<Resource, Error>;
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn handle(&self, params: serde_json::Value)
        -> Result<serde_json::Value, Error>;
}
```

**Concurrency**: Single task reading from STDIN, spawns tasks for each request

**Error Handling**: Returns JSON-RPC error responses, never panics

---

### 2. Session Manager (`src/debug/mod.rs`)

**Purpose**: Manage lifecycle of debug sessions

**Key Types**:
```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<DebugSession>>>>>,
    adapter_registry: Arc<AdapterRegistry>,
}

impl SessionManager {
    pub async fn create_session(
        &self,
        language: &str,
        config: LaunchConfig,
    ) -> Result<String, Error> {
        // 1. Get adapter config from registry
        // 2. Spawn adapter process
        // 3. Create DAP client
        // 4. Create DebugSession
        // 5. Initialize session
        // 6. Store in sessions map
        // 7. Return session ID
    }

    pub async fn get_session(
        &self,
        id: &str,
    ) -> Result<Arc<RwLock<DebugSession>>, Error>;

    pub async fn list_sessions(&self) -> Vec<SessionInfo>;

    pub async fn terminate_session(&self, id: &str) -> Result<(), Error>;
}
```

**Concurrency**:
- Read-heavy: `RwLock` for sessions map
- Write operations: Exclusive lock only during add/remove
- Session actors run independently

**Lifecycle**:
1. `create_session`: Spawn adapter, initialize, store
2. Session runs independently via internal tasks
3. `terminate_session`: Disconnect adapter, cleanup, remove from map

---

### 3. Debug Session (`src/debug/session.rs`)

**Purpose**: Represent a single debugging session

**Key Types**:
```rust
pub struct DebugSession {
    pub id: String,
    pub state: SessionState,
    pub language: String,
    pub adapter_id: String,
    pub program: Option<String>,
    pub pid: Option<u32>,
    pub capabilities: AdapterCapabilities,
    pub breakpoints: Vec<Breakpoint>,
    pub threads: Vec<Thread>,
    pub current_frame: Option<i64>,
    dap_client: Arc<DapClient>,
    event_task: Option<JoinHandle<()>>,
}

pub enum SessionState {
    Initializing,
    Configuring,
    Launching,
    Running,
    Paused { reason: StopReason, thread_id: Option<i64> },
    Terminated { exit_code: Option<i32> },
}

impl DebugSession {
    pub async fn initialize(&mut self) -> Result<(), Error>;
    pub async fn launch(&mut self, config: serde_json::Value) -> Result<(), Error>;
    pub async fn attach(&mut self, config: serde_json::Value) -> Result<(), Error>;

    pub async fn set_breakpoint(&mut self, spec: BreakpointSpec)
        -> Result<Breakpoint, Error>;
    pub async fn remove_breakpoint(&mut self, id: i64) -> Result<(), Error>;

    pub async fn continue_execution(&mut self, thread_id: Option<i64>)
        -> Result<(), Error>;
    pub async fn pause(&mut self, thread_id: Option<i64>) -> Result<(), Error>;
    pub async fn step_over(&mut self, thread_id: i64) -> Result<(), Error>;
    pub async fn step_into(&mut self, thread_id: i64) -> Result<(), Error>;
    pub async fn step_out(&mut self, thread_id: i64) -> Result<(), Error>;

    pub async fn get_stack_trace(&mut self, thread_id: i64)
        -> Result<Vec<StackFrame>, Error>;
    pub async fn get_variables(&mut self, var_ref: i64)
        -> Result<Vec<Variable>, Error>;
    pub async fn evaluate(&mut self, expr: &str, frame_id: Option<i64>)
        -> Result<EvalResult, Error>;

    pub async fn disconnect(&mut self, terminate: bool) -> Result<(), Error>;

    fn spawn_event_processor(&mut self);
}
```

**State Transitions**:
```
Initializing → (initialize) → Configuring
Configuring → (launch/attach) → Launching
Launching → (process started) → Running
Running → (stopped event) → Paused
Paused → (continue/step) → Running
Any → (terminated event) → Terminated
```

**Event Processing**:
- Separate Tokio task processes DAP events
- Updates session state based on events
- Stores thread/frame state
- Invalidates caches on state changes

---

### 4. DAP Client (`src/dap/client.rs`)

**Purpose**: Implement Debug Adapter Protocol client

**Key Types**:
```rust
pub struct DapClient {
    process: Arc<RwLock<Option<Child>>>,
    transport: Arc<DapTransport>,
    event_sender: flume::Sender<DapEvent>,
    event_receiver: flume::Receiver<DapEvent>,
    request_id: Arc<AtomicU64>,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<DapResponse>>>>,
}

impl DapClient {
    pub async fn new(
        adapter_config: &AdapterConfig,
    ) -> Result<Self, Error> {
        // 1. Spawn adapter process (ProcessManager)
        // 2. Create DapTransport from process STDIO
        // 3. Spawn event processing task
        // 4. Return client
    }

    async fn send_request<R: DapRequest>(
        &self,
        request: R,
    ) -> Result<R::Response, Error> {
        // 1. Generate sequence number
        // 2. Create oneshot channel for response
        // 3. Store in pending_requests map
        // 4. Serialize and send via transport
        // 5. Await response on oneshot channel
        // 6. Deserialize and return
    }

    pub async fn initialize(&self, caps: ClientCapabilities)
        -> Result<ServerCapabilities, Error> {
        self.send_request(InitializeRequest { ... }).await
    }

    pub async fn launch(&self, config: serde_json::Value)
        -> Result<(), Error> {
        self.send_request(LaunchRequest { config }).await
    }

    // ... other DAP requests

    pub fn subscribe_events(&self) -> flume::Receiver<DapEvent> {
        self.event_receiver.clone()
    }
}
```

**Request/Response Flow**:
```
send_request()
    ↓
1. Generate seq number
    ↓
2. Create oneshot channel
    ↓
3. Store in pending_requests[seq] = oneshot_sender
    ↓
4. Serialize request to JSON
    ↓
5. Send via DapTransport
    ↓
6. Await on oneshot_receiver
    ↓
[Event processing task receives response]
    ↓
7. Match response.request_seq
    ↓
8. Remove pending_requests[seq]
    ↓
9. Send response to oneshot_sender
    ↓
[send_request() receives response]
    ↓
10. Deserialize and return
```

**Event Processing**:
```rust
async fn process_events(
    transport: Arc<DapTransport>,
    event_sender: flume::Sender<DapEvent>,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<Response>>>>,
) {
    loop {
        match transport.receive().await {
            Ok(msg) => match msg.message_type {
                "response" => {
                    // Send to waiting request via oneshot
                    if let Some(tx) = pending_requests.write().await
                        .remove(&msg.request_seq) {
                        tx.send(msg.into_response()).ok();
                    }
                }
                "event" => {
                    // Broadcast to subscribers
                    event_sender.send_async(msg.into_event()).await.ok();
                }
                _ => {}
            },
            Err(e) => {
                tracing::error!("DAP transport error: {}", e);
                break;
            }
        }
    }
}
```

---

### 5. DAP Transport (`src/dap/transport.rs`)

**Purpose**: Handle DAP wire protocol (header + JSON body)

**Key Types**:
```rust
pub struct DapTransport {
    writer: Arc<Mutex<Box<dyn AsyncWrite + Send + Unpin>>>,
    reader: Arc<Mutex<Box<dyn AsyncBufRead + Send + Unpin>>>,
}

impl DapTransport {
    pub fn new_stdio(
        stdin: impl AsyncWrite + Send + Unpin + 'static,
        stdout: impl AsyncBufRead + Send + Unpin + 'static,
    ) -> Self;

    pub fn new_tcp(stream: TcpStream) -> Self;

    pub async fn send(&self, message: &DapMessage) -> Result<(), Error> {
        // 1. Serialize message to JSON
        // 2. Calculate Content-Length
        // 3. Write headers (Content-Length: N\r\n\r\n)
        // 4. Write JSON body
        // 5. Flush
    }

    pub async fn receive(&self) -> Result<DapMessage, Error> {
        // 1. Read headers line by line
        // 2. Extract Content-Length
        // 3. Read exactly N bytes for body
        // 4. Deserialize JSON
        // 5. Return DapMessage
    }
}
```

**Wire Format**:
```
Content-Length: 119\r\n
\r\n
{"seq":1,"type":"request","command":"initialize","arguments":{...}}
```

---

### 6. Adapter Registry (`src/adapters/mod.rs`)

**Purpose**: Store and provide adapter configurations

**Key Types**:
```rust
pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterConfig>,
}

pub struct AdapterConfig {
    pub id: String,
    pub language: String,
    pub adapter_type: AdapterType,
    pub spawn_config: SpawnConfig,
    pub default_capabilities: AdapterCapabilities,
}

pub enum AdapterType {
    Executable { command: String, args: Vec<String> },
    Server { host: String, port: u16 },
    Pipe { path: String },
}

pub struct SpawnConfig {
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub startup_timeout: Duration,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        // Pre-register built-in adapters
    }

    pub fn register(&mut self, config: AdapterConfig);
    pub fn get(&self, language: &str) -> Option<&AdapterConfig>;
    pub fn supported_languages(&self) -> Vec<String>;
}
```

**Built-in Adapters**:
- Python: `debugpy` (python -m debugpy.adapter)
- Node.js: `vscode-node-debug2` or inspector protocol
- Go: `delve` (dlv dap)
- Rust: `CodeLLDB` (codelldb --port 0)

---

### 7. Process Manager (`src/process/manager.rs`)

**Purpose**: Spawn and manage debugger adapter processes

**Key Types**:
```rust
pub struct ProcessManager;

impl ProcessManager {
    pub async fn spawn_adapter(
        config: &AdapterConfig,
    ) -> Result<(Child, DapTransport), Error> {
        match &config.adapter_type {
            AdapterType::Executable { command, args } => {
                let mut child = Command::new(command)
                    .args(args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .envs(&config.spawn_config.env)
                    .current_dir(/* ... */)
                    .kill_on_drop(true)  // Important!
                    .spawn()?;

                let stdin = child.stdin.take().unwrap();
                let stdout = child.stdout.take().unwrap();

                let transport = DapTransport::new_stdio(stdin, stdout);

                Ok((child, transport))
            }
            // ... other adapter types
        }
    }
}
```

**Process Monitoring**:
```rust
pub async fn monitor_process(
    mut child: Child,
    session_id: String,
) {
    let status = child.wait().await;

    tracing::warn!(
        "Adapter process for session {} exited: {:?}",
        session_id,
        status
    );

    // Notify session manager to clean up
}
```

---

## Error Handling Strategy

### Error Types (`src/error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Adapter not found for language: {0}")]
    AdapterNotFound(String),

    #[error("Invalid session state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    #[error("Feature not supported: {0}")]
    UnsupportedFeature(String),

    #[error("DAP protocol error: {0}")]
    DapProtocol(String),

    #[error("Process error: {0}")]
    Process(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl Error {
    /// Convert to MCP JSON-RPC error
    pub fn to_json_rpc_error(&self) -> serde_json::Value {
        json!({
            "code": self.error_code(),
            "message": self.to_string(),
            "data": self.error_data(),
        })
    }

    fn error_code(&self) -> i32 {
        match self {
            Error::SessionNotFound(_) => 404,
            Error::InvalidState { .. } => 400,
            Error::UnsupportedFeature(_) => 501,
            _ => 500,
        }
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        // Provide context for AI agent
        match self {
            Error::UnsupportedFeature(feature) => Some(json!({
                "feature": feature,
                "suggestion": "Check adapter capabilities before using this feature"
            })),
            _ => None,
        }
    }
}
```

### Error Propagation Pattern

```rust
// Use ? operator for automatic conversion
pub async fn create_session(&self, ...) -> Result<String, Error> {
    let adapter = self.adapter_registry.get(language)
        .ok_or_else(|| Error::AdapterNotFound(language.to_string()))?;

    let (process, transport) = ProcessManager::spawn_adapter(adapter).await?;

    let dap_client = DapClient::new(transport)?;

    // ...
}
```

---

## Concurrency Patterns

### Pattern 1: Arc + RwLock for Shared State

**Use Case**: Session map (read-heavy)

```rust
type SessionMap = Arc<RwLock<HashMap<String, Arc<RwLock<DebugSession>>>>>;

// Read access (concurrent, no blocking)
async fn get_session(map: &SessionMap, id: &str) -> Option<Arc<RwLock<DebugSession>>> {
    map.read().await.get(id).cloned()
}

// Write access (exclusive)
async fn add_session(map: &SessionMap, id: String, session: DebugSession) {
    map.write().await.insert(id, Arc::new(RwLock::new(session)));
}
```

### Pattern 2: Actor Model for Session Isolation

**Use Case**: Independent session tasks

```rust
async fn session_actor(
    session: Arc<RwLock<DebugSession>>,
    mut event_rx: flume::Receiver<DapEvent>,
) {
    while let Ok(event) = event_rx.recv_async().await {
        let mut session = session.write().await;

        match event {
            DapEvent::Stopped { reason, thread_id, .. } => {
                session.state = SessionState::Paused { reason, thread_id };
                session.current_frame = None; // Invalidate
            }
            DapEvent::Continued { .. } => {
                session.state = SessionState::Running;
            }
            DapEvent::Terminated { .. } => {
                session.state = SessionState::Terminated { exit_code: None };
                break; // Exit actor
            }
            _ => {}
        }
    }
}
```

### Pattern 3: Request/Response Correlation

**Use Case**: DAP request tracking

```rust
// Sender side
let (tx, rx) = oneshot::channel();
let seq = request_id.fetch_add(1, Ordering::SeqCst);
pending_requests.write().await.insert(seq, tx);
transport.send(Request { seq, ... }).await?;
let response = rx.await?;

// Receiver side (event loop)
if msg.type == "response" {
    if let Some(tx) = pending_requests.write().await.remove(&msg.request_seq) {
        tx.send(msg.into_response()).ok();
    }
}
```

---

## Testing Strategy

### Unit Tests

**Example: Session State Machine**

```rust
#[tokio::test]
async fn test_session_state_transitions() {
    let session = DebugSession::new_mock();

    assert_eq!(session.state, SessionState::Initializing);

    session.initialize().await.unwrap();
    assert_eq!(session.state, SessionState::Configuring);

    session.launch(mock_config()).await.unwrap();
    assert_eq!(session.state, SessionState::Running);

    session.handle_event(DapEvent::Stopped { ... });
    assert!(matches!(session.state, SessionState::Paused { .. }));
}
```

### Integration Tests

**Example: End-to-End Python Debugging**

```rust
#[tokio::test]
async fn test_python_debugging_e2e() {
    let server = McpServer::new();

    // Start debugger
    let session_id = server.call_tool("debugger_start", json!({
        "mode": "launch",
        "language": "python",
        "program": "test_fixtures/simple.py"
    })).await.unwrap();

    // Set breakpoint
    server.call_tool("debugger_set_breakpoint", json!({
        "sessionId": session_id,
        "source": "test_fixtures/simple.py",
        "line": 5
    })).await.unwrap();

    // Continue
    server.call_tool("debugger_continue", json!({
        "sessionId": session_id
    })).await.unwrap();

    // Wait for breakpoint hit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check state
    let resource = server.get_resource(&format!("debugger://sessions/{}", session_id))
        .await.unwrap();
    assert_eq!(resource.content.state, "paused");

    // Evaluate
    let result = server.call_tool("debugger_evaluate", json!({
        "sessionId": session_id,
        "expression": "x"
    })).await.unwrap();
    assert_eq!(result.value, "42");
}
```

---

## Performance Considerations

### Latency Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| `debugger_start` | < 500ms | Includes adapter spawn |
| `debugger_set_breakpoint` | < 50ms | Simple breakpoint |
| `debugger_continue` | < 20ms | Just send command |
| `debugger_evaluate` | < 100ms | Depends on expression |
| `get_resource` | < 10ms | Read from cache |

### Optimization Techniques

1. **Connection Pooling**: Reuse adapter processes for multiple sessions
2. **Lazy Loading**: Don't fetch variables until requested
3. **Caching**: Cache stack traces, variables (invalidate on state change)
4. **Pagination**: Limit stack frames/variables per request
5. **Batch Requests**: Support batching multiple breakpoints

---

## Security Considerations

### Input Validation

```rust
fn validate_file_path(path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);

    // Prevent directory traversal
    if path.components().any(|c| c == Component::ParentDir) {
        return Err(Error::InvalidPath("Path traversal not allowed"));
    }

    // Must be absolute
    if !path.is_absolute() {
        return Err(Error::InvalidPath("Path must be absolute"));
    }

    Ok(path)
}
```

### Resource Limits

```rust
const MAX_SESSIONS: usize = 100;
const MAX_BREAKPOINTS_PER_SESSION: usize = 1000;
const MAX_EXPRESSION_LENGTH: usize = 10_000;

impl SessionManager {
    pub async fn create_session(&self, ...) -> Result<String, Error> {
        let count = self.sessions.read().await.len();
        if count >= MAX_SESSIONS {
            return Err(Error::ResourceLimit("Max sessions reached"));
        }
        // ...
    }
}
```

### Timeouts

```rust
use tokio::time::timeout;

async fn send_request_with_timeout<R: DapRequest>(
    &self,
    request: R,
) -> Result<R::Response, Error> {
    timeout(
        Duration::from_secs(30),
        self.send_request(request)
    ).await
    .map_err(|_| Error::Timeout("DAP request timed out"))?
}
```

---

**End of Component Specifications**
