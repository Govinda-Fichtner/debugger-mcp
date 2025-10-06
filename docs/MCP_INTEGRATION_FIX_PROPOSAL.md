# MCP Integration Fix Proposal

**Date**: 2025-10-06
**Status**: Proposal for Review
**Priority**: HIGH - Blocks real-world usage

## Problem Summary

The debugger MCP works perfectly in integration tests (2.5s, all features functional) but fails when used through Claude Code's MCP interface:

1. **DAP initialization hangs** after "Spawning DAP client"
2. **No session ID returned** to MCP clients
3. **Race condition** with short-running programs completing before breakpoints set
4. **Events not exposed** - MCP clients can't receive DAP events like 'stopped'
5. **Poor error messages** - hard to distinguish "program exited" from "adapter crashed"

## Root Causes

### 1. Event Loop Blocking
The `initialize_and_launch` method waits for 'initialized' event with a 5-second timeout, but the event pump may not be running properly during initialization.

### 2. Synchronous Initialization
Current flow:
```
spawn adapter → initialize → launch → wait for 'initialized' → configurationDone
```
This is synchronous and blocking. If any step fails or times out, the entire operation hangs.

### 3. Program Completion Race
Short scripts like fizzbuzz.py run to completion in milliseconds. Even with `stopOnEntry: true`, there's a race between:
- DAP adapter starting and pausing the program
- Program running to completion
- Client setting breakpoints

### 4. No Event Exposure
The MCP protocol wraps DAP but doesn't expose DAP events. Clients can't:
- Wait for 'stopped' event before setting breakpoints
- Know when program has exited
- Receive thread/process events

## Proposed Solutions

### Priority 1: Async Initialization with Early Return ✅ RECOMMENDED

**Goal**: Return session ID immediately, complete initialization in background

**Changes**:

1. **Split initialization** into phases:
   - Phase 1 (synchronous): Spawn adapter, send initialize
   - Phase 2 (async): Wait for events, send launch
   - Phase 3 (async): Wait for stopped/running state

2. **Return session ID early**:
```rust
pub async fn create_session(&mut self, ...) -> Result<String> {
    let session_id = Uuid::new_v4().to_string();

    // Phase 1: Spawn and initialize (fast, ~100ms)
    let client = DapClient::new(...);
    client.initialize().await?;

    // Store session immediately
    self.sessions.insert(session_id.clone(), session);

    // Phase 2 & 3: Launch and wait (slow, in background)
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        if let Err(e) = client.launch_and_wait().await {
            error!("Session {} initialization failed: {}", session_id_clone, e);
            // Mark session as failed, but don't remove
        }
    });

    Ok(session_id)
}
```

3. **Add session state tracking**:
```rust
pub enum SessionState {
    Initializing,      // Adapter spawned, waiting for 'initialized'
    Ready,             // Can set breakpoints and control execution
    Running,           // Program executing
    Stopped,           // Hit breakpoint or stopOnEntry
    Exited,            // Program completed
    Failed(String),    // Initialization or execution error
}
```

4. **Add state query tool**:
```json
{
  "name": "debugger_session_state",
  "arguments": {
    "sessionId": "..."
  },
  "returns": {
    "state": "Ready|Running|Stopped|Exited|Failed",
    "reason": "entry|breakpoint|step|exit|..."
  }
}
```

**Benefits**:
- ✅ Immediate session ID return (no timeout)
- ✅ Non-blocking initialization
- ✅ Clear error reporting
- ✅ Can poll for ready state

**Testing**:
```bash
# Should return session ID in <200ms
debugger_start → session_id

# Poll until ready
debugger_session_state → "Initializing"
debugger_session_state → "Ready"

# Now safe to set breakpoints
debugger_set_breakpoint → success
```

---

### Priority 2: Event Exposure through MCP 🔄 COMPLEMENTARY

**Goal**: Allow MCP clients to receive DAP events

**Changes**:

1. **Add event subscription tool**:
```json
{
  "name": "debugger_subscribe_events",
  "arguments": {
    "sessionId": "...",
    "events": ["stopped", "continued", "thread", "process", "exited"]
  },
  "returns": {
    "subscriptionId": "..."
  }
}
```

2. **Add event polling tool**:
```json
{
  "name": "debugger_poll_events",
  "arguments": {
    "sessionId": "...",
    "timeout": 5000  // milliseconds
  },
  "returns": {
    "events": [
      {"event": "stopped", "reason": "entry", "threadId": 1},
      {"event": "continued", "threadId": 1}
    ]
  }
}
```

