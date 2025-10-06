# Critical Bug Fix: Session State Not Updating on DAP Events

## Root Cause Analysis

The MCP debugger has a **critical bug** where `debugger_session_state` always returns `"Running"` even when execution is stopped at a breakpoint.

### Why This Happens

1. **State enum exists** - `DebugState::Stopped { thread_id, reason }` is defined in `src/debug/state.rs:11`
2. **State is manually set** - Only transitions to `Running` after initialization (session.rs:88)
3. **No DAP event handlers** - **Missing**: No code listens for DAP 'stopped' events
4. **Result**: State never transitions from `Running` to `Stopped`

### Evidence from User Report

```
debugger_continue(sessionId="...")
→ Response: {"status": "continued"}

debugger_session_state(sessionId="...")
→ Response: {"state": "Running", ...}  ← WRONG! Should be "Stopped"

debugger_stack_trace(sessionId="...")  ← User tried this "out of frustration"
→ Response: {"stackFrames": [{"line": 20, ...}]}  ← Proves debugger WAS stopped!
```

The debugger was correctly stopped at the breakpoint, but `debugger_session_state()` didn't reflect this.

## The Fix

We need to register DAP event handlers during session initialization to update the session state when DAP sends events.

### Files to Modify

1. **src/debug/session.rs** - Add event handlers during initialization
2. **src/dap/client.rs** - Already has event infrastructure via `on_event()`
3. **src/debug/state.rs** - Already has `Stopped` state, no changes needed

### Implementation

#### Step 1: Register 'stopped' Event Handler in Session

```rust
// In src/debug/session.rs, in initialize_and_launch() method

pub async fn initialize_and_launch(&self, adapter_id: &str, launch_args: serde_json::Value) -> Result<()> {
    {
        let mut state = self.state.write().await;
        state.set_state(DebugState::Initializing);
    }

    let client = self.client.read().await;

    // ✨ NEW: Register event handlers BEFORE launch
    let session_state = self.state.clone();
    client.on_event("stopped", move |event| {
        info!("📍 Received 'stopped' event: {:?}", event);

        // Parse stopped event body
        if let Some(body) = &event.body {
            let thread_id = body.get("threadId")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(1);

            let reason = body.get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            info!("   Thread: {}, Reason: {}", thread_id, reason);

            // Update session state
            let state_clone = session_state.clone();
            tokio::spawn(async move {
                let mut state = state_clone.write().await;
                state.set_state(DebugState::Stopped { thread_id, reason });
                info!("✅ Session state updated to Stopped");
            });
        }
    }).await;

    // Register 'continued' event handler
    let session_state = self.state.clone();
    client.on_event("continued", move |event| {
        info!("▶️  Received 'continued' event: {:?}", event);

        let state_clone = session_state.clone();
        tokio::spawn(async move {
            let mut state = state_clone.write().await;
            state.set_state(DebugState::Running);
            info!("✅ Session state updated to Running");
        });
    }).await;

    // Register 'terminated' event handler
    let session_state = self.state.clone();
    client.on_event("terminated", move |event| {
        info!("🛑 Received 'terminated' event: {:?}", event);

        let state_clone = session_state.clone();
        tokio::spawn(async move {
            let mut state = state_clone.write().await;
            state.set_state(DebugState::Terminated);
            info!("✅ Session state updated to Terminated");
        });
    }).await;

    // Register 'exited' event handler
    let session_state = self.state.clone();
    client.on_event("exited", move |event| {
        info!("🚪 Received 'exited' event: {:?}", event);

        let state_clone = session_state.clone();
        tokio::spawn(async move {
            let mut state = state_clone.write().await;
            state.set_state(DebugState::Terminated);
            info!("✅ Session state updated to Terminated (exited)");
        });
    }).await;

    // Now proceed with initialization
    client.initialize_and_launch(adapter_id, launch_args).await?;

    // Rest of the method...
}
```

#### Step 2: Test the Fix

After implementing the fix, the user's workflow should work:

```javascript
// Start session
debugger_start({
  "program": "/workspace/fizzbuzz.py",
  "stopOnEntry": true
})
→ {"sessionId": "...", "status": "started"}

// Poll state (should show Stopped at entry)
debugger_session_state({"sessionId": "..."})
→ {"state": "Stopped", "details": {"reason": "entry", "threadId": 1}}  ✅

// Set breakpoint
debugger_set_breakpoint({
  "sessionId": "...",
  "sourcePath": "/workspace/fizzbuzz.py",
  "line": 20
})
→ {"verified": true, ...}

// Continue execution
debugger_continue({"sessionId": "..."})
→ {"status": "continued"}

// Poll state (should show Stopped at breakpoint)
debugger_session_state({"sessionId": "..."})
→ {"state": "Stopped", "details": {"reason": "breakpoint", "threadId": 1}}  ✅
```

