# Node.js Child Session Spawning - Investigation Needed

## Date: 2025-10-07

## Current Status

**Parent session**: ✅ Working
- vscode-js-debug spawns successfully
- Parent session connects via TCP socket
- Initialize and launch work correctly
- Breakpoints can be set on parent

**Child session**: ❌ Not working
- `startDebugging` reverse request received correctly
- However, child session port extraction fails
- Expected `__jsDebugChildServer` field not present
- Instead, configuration has `__pendingTargetId`

## The Problem

### What We Expected

Based on research, we expected vs code-js-debug to send:

```json
{
  "seq": 8,
  "type": "request",
  "command": "startDebugging",
  "arguments": {
    "configuration": {
      "__jsDebugChildServer": "9230"  // ❌ NOT PRESENT
    }
  }
}
```

### What We Actually Got

```json
{
  "seq": 8,
  "type": "request",
  "command": "startDebugging",
  "arguments": {
    "request": "launch",
    "configuration": {
      "type": "pwa-node",
      "name": "fizzbuzz.js [623]",
      "__pendingTargetId": "e7a6f5e064861518f63a9c64"  // ⚠️  Different mechanism
    }
  }
}
```

## Root Cause Analysis

### Hypothesis 1: Different vscode-js-debug Mode

vscode-js-debug may have multiple modes:
- **Server mode** (what VS Code uses) - provides `__jsDebugChildServer`
- **Standalone mode** (what we're using) - uses `__pendingTargetId` + different mechanism

**Evidence**:
- We spawn vscode-js-debug directly: `node dapDebugServer.js`
- VS Code may use a different entry point or configuration

### Hypothesis 2: Two-Step Child Session Creation

The child session may require:
1. Respond to `startDebugging` with success ✅ (we do this)
2. Listen for another DAP message/event that provides the actual child port ❌ (we don't do this)

**Possible mechanisms**:
- Another reverse request after `startDebugging`
- An event that contains the child session info
- The port might be embedded in `__pendingTargetId` somehow

### Hypothesis 3: We Need to Handle `__pendingTargetId`

The `__pendingTargetId` might be:
- A token to query vscode-js-debug for the actual port
- Anidentifier for a pending connection that we need to accept
- A signal that we should connect to a different port (not provided in message)

## Investigation Steps

### Step 1: Check vscode-js-debug Source Code

**File to examine**: `/tmp/js-debug/src/dapDebugServer.js` and related files

**What to look for**:
- How `__pendingTargetId` is used
- What DAP messages follow `startDebugging`
- Alternative APIs for child session management

### Step 2: Enable vscode-js-debug Debug Logging

vscode-js-debug likely has debug logging. Try:
```bash
NODE_DEBUG=* node /tmp/js-debug/src/dapDebugServer.js
```

Or check for environment variables like:
- `DEBUG=*`
- `VSCODE_JS_DEBUG_LOG=trace`

### Step 3: Compare with Working VS Code Setup

**Reference implementations**:
1. VS Code's built-in debugger configuration
2. nvim-dap with vscode-js-debug
3. Other DAP clients using vscode-js-debug

**What to extract**:
- Exact DAP message sequences
- How child session port is discovered
- Configuration differences

### Step 4: Minimal Reproduction

Create minimal DAP client that:
1. Connects to vscode-js-debug
2. Sends initialize
3. Sends launch
4. Logs ALL DAP messages received
5. Documents the complete message flow

### Step 5: Alternative: Use Different Adapter

If vscode-js-debug proves too complex, consider:
- **node-debug2** (older, simpler)
- **node-inspector** (deprecated but well-documented)
- Direct V8 Inspector Protocol (bypass DAP entirely)

## Code Locations

### Where Child Session Spawn is Handled

**File**: `src/dap/client.rs:206-229`

```rust
if command == "startDebugging" {
    info!("🔄 REVERSE REQUEST received: 'startDebugging' (seq {})", seq);
    info!("   🎯 startDebugging request detected - checking for child session");

    // Try to extract child port from __jsDebugChildServer
    if let Some(config) = args.get("configuration") {
        if let Some(child_server) = config.get("__jsDebugChildServer") {
            // ... spawn child session
        } else {
            info!("   No __jsDebugChildServer in configuration");
        }
    }
}
```

**What needs to change**:
- Handle `__pendingTargetId` field
- Implement alternative child port discovery
- Add more logging to understand vscode-js-debug behavior

### Where Callback is Registered

**File**: `src/debug/manager.rs:190-207`

```rust
// Register child session spawn callback on parent client
info!("🔄 [NODEJS] Registering child session spawn callback");
let session_clone = session_arc.clone();
if let SessionMode::MultiSession { parent_client, .. } = &session_arc.session_mode {
    let parent = parent_client.read().await;
    parent
        .on_child_session_spawn(move |port| {
            let session = session_clone.clone();
            Box::pin(async move {
                info!("🎯 [NODEJS] Child session spawn callback invoked for port {}", port);
                if let Err(e) = session.spawn_child_session(port).await {
                    error!("❌ [NODEJS] Failed to spawn child session on port {}: {}", port, e);
                }
            })
        })
        .await;
}
```

**Callback is registered** ✅ but **never invoked** ❌ because port isn't found.

## Workaround for Tests

Until child session spawning is fixed, tests should:

1. **Use `stopOnEntry=false`** - Don't rely on entry breakpoint
2. **Set explicit breakpoints** - Use `set_breakpoint()` before continuing
3. **Test single-session functionality** - Python/Ruby style debugging
4. **Document limitation** - Explain multi-session is WIP

### Example Test Pattern

```rust
// Create session WITHOUT stopOnEntry
let session_id = manager.create_session(
    "nodejs",
    "program.js",
    vec![],
    None,
    false,  // stopOnEntry = false
).await?;

// Set breakpoint manually
let session = manager.get_session(&session_id).await?;
session.set_breakpoint("program.js", 10).await?;

// Program runs, hits breakpoint (if child session works)
// OR runs to completion (if child session doesn't work)
```

## Impact Assessment

### What Works ✅

- **Parent session creation** - vscode-js-debug starts correctly
- **Basic DAP operations on parent** - initialize, launch, setBreakpoints
- **Reverse request handling** - `startDebugging` received and responded to
- **Breakpoint setting** - Breakpoints can be set (though not verified until child spawns)

### What Doesn't Work ❌

- **Child session spawning** - Can't extract port from `startDebugging`
- **Stop events** - Child sends stopped events, parent doesn't receive them
- **Expression evaluation** - Can't evaluate in child context
- **Stepping** - Can't step through code (needs active child session)
- **StopOnEntry** - Entry breakpoint workaround doesn't help without child

### User Experience

**Current**: Node.js debugging **appears** to work (no errors) but **doesn't actually debug**:
- Sessions are created successfully
- Breakpoints are set (unverified)
- Programs run to completion without stopping
- No error messages to indicate why it's not working

**This is worse than an obvious error!** Users will be confused.

### Recommended Approach

**Short term** (until fix):
1. Document limitation clearly in README
2. Mark Node.js as "experimental" or "WIP"
3. Provide clear error message when child session spawn fails
4. Fall back to simpler debugger (node-inspect?) if vscode-js-debug doesn't work

**Long term**:
1. Research vscode-js-debug protocol thoroughly
2. Implement correct child session discovery
3. Add comprehensive integration tests
4. Document the complete multi-session flow

## Timeline Estimate

- **Investigation**: 4-8 hours (understanding vscode-js-debug internals)
- **Implementation**: 2-4 hours (once mechanism is clear)
- **Testing**: 2-3 hours (validation with real programs)
- **Total**: 8-15 hours

## Priority

**Medium-High**:
- Not blocking Python/Ruby debugging ✅
- Node.js is a major use case (high demand)
- Current implementation is misleading (appears to work but doesn't)
- Alternative: document limitation and defer to future release

## Next Action

1. **Document current limitation** in README and NODEJS_INTEGRATION_STATUS
2. **Add warning in code** when Node.js session creation fails silently
3. **Create GitHub issue** with this analysis for future work
4. **Consider alternative adapters** if vscode-js-debug too complex

---

**Status**: Investigation Needed
**Assignee**: Future Work
**Related**: Multi-session architecture (working), vscode-js-debug integration (partial)
