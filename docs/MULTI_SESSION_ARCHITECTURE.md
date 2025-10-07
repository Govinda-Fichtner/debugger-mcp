# Multi-Session Architecture for Node.js Debugging

## Date: 2025-10-07

## Overview

This document describes the multi-session architecture implementation for Node.js debugging with vscode-js-debug. This architecture enables proper handling of parent-child session relationships required by vscode-js-debug.

## Background

### The Problem

vscode-js-debug uses a **multi-session architecture**:
- **Parent session**: Coordinates debugging, spawns processes
- **Child sessions**: Perform actual debugging (breakpoints, stepping, evaluation)

When you launch a Node.js program:
1. Client connects to parent session (dapDebugServer.js on port X)
2. Parent sends `startDebugging` reverse request with child port
3. Client must connect to child session (pwa-node on port Y)
4. Debugging operations must be routed to child session

### Current Limitation

Our current implementation only supports single sessions:
- Connects to parent session
- Ignores reverse requests for child sessions
- Breakpoints sent to parent (not verified)
- No stopped events received (child sends them, not parent)

### The Solution

Implement full multi-session support:
- Track parent-child session relationships
- Connect to child sessions dynamically
- Route operations to correct session
- Forward events from all sessions

## Architecture

### Component Structure

```
┌─────────────────────────────────────────────────────────┐
│                    DebugSession                         │
│  - id: String                                           │
│  - session_type: SessionType (Single/MultiSession)      │
│  - primary_client: Arc<RwLock<DapClient>>              │
│  - child_sessions: Arc<RwLock<Vec<ChildSession>>>      │
└─────────────────────────────────────────────────────────┘
                          │
                          │ manages
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  MultiSessionManager                    │
│  - parent_session_id: String                           │
│  - children: HashMap<String, ChildSession>             │
│  - active_child: Option<String>                        │
└─────────────────────────────────────────────────────────┘
                          │
                          │ contains
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    ChildSession                         │
│  - id: String                                           │
│  - client: Arc<RwLock<DapClient>>                      │
│  - port: u16                                            │
│  - session_type: String (e.g., "pwa-node")             │
└─────────────────────────────────────────────────────────┘
```

### Session Types

```rust
pub enum SessionType {
    /// Single session (Python, Ruby)
    Single,
    /// Multi-session parent (Node.js vscode-js-debug)
    MultiSessionParent {
        child_sessions: Arc<RwLock<Vec<ChildSession>>>
    },
}
```

### Child Session Lifecycle

```
1. Parent session launches
   └─> initialize() + launch() on parent

2. Parent sends reverse request
   └─> "startDebugging" with child port in arguments

3. Client spawns child session
   └─> Connect to child port
   └─> initialize() on child
   └─> Add to child_sessions list

4. Operations routed to child
   └─> set_breakpoints() → child
   └─> continue() → child
   └─> evaluate() → child

5. Events forwarded from child
   └─> stopped → update DebugSession state
   └─> continued → update DebugSession state
```

## Implementation

### Phase 1: Add Multi-Session Types

**File**: `src/debug/multi_session.rs` (new)

```rust
use crate::{Result, Error};
use crate::dap::client::DapClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Child session in a multi-session architecture
pub struct ChildSession {
    pub id: String,
    pub client: Arc<RwLock<DapClient>>,
    pub port: u16,
    pub session_type: String, // "pwa-node", "chrome", etc.
}

/// Manager for parent-child session relationships
pub struct MultiSessionManager {
    parent_session_id: String,
    children: Arc<RwLock<HashMap<String, ChildSession>>>,
    active_child: Arc<RwLock<Option<String>>>,
}

impl MultiSessionManager {
    pub fn new(parent_session_id: String) -> Self {
        Self {
            parent_session_id,
            children: Arc::new(RwLock::new(HashMap::new())),
            active_child: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a child session
    pub async fn add_child(&self, child: ChildSession) {
        let child_id = child.id.clone();
        self.children.write().await.insert(child_id.clone(), child);

        // Set as active if first child
        let mut active = self.active_child.write().await;
        if active.is_none() {
            *active = Some(child_id);
        }
    }

    /// Get active child session
    pub async fn get_active_child(&self) -> Option<Arc<RwLock<DapClient>>> {
        let active_id = self.active_child.read().await;
        if let Some(id) = active_id.as_ref() {
            let children = self.children.read().await;
            children.get(id).map(|child| child.client.clone())
        } else {
            None
        }
    }

    /// Get all child sessions
    pub async fn get_children(&self) -> Vec<String> {
        self.children.read().await.keys().cloned().collect()
    }
}
```

### Phase 2: Update DebugSession for Multi-Session

**File**: `src/debug/session.rs`