## Additional Improvements

### 1. Add `debugger_wait_for_stop` Tool

To avoid polling, add a blocking tool that waits for the next 'stopped' event:

```rust
// In src/mcp/tools/mod.rs

pub async fn wait_for_stop(
    &self,
    session_id: String,
    timeout_ms: Option<u64>,
) -> Result<serde_json::Value> {
    let session = self.manager.get_session(&session_id).await?;

    let timeout = Duration::from_millis(timeout_ms.unwrap_or(5000));
    let start_time = Instant::now();

    loop {
        let state = session.get_state().await;

        if let DebugState::Stopped { thread_id, reason } = state.state {
            return Ok(json!({
                "state": "Stopped",
                "threadId": thread_id,
                "reason": reason
            }));
        }

        if start_time.elapsed() > timeout {
            return Err(Error::Timeout("Timeout waiting for stop".to_string()));
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
```

**Tool definition**:
```json
{
  "name": "debugger_wait_for_stop",
  "description": "Blocks until the debugger stops (at breakpoint, step, or entry), or times out. More efficient than polling debugger_session_state.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "sessionId": {"type": "string"},
      "timeoutMs": {"type": "integer", "default": 5000, "description": "Max wait time in milliseconds"}
    },
    "required": ["sessionId"]
  }
}
```

**Usage**:
```javascript
debugger_continue({"sessionId": "..."})
debugger_wait_for_stop({"sessionId": "...", "timeoutMs": 5000})
// Returns immediately when stopped, or after 5s timeout
```

### 2. Add `debugger_list_breakpoints` Tool

```rust
pub async fn list_breakpoints(&self, session_id: String) -> Result<Vec<Breakpoint>> {
    let session = self.manager.get_session(&session_id).await?;
    let state = session.get_state().await;

    let mut all_breakpoints = Vec::new();
    for bps in state.breakpoints.values() {
        all_breakpoints.extend(bps.clone());
    }

    Ok(all_breakpoints)
}
```

### 3. Add State Validation to Tools

Update tools like `debugger_stack_trace` and `debugger_evaluate` to validate state:

```rust
pub async fn stack_trace(&self, session_id: String) -> Result<Vec<StackFrame>> {
    let session = self.manager.get_session(&session_id).await?;
    let state = session.get_state().await;

    // Validate we're stopped
    if !matches!(state.state, DebugState::Stopped { .. }) {
        return Err(Error::InvalidState(
            "Cannot get stack trace while program is running. Use debugger_wait_for_stop() first.".to_string()
        ));
    }

    // Proceed with stack trace...
}
```

## Testing Plan

### Test 1: Stop on Entry
```bash
1. Start with stopOnEntry: true
2. Check state immediately → Should be "Stopped" with reason: "entry"
3. Continue
4. Check state → Should transition to "Running"
```

### Test 2: Breakpoint Hit
```bash
1. Start session
2. Set breakpoint at line 20
3. Continue
4. Poll state → Should become "Stopped" with reason: "breakpoint"
5. Get stack trace → Should work and show line 20
```

### Test 3: Fast Loop (15 iterations)
```bash
1. Start with program that loops 1-15
2. Set breakpoint inside loop
3. For i in 1..15:
   a. Continue
   b. Wait for stop → Should detect each iteration
   c. Evaluate expressions
```

### Test 4: Program Termination
```bash
1. Start session
2. Continue without breakpoints
3. Poll state → Should eventually become "Terminated"
```

## Impact

**Before Fix**:
- ❌ State always shows "Running"
- ❌ Users can't tell when to inspect state
- ❌ Requires guessing and lucky discoveries
- ❌ Documentation doesn't match reality

**After Fix**:
- ✅ State accurately reflects debugger status
- ✅ Clear workflow: continue → wait → inspect
- ✅ Documentation accurate
- ✅ Professional debugging experience

## Priority: CRITICAL

This bug makes the debugger appear broken and frustrates users. Fixing it transforms the MCP tool from "barely usable with workarounds" to "excellent professional debugging tool".

## Estimated Complexity

- **Time**: 2-3 hours
- **Files**: 2-3 files
- **Lines**: ~100 lines of code
- **Risk**: Low (adding event handlers, not changing existing logic)
- **Testing**: Existing integration tests can be enhanced

## Summary

The fix is straightforward:
1. Register DAP event handlers during session initialization
2. Update session state when events arrive
3. Add validation to tools that require stopped state
4. Add `wait_for_stop` tool for better UX

This single change fixes the #1 critical issue and makes the debugger highly usable.
