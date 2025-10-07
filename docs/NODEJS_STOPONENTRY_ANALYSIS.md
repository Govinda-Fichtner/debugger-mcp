# Node.js stopOnEntry Analysis

## Date: 2025-10-07

## Summary

After extensive testing and source code analysis, **stopOnEntry does NOT work natively** with vscode-js-debug in its current "debug server" mode architecture. A workaround using entry breakpoints (like Ruby) is required.

## Investigation Process

### 1. Initial Hypothesis
**Hypothesis**: Node.js with vscode-js-debug should support stopOnEntry natively via `--inspect-brk`.

**Status**: ❌ REJECTED

### 2. Root Cause Analysis

#### Discovery 1: Launch Request Timeout
**Issue**: DAP launch requests were timing out after 10 seconds.

**Cause**: vscode-js-debug sends **reverse requests** (requests FROM server TO client) that we weren't handling.

**Solution**: Implemented reverse request handling in `src/dap/client.rs`:
```rust
Message::Request(req) => {
    // Send success response to reverse requests
    let response = Response {
        request_seq: req.seq,
        success: true,
        command: req.command.clone(),
        ...
    };
    transport.write_message(&Message::Response(response)).await?;
}
```

**Result**: ✅ Launch request no longer times out

#### Discovery 2: DAP Sequence Order
**Issue**: Even after fixing reverse requests, launch still hung.