```rust
pub enum SessionMode {
    Single {
        client: Arc<RwLock<DapClient>>,
    },
    MultiSession {
        parent_client: Arc<RwLock<DapClient>>,
        multi_session_manager: MultiSessionManager,
    },
}

pub struct DebugSession {
    pub id: String,
    pub language: String,
    pub program: String,
    session_mode: SessionMode, // Replaces single client field
    pub(crate) state: Arc<RwLock<SessionState>>,
    pending_breakpoints: Arc<RwLock<HashMap<String, Vec<SourceBreakpoint>>>>,
}

impl DebugSession {
    /// Get the client to use for debugging operations
    /// For multi-session, returns active child; for single, returns the client
    async fn get_debug_client(&self) -> Arc<RwLock<DapClient>> {
        match &self.session_mode {
            SessionMode::Single { client } => client.clone(),
            SessionMode::MultiSession { multi_session_manager, parent_client, .. } => {
                // Try to get active child, fall back to parent
                multi_session_manager.get_active_child().await
                    .unwrap_or_else(|| parent_client.clone())
            }
        }
    }
}
```

### Phase 3: Reverse Request Handler Enhancement

**File**: `src/dap/client.rs`

Update reverse request handler to extract child session info:

```rust
// In message_reader, when handling Message::Request:
Message::Request(req) => {
    info!("🔄 REVERSE REQUEST received: '{}' (seq {})", req.command, req.seq);

    // Handle startDebugging reverse request
    if req.command == "startDebugging" {
        if let Some(args) = &req.arguments {
            // Extract child session info
            if let Some(config) = args.get("configuration") {
                if let Some(port_str) = config.get("__jsDebugChildServer") {
                    if let Some(port) = port_str.as_str().and_then(|s| s.parse::<u16>().ok()) {
                        info!("🎯 Child session requested on port: {}", port);

                        // Trigger child session spawn via callback
                        if let Some(child_spawn_callback) = &*child_spawn_callback.lock().await {
                            tokio::spawn(child_spawn_callback(port));
                        }
                    }
                }
            }
        }
    }

    // Send success response
    let response = Response {
        seq: 0,
        request_seq: req.seq,
        success: true,
        command: req.command.clone(),
        message: None,
        body: None,
    };

    // ... send response
}
```

### Phase 4: Child Session Spawning

**File**: `src/debug/session.rs`

```rust
impl DebugSession {
    /// Spawn child session for multi-session debugging (Node.js)
    async fn spawn_child_session(&self, port: u16) -> Result<()> {
        info!("🔄 Spawning child session on port {}", port);

        // Connect to child port
        let socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .map_err(|e| Error::Process(format!("Failed to connect to child session port {}: {}", port, e)))?;

        info!("✅ Connected to child session on port {}", port);

        // Create DAP client for child
        let child_client = DapClient::from_socket(socket).await?;

        // Initialize child session
        child_client.initialize(&format!("nodejs-child-{}", port)).await?;

        info!("✅ Child session initialized on port {}", port);

        // Register event handlers for child (forward to parent state)
        let session_state = self.state.clone();
        child_client.on_event("stopped", move |event| {
            info!("📍 [CHILD] Received 'stopped' event: {:?}", event);
            // Update parent session state
            let state_clone = session_state.clone();
            tokio::spawn(async move {
                if let Some(body) = &event.body {
                    let thread_id = body.get("threadId").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(1);
                    let reason = body.get("reason").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let mut state = state_clone.write().await;
                    state.set_state(DebugState::Stopped { thread_id, reason });
                }
            });
        }).await;

        // Add to multi-session manager
        if let SessionMode::MultiSession { multi_session_manager, .. } = &self.session_mode {
            let child = ChildSession {
                id: format!("child-{}", port),
                client: Arc::new(RwLock::new(child_client)),
                port,
                session_type: "pwa-node".to_string(),
            };
            multi_session_manager.add_child(child).await;
        }

        Ok(())
    }
}
```

### Phase 5: Operation Routing

All debugging operations use `get_debug_client()`:

```rust
impl DebugSession {
    pub async fn set_breakpoint(&self, source_path: String, line: i32) -> Result<bool> {
        // ... existing state checks ...

        // Use appropriate client (child for multi-session, direct for single)
        let client = self.get_debug_client().await;
        let client = client.read().await;

        // ... existing breakpoint logic ...
    }

    pub async fn continue_execution(&self) -> Result<()> {
        let client = self.get_debug_client().await;
        let client = client.read().await;
        client.continue_execution(thread_id).await
    }

    // Same pattern for step_over, step_into, evaluate, etc.
}
```

### Phase 6: Session Creation for Node.js

**File**: `src/debug/manager.rs`