3. **Store events in session**:
```rust
pub struct Session {
    client: DapClient,
    state: SessionState,
    event_queue: Arc<Mutex<VecDeque<DapEvent>>>,
    max_event_queue_size: usize,
}
```

4. **DAP event callback** pushes to queue:
```rust
client.on_event("*", move |event| {
    let mut queue = event_queue.lock().unwrap();
    queue.push_back(event);
    if queue.len() > max_event_queue_size {
        queue.pop_front();
    }
});
```

**Benefits**:
- ✅ Clients can wait for 'stopped' before setting breakpoints
- ✅ Know when program exits
- ✅ React to thread/process events
- ✅ No polling needed for state

**Usage**:
```javascript
// Subscribe to stopped events
debugger_subscribe_events(sessionId, ["stopped"])

// Start debugging
debugger_start(program, stopOnEntry=true) → sessionId

// Wait for stopped event
events = debugger_poll_events(sessionId, timeout=5000)
// events = [{"event": "stopped", "reason": "entry"}]

// Now safe to set breakpoints
debugger_set_breakpoint(sessionId, line=21)

// Continue execution
debugger_continue(sessionId)

// Wait for breakpoint hit
events = debugger_poll_events(sessionId, timeout=10000)
// events = [{"event": "stopped", "reason": "breakpoint"}]
```

---

### Priority 3: Pre-Launch Breakpoint Support 🔧 HIGH IMPACT

**Goal**: Set breakpoints before program starts

**Changes**:

1. **Store pending breakpoints** in session:
```rust
pub struct Session {
    client: DapClient,
    pending_breakpoints: HashMap<String, Vec<SourceBreakpoint>>,
}
```

2. **Modify debugger_set_breakpoint** to accept unlaunched sessions:
```rust
pub async fn set_breakpoint(&mut self, session_id: &str, ...) -> Result<()> {
    let session = self.get_session(session_id)?;

    match session.state {
        SessionState::Initializing => {
            // Store for later
            session.pending_breakpoints
                .entry(source_path.clone())
                .or_insert_vec![])
                .push(SourceBreakpoint { line, condition });
            Ok(())
        }
        SessionState::Ready | SessionState::Stopped => {
            // Send immediately
            session.client.set_breakpoints(...).await
        }
        _ => Err("Invalid state for setting breakpoints")
    }
}
```

3. **Apply pending breakpoints** after launch:
```rust
async fn launch_and_wait(&mut self) -> Result<()> {
    self.client.launch(...).await?;
    self.client.wait_for_event("initialized", timeout).await?;

    // Apply pending breakpoints
    for (source_path, breakpoints) in &self.pending_breakpoints {
        let source = Source::from_path(source_path);
        self.client.set_breakpoints(source, breakpoints.clone()).await?;
    }

    self.client.configuration_done().await?;
}
```

**Benefits**:
- ✅ No race condition - breakpoints set before program runs
- ✅ Works with stopOnEntry
- ✅ Simpler client code
- ✅ Reliable for short-running programs

**Usage**:
```javascript
// Start session
debugger_start(program, stopOnEntry=true) → sessionId

// Set breakpoints immediately (stored as pending)
debugger_set_breakpoint(sessionId, source, line=21) → OK

// Program launches with breakpoints already set
debugger_session_state(sessionId) → "Stopped" (at entry)

// Continue
debugger_continue(sessionId)

// Will hit breakpoint that was set before launch
```

---

### Priority 4: Improved Error Handling 📊 QUALITY OF LIFE

**Goal**: Better diagnostics and error messages

**Changes**:

1. **Structured error types**:
```rust
pub enum DebuggerError {
    AdapterSpawnFailed(String),
    InitializationTimeout,
    ProgramExited,
    AdapterCrashed(i32),
    BreakpointFailed { reason: String, line: u32 },
    InvalidState { expected: Vec<SessionState>, actual: SessionState },
}
```

2. **Detailed error messages**:
```rust
return Err(DebuggerError::InitializationTimeout.with_context(
    "DAP adapter did not send 'initialized' event within 5 seconds. \
     This usually means: \n\
     1. The adapter crashed (check stderr)\n\
     2. The program exited immediately\n\
     3. The adapter is incompatible\n\
     Try: Check program runs normally outside debugger"
));
```