**Cause**: vscode-js-debug expects this sequence:
1. Send `launch` request (don't wait)
2. Send `configurationDone` request
3. THEN receive `launch` response

But we were using `send_request()` which waits for response before continuing.

**Solution**: Use `send_request_nowait()` for launch:
```rust
client.send_request_nowait("launch", Some(launch_args)).await?;
client.configuration_done().await?;
```

**Result**: ✅ Launch completes successfully, Node.js process starts

#### Discovery 3: No 'stopped' Event
**Issue**: Node.js process launches but no 'stopped' event received.

**Investigation**:
- Manual testing confirms vscode-js-debug behavior
- Events received: `initialized`, `output` (telemetry), `output` (console)
- No `stopped` event despite `stopOnEntry: true`

**Root Cause**: vscode-js-debug uses a **parent-child session architecture**:

```
Parent Session (dapDebugServer.js)
    ↓ sends reverse request: startDebugging
Client (our code)
    ↓ should connect to child port
Child Session (actual debugging)
    ↓ sends stopped events
```

The parent session coordinates, but actual debugging events (stopped, breakpoint hit, etc.) come from the **child session** on a different port.

#### Discovery 4: Child Session Architecture

From nvim-dap-vscode-js source code:
```lua
reverse_request_handlers = {
    attachedChildSession = function(parent, request)
        -- Extract child port from request
        local child_port = tonumber(request.arguments.config.__jsDebugChildServer)

        -- Connect to child session
        session = require("dap.session"):connect(
            {host = "127.0.0.1", port = child_port},
            ...
        )
    end,
}
```

The reverse request contains:
- `__jsDebugChildServer`: Port number for child session
- `__pendingTargetId`: Target identifier

We would need to:
1. Parse the reverse request arguments
2. Extract the child session port
3. Create a NEW DAP client connection to that port
4. Manage multiple concurrent DAP sessions

### 3. vscode-js-debug Architecture

```
┌─────────────────────────────────┐
│ dapDebugServer.js (Parent)      │
│ - Coordinates debugging         │
│ - Spawns child sessions         │
│ - Sends reverse requests        │
└──────────┬──────────────────────┘
           │
           │ startDebugging reverse request
           │ (includes child port)
           ▼
┌─────────────────────────────────┐
│ Client (Our DAP MCP Server)     │
│ - Responds to reverse requests  │
│ - SHOULD connect to child port  │
└──────────┬──────────────────────┘
           │
           │ Connect to child port
           ▼
┌─────────────────────────────────┐
│ Child Session (pwa-node)        │
│ - Actually debugs Node.js       │
│ - Sends stopped events          │
│ - Handles breakpoints           │
└─────────────────────────────────┘
```

### 4. Why This Is Complex

Implementing full multi-session support requires:

1. **Session Management**
   - Track parent and child sessions
   - Route requests to correct session
   - Handle session lifecycle

2. **DAP Client Refactoring**
   - Support multiple concurrent connections
   - Handle per-session event streams
   - Manage per-session state

3. **MCP Integration**
   - Expose child sessions to MCP tools
   - Handle session identifiers
   - Coordinate between sessions

4. **Testing**
   - Multi-session test scenarios
   - Parent-child communication tests
   - Session cleanup tests

**Estimated effort**: 8-12 hours

## Workaround: Entry Breakpoint Pattern

**Solution**: Use the Ruby workaround pattern:

1. Launch with `stopOnEntry: false`
2. Set breakpoint at first line of program
3. Continue execution
4. Program stops at first line (like stopOnEntry)

**Advantages**:
- ✅ Works with current architecture
- ✅ No multi-session complexity
- ✅ Already proven with Ruby
- ✅ Can implement in 1-2 hours

**Disadvantages**:
- ❌ Not "true" stopOnEntry
- ❌ Requires source file access
- ❌ Extra DAP round-trip

## Implementation Plan: Entry Breakpoint Workaround

### 1. Update NodeJsAdapter (src/adapters/nodejs.rs)

Add entry breakpoint helper:
```rust
impl NodeJsAdapter {
    pub fn requires_entry_breakpoint_workaround() -> bool {
        true  // Node.js with vscode-js-debug needs workaround
    }

    pub fn get_entry_line(program: &str) -> Result<u32> {
        // Find first executable line in JavaScript file
        // Skip comments, empty lines, etc.
        // Return line number (usually 1 for most JS files)
        Ok(1)
    }
}
```

### 2. Update DebugSession (src/debug/session.rs)

Implement entry breakpoint sequence:
```rust
async fn initialize_and_launch_with_entry_workaround(&self, ...) -> Result<()> {
    // 1. Initialize
    self.client.initialize(adapter_id).await?;

    // 2. Wait for 'initialized' event
    self.client.wait_for_event("initialized", Duration::from_secs(2)).await?;

    if stop_on_entry && self.adapter_type == Some("nodejs") {
        // 3. Set breakpoint at entry line
        let entry_line = NodeJsAdapter::get_entry_line(&self.program)?;
        self.client.set_breakpoints(&self.program, &[entry_line]).await?;
    }

    // 4. Send launch (with stopOnEntry: false)
    let mut launch_args = launch_args.clone();
    if self.adapter_type == Some("nodejs") {
        launch_args["stopOnEntry"] = json!(false);
    }
    self.client.send_request_nowait("launch", Some(launch_args)).await?;

    // 5. Send configurationDone
    self.client.configuration_done().await?;

    if stop_on_entry && self.adapter_type == Some("nodejs") {
        // 6. Continue (will stop at entry breakpoint)
        self.client.continue_execution(None).await?;

        // 7. Wait for stopped event
        self.wait_for_stopped(Duration::from_secs(5)).await?;
    }

    Ok(())
}
```

### 3. Testing

Update integration test:
```rust
#[tokio::test]
#[ignore]
async fn test_nodejs_stop_on_entry_with_workaround() {
    // Test entry breakpoint workaround
    // Should receive 'stopped' event at line 1
}
```

## Alternative Solutions (Future Work)

### Option 1: Multi-Session Support
Implement full parent-child session handling.

**Effort**: 8-12 hours
**Benefit**: "True" stopOnEntry, more robust

### Option 2: Direct CDP Integration
Skip vscode-js-debug, use Chrome DevTools Protocol directly.

**Effort**: 20+ hours (new protocol implementation)
**Benefit**: No intermediate adapter, potentially simpler

### Option 3: Use js-debug-adapter
Check if there's a simpler entry point than dapDebugServer.js.

**Effort**: 4-6 hours
**Benefit**: May avoid multi-session complexity

## Conclusion

**Decision**: Implement entry breakpoint workaround for MVP.

**Rationale**:
- Proven pattern (Ruby uses it successfully)
- Low implementation effort (1-2 hours)
- Unblocks integration testing
- Can iterate to multi-session support later

**Trade-off**: Accept that stopOnEntry is not "native" for Node.js in this version.

**Future Enhancement**: Implement multi-session support in v2.0 for true stopOnEntry.

---

**Status**: Workaround approach approved
**Next Step**: Implement entry breakpoint pattern
**Timeline**: 1-2 hours