```rust
"nodejs" => {
    // ... existing spawn code ...

    // Create session with multi-session mode
    let parent_client = DapClient::from_socket(nodejs_session.socket).await?;
    let multi_session_manager = MultiSessionManager::new(session_id.clone());

    let session_mode = SessionMode::MultiSession {
        parent_client: Arc::new(RwLock::new(parent_client)),
        multi_session_manager: multi_session_manager.clone(),
    };

    let session = DebugSession::new_with_mode(
        language.to_string(),
        program.clone(),
        session_mode,
    ).await?;

    // Register callback for child session spawning
    let session_arc = Arc::new(session);
    let session_clone = session_arc.clone();
    parent_client.on_reverse_request("startDebugging", move |req| {
        let session = session_clone.clone();
        async move {
            // Extract port and spawn child
            if let Some(port) = extract_child_port(req) {
                session.spawn_child_session(port).await?;
            }
            Ok(())
        }
    }).await;

    // ... rest of initialization ...
}
```

## Benefits

1. **Proper Node.js Debugging**
   - Breakpoints verified and hit correctly
   - Stopped events received from child sessions
   - All debugging operations work as expected

2. **Clean Abstraction**
   - Python/Ruby use Single mode (unchanged)
   - Node.js uses MultiSession mode
   - Operations transparently routed to correct client

3. **Extensibility**
   - Support for multiple child sessions (future)
   - Can handle Chrome debugging, Electron, etc.
   - Pattern reusable for other multi-session adapters

4. **State Consistency**
   - Events from child sessions update parent session state
   - Single source of truth for session state
   - No race conditions

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_multi_session_manager_add_child() {
    let manager = MultiSessionManager::new("parent".to_string());
    let child = create_mock_child_session();
    manager.add_child(child).await;

    assert_eq!(manager.get_children().await.len(), 1);
    assert!(manager.get_active_child().await.is_some());
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore]
async fn test_nodejs_multi_session_breakpoint() {
    // Start Node.js debugging
    let session_id = start_nodejs_session(...).await?;

    // Wait for child session to spawn
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Set breakpoint (should go to child)
    let verified = set_breakpoint(&session_id, "fizzbuzz.js", 5).await?;
    assert!(verified); // Child verifies it

    // Continue (on child)
    continue_execution(&session_id).await?;

    // Wait for stopped (from child)
    wait_for_stopped(&session_id).await?;
}
```

## Migration Path

### Phase 1: Core Infrastructure (2-3 hours)
- Create `multi_session.rs` module
- Add `SessionMode` enum
- Update `DebugSession` structure

### Phase 2: Reverse Request Handling (1-2 hours)
- Enhance `DapClient` reverse request handler
- Extract child port from `startDebugging`
- Add callback mechanism

### Phase 3: Child Session Lifecycle (2-3 hours)
- Implement `spawn_child_session()`
- Connect to child port
- Initialize child session
- Register event handlers

### Phase 4: Operation Routing (1-2 hours)
- Update all debugging methods to use `get_debug_client()`
- Test routing logic

### Phase 5: Integration & Testing (2-3 hours)
- Update session manager
- Create integration tests
- Verify FizzBuzz workflow

**Total Estimated Time**: 8-13 hours

## Success Criteria

- [ ] Multi-session manager tracks parent-child relationships
- [ ] Child sessions spawn automatically from reverse requests
- [ ] Breakpoints set on child sessions are verified
- [ ] Stopped events received from child sessions
- [ ] All debugging operations work with Node.js
- [ ] Python and Ruby unaffected (backward compatible)
- [ ] Integration tests pass
- [ ] Logging shows parent/child context clearly

## Risks & Mitigations

**Risk 1**: Child port extraction fails
- **Mitigation**: Comprehensive logging, fallback to entry breakpoint workaround

**Risk 2**: Child session connection timeout
- **Mitigation**: Use aggressive 2s timeout with retry

**Risk 3**: Event forwarding causes state inconsistency
- **Mitigation**: Careful state machine design, comprehensive tests

**Risk 4**: Breaking Python/Ruby functionality
- **Mitigation**: SessionMode enum allows different code paths, extensive regression testing

## Future Enhancements

1. **Multiple Concurrent Children**
   - Track multiple child sessions
   - Switch active child dynamically

2. **Child Session Health Monitoring**
   - Detect when child disconnects
   - Spawn replacement if needed

3. **Advanced Routing**
   - Route operations based on file path
   - Support debugging multiple programs simultaneously

4. **Performance Optimization**
   - Connection pooling for child sessions
   - Lazy child spawning (only when needed)

## References

- vscode-js-debug source: https://github.com/microsoft/vscode-js-debug
- nvim-dap-vscode-js: https://github.com/mxsdev/nvim-dap-vscode-js
- DAP Specification (Reverse Requests): https://microsoft.github.io/debug-adapter-protocol/specification#reverse-requests
- Previous analysis: `docs/NODEJS_SESSION_SUMMARY.md`

---

**Status**: Design Complete - Ready for Implementation
**Priority**: HIGH - Enables proper Node.js debugging
**Estimated Effort**: 8-13 hours
**Dependencies**: None (builds on current architecture)