3. **Capture adapter stderr**:
```rust
let mut child = Command::new(&adapter_path)
    .stderr(Stdio::piped())
    .spawn()?;

let stderr = child.stderr.take().unwrap();
let stderr_reader = BufReader::new(stderr);

// Stream stderr to logs
tokio::spawn(async move {
    for line in stderr_reader.lines() {
        error!("Adapter stderr: {}", line);
    }
});
```

4. **Session diagnostics tool**:
```json
{
  "name": "debugger_session_diagnostics",
  "arguments": {
    "sessionId": "..."
  },
  "returns": {
    "state": "Failed",
    "error": "InitializationTimeout",
    "context": "DAP adapter did not send 'initialized' event...",
    "adapterStderr": ["...", "..."],
    "lastEvents": ["initialized", "stopped"],
    "pendingRequests": [{"seq": 5, "command": "setBreakpoints", "age": "2.3s"}]
  }
}
```

**Benefits**:
- ✅ Clear error messages guide users
- ✅ Easy to distinguish different failure modes
- ✅ Debugging MCP issues easier
- ✅ Better user experience

---

### Priority 5: Attach Mode Support 🔌 FUTURE ENHANCEMENT

**Goal**: Attach to already-running programs

**Changes**:

1. **Add attach configuration**:
```rust
pub async fn attach(&mut self, pid: u32, ...) -> Result<String> {
    // Similar to create_session but uses 'attach' request instead of 'launch'
}
```

2. **Add tool**:
```json
{
  "name": "debugger_attach",
  "arguments": {
    "language": "python",
    "processId": 12345
  },
  "returns": {
    "sessionId": "..."
  }
}
```

**Benefits**:
- ✅ No race condition for short programs
- ✅ Can debug already-running services
- ✅ Useful for long-running processes

**Note**: This is lower priority, as it requires programs to be started with debug support enabled.

---

## Recommended Implementation Order

### Phase 1: Immediate Fixes (1-2 hours) 🚨
1. **Async initialization with early return** (Priority 1)
2. **Session state tracking** (Priority 1)
3. **State query tool** (Priority 1)

This alone will fix the "hanging" issue and make the MCP tool usable.

### Phase 2: Core Functionality (2-3 hours) 🔧
4. **Pre-launch breakpoint support** (Priority 3)
5. **Improved error handling** (Priority 4)

This will eliminate race conditions and improve reliability.

### Phase 3: Advanced Features (3-4 hours) 🔄
6. **Event exposure** (Priority 2)
7. **Event polling** (Priority 2)

This enables reactive workflows and eliminates polling.

### Phase 4: Future (Optional) 🔌
8. **Attach mode** (Priority 5)

Only if there's demand for attaching to running processes.

---

## Testing Strategy

### Integration Test Updates

1. **Test async initialization**:
```rust
#[tokio::test]
async fn test_async_initialization() {
    let session_id = manager.create_session(...).await?;
    // Should return immediately, not wait 5 seconds

    // Poll for ready state
    loop {
        let state = manager.get_session_state(&session_id).await?;
        if state == SessionState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Now can set breakpoints
    manager.set_breakpoint(&session_id, ...).await?;
}
```

2. **Test pre-launch breakpoints**:
```rust
#[tokio::test]
async fn test_pre_launch_breakpoints() {
    let session_id = manager.create_session(...).await?;

    // Set breakpoint before launch completes
    manager.set_breakpoint(&session_id, source, 21).await?;

    // Wait for ready
    manager.wait_for_state(&session_id, SessionState::Stopped).await?;

    // Continue - should hit breakpoint
    manager.continue_execution(&session_id).await?;

    // Verify stopped at breakpoint
    let state = manager.get_session_state(&session_id).await?;
    assert_eq!(state.reason, Some("breakpoint"));
}
```

3. **Test event polling**:
```rust
#[tokio::test]
async fn test_event_polling() {
    manager.subscribe_events(&session_id, vec!["stopped"]).await?;

    manager.continue_execution(&session_id).await?;

    let events = manager.poll_events(&session_id, 5000).await?;
    assert_eq!(events[0].event, "stopped");
    assert_eq!(events[0].reason, Some("breakpoint"));
}
```

### Real-World Test

Create a test script that mimics Claude Code's MCP usage:

```rust
// tests/real_world_mcp_test.rs
#[tokio::test]
async fn test_mcp_through_stdio() {
    // Spawn MCP server as subprocess
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "debugger_mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.as_mut().unwrap();
    let stdout = BufReader::new(child.stdout.as_mut().unwrap());

    // Send JSON-RPC initialize
    send_jsonrpc(stdin, "initialize", ...)?;
    let response = read_jsonrpc(stdout)?;
    assert!(response.is_success());

    // Start debugging
    send_jsonrpc(stdin, "tools/call", {
        "name": "debugger_start",
        "arguments": {
            "language": "python",
            "program": "/tmp/fizzbuzz.py",
            "stopOnEntry": true
        }
    })?;

    let response = read_jsonrpc(stdout)?;
    let session_id = response.result["sessionId"];

    // Should return in < 500ms
    assert!(response.duration_ms < 500);

    // Poll for ready state
    loop {
        send_jsonrpc(stdin, "tools/call", {
            "name": "debugger_session_state",
            "arguments": {"sessionId": session_id}
        })?;
        let response = read_jsonrpc(stdout)?;

        if response.result["state"] == "Ready" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Set breakpoint
    send_jsonrpc(stdin, "tools/call", {
        "name": "debugger_set_breakpoint",
        "arguments": {
            "sessionId": session_id,
            "sourcePath": "/tmp/fizzbuzz.py",
            "line": 21
        }
    })?;

    let response = read_jsonrpc(stdout)?;
    assert!(response.is_success());
}
```

---

## Success Criteria

### Phase 1 Success:
- ✅ `debugger_start` returns session ID in < 500ms
- ✅ No hanging or timeout issues
- ✅ Can query session state
- ✅ Clear error messages if initialization fails

### Phase 2 Success:
- ✅ Can set breakpoints before program launches
- ✅ No race conditions with short-running programs
- ✅ Breakpoints always hit correctly
- ✅ Error messages distinguish failure types

### Phase 3 Success:
- ✅ Can poll for DAP events
- ✅ Know when program stops/continues/exits
- ✅ Reactive workflows possible
- ✅ No need for manual state polling

### Overall Success:
- ✅ Real-world test passes (MCP through STDIO)
- ✅ Claude Code integration works
- ✅ fizzbuzz.py can be debugged externally
- ✅ All existing integration tests still pass

---

## Risk Assessment

### Low Risk:
- Session state tracking
- State query tool
- Error message improvements
- Pre-launch breakpoint storage

### Medium Risk:
- Async initialization (changes core flow)
- Event queue implementation (memory management)

### High Risk:
- Event exposure (new protocol surface)
- Attach mode (requires debugpy-specific setup)

---

## Estimated Timeline

- **Phase 1**: 1-2 hours implementation + 1 hour testing = **3 hours**
- **Phase 2**: 2-3 hours implementation + 1 hour testing = **4 hours**
- **Phase 3**: 3-4 hours implementation + 2 hours testing = **6 hours**
- **Total**: 13 hours for complete solution

**Recommended**: Start with Phase 1 (3 hours), validate with real-world test, then proceed to Phase 2.

---

## Alternative Approaches Considered

### Alt 1: Synchronous Initialization with Longer Timeout
**Rejected**: Doesn't solve race condition, just masks it

### Alt 2: Launch Without stopOnEntry, Set Breakpoints, Then Continue
**Rejected**: Race condition still exists, breakpoints might miss

### Alt 3: Require Programs to Have Infinite Loop
**Rejected**: User-hostile, not practical

### Alt 4: Use debugpy's Attach Mode Only
**Rejected**: Requires users to start programs specially

---

## Questions for Confirmation

1. **Prioritization**: Do you agree with the Phase 1 → 2 → 3 order?
2. **Testing**: Should I implement real-world STDIO test first to validate?
3. **Breaking Changes**: Phase 1 changes `create_session` signature - acceptable?
4. **Event Exposure**: Is event polling sufficient, or do you need streaming/push?
5. **Timeline**: Is 3-hour Phase 1 acceptable, or need faster?

---

## Conclusion

The recommended approach is:

1. **Implement async initialization** (Priority 1) - solves hanging
2. **Add session state tracking** (Priority 1) - enables polling
3. **Add pre-launch breakpoints** (Priority 3) - eliminates race condition
4. **Improve error messages** (Priority 4) - better UX
5. **Add event exposure** (Priority 2) - enables reactive workflows

This provides a robust, production-ready solution that works through MCP protocol while maintaining backward compatibility with existing integration tests.

**Next Step**: Confirm approach and begin Phase 1 implementation.
