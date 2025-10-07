# Node.js Debugging Investigation - Session Summary

## Date: 2025-10-07

## Objective

Resolve the Node.js launch timeout issue and enable stopOnEntry functionality.

## Investigation Methodology

Used **direct binary experiments** and **source code analysis** to eliminate confusion:

1. **Manual DAP Testing**: Created Node.js script to send raw DAP messages to vscode-js-debug
2. **Source Code Analysis**: Studied nvim-dap-vscode-js and vscode-js-debug repositories
3. **Iterative Testing**: Made targeted changes, tested, analyzed results

## Key Discoveries

### Discovery 1: Reverse Requests (Root Cause of Timeout)

**Problem**: Launch requests timed out after 10 seconds.

**Investigation**:
```bash
# Manual test revealed vscode-js-debug sends requests TO client
node test_dap_manual.js 8127
# Output showed: startDebugging request (seq: 7)
```

**Finding**: DAP specification allows "reverse requests" (server→client). Our client logged:
```rust
Message::Request(_) => {
    warn!("Received request from debug adapter (reverse requests not implemented)");
}
```

**Solution**: Implemented reverse request handler:
```rust
Message::Request(req) => {
    info!("🔄 REVERSE REQUEST received: '{}' (seq {})", req.command, req.seq);
    let response = Response {
        request_seq: req.seq,
        success: true,
        command: req.command.clone(),
        ...
    };
    transport.write_message(&Message::Response(response)).await?;
}
```

**Result**: ✅ Launch timeout fixed

### Discovery 2: DAP Sequence Order

**Problem**: Even after fixing reverse requests, launch still hung.

**Investigation**: Manual test showed this sequence:
```
1. Send launch request
2. Send configurationDone request
3. Receive configurationDone response (immediate)
4. Receive launch response (delayed)
```

**Finding**: vscode-js-debug expects launch/configurationDone to be sent before waiting for responses.

**Solution**: Use `send_request_nowait()` for launch:
```rust
client.send_request_nowait("launch", Some(launch_args)).await?;
client.configuration_done().await?;
// Responses arrive asynchronously
```

**Result**: ✅ Launch completes successfully, Node.js process starts

### Discovery 3: Multi-Session Architecture

**Problem**: Node.js launches but no 'stopped' event received.

**Investigation**:
1. Manual test showed `startDebugging` reverse request
2. nvim-dap source showed child session handling:
   ```lua
   reverse_request_handlers = {
       attachedChildSession = function(parent, request)
           local child_port = tonumber(request.arguments.config.__jsDebugChildServer)
           session = require("dap.session"):connect({port = child_port}, ...)
       end,
   }
   ```

**Finding**: vscode-js-debug uses parent-child session model:
- **Parent session**: Coordination, spawning
- **Child session**: Actual debugging (stopped events, breakpoints, etc.)

**Architecture**:
```
Parent (dapDebugServer.js)
    ↓ startDebugging reverse request
Client (our code)
    ↓ SHOULD connect to child port
Child (pwa-node)
    ↓ stopped events here!
```

**Implication**: To get 'stopped' events, we need multi-session support.

**Effort**: 8-12 hours to implement properly

### Discovery 4: Pragmatic Workaround

**Solution**: Entry breakpoint pattern (Ruby uses this):
1. Launch with `stopOnEntry: false`
2. Set breakpoint at line 1
3. Continue execution
4. Program stops at first line (same effect as stopOnEntry)

**Advantages**:
- ✅ Works with current architecture
- ✅ 1-2 hour implementation
- ✅ Proven with Ruby
- ✅ Unblocks testing

**Trade-off**: Not "true" stopOnEntry, but functionally equivalent

## Results

### Before Investigation:
- ❌ Launch timeout (10 seconds)
- ❌ Node.js not starting
- ❌ No debugging possible
- ❓ Root cause unknown

### After Investigation:
- ✅ Launch completes immediately
- ✅ Node.js process starts successfully
- ✅ Events received (telemetry, console output)
- ✅ Root cause identified and documented
- ✅ Clear path forward (workaround)
- 📋 Future enhancement documented (multi-session)

## Files Created/Modified

**Created**:
- `/tmp/test_dap_manual.js` - Manual DAP testing script
- `docs/NODEJS_STOPONENTRY_ANALYSIS.md` - Complete analysis (400+ lines)
- `docs/NODEJS_SESSION_SUMMARY.md` - This summary

**Modified**:
- `src/dap/client.rs` - Reverse request handling
- `tests/test_nodejs_integration.rs` - Correct DAP sequence, event logging

## Commits

```
36a262b fix(dap): Implement reverse request handling for vscode-js-debug
```

## Progress

**Before**: 90% complete (blocked by launch timeout)
**After**: 95% complete (just need entry breakpoint workaround)

**Remaining**:
1. Implement entry breakpoint workaround (1-2 hours)
2. Update integration tests (1 hour)
3. Create Docker image (1 hour)
4. Documentation and PR (1 hour)

**Total remaining**: 4-5 hours

## Key Learnings

### 1. Direct Binary Experiments Are Essential

Creating `/tmp/test_dap_manual.js` to send raw DAP messages was crucial. It revealed:
- Exact message sequence
- Reverse request behavior
- Response timing

**Lesson**: When protocol communication is unclear, test at the binary/protocol level.

### 2. Source Code Is the Truth

Reading nvim-dap-vscode-js source code revealed:
- `reverse_request_handlers` pattern
- Child session port extraction
- Multi-session architecture

**Lesson**: Study working implementations to understand complex systems.

### 3. Pragmatic Solutions > Perfect Solutions

Multi-session support is the "correct" solution, but:
- Takes 8-12 hours
- High complexity
- Blocks progress

Entry breakpoint workaround:
- Takes 1-2 hours
- Low complexity
- Unblocks testing
- Can iterate later

**Lesson**: Choose pragmatic solutions for MVP, iterate to perfection later.

### 4. Test-Driven Investigation

Process:
1. Manual binary test → reveals behavior
2. Analyze source code → confirms understanding
3. Implement fix → test validates
4. Document findings → prevents regression

**Lesson**: TDD applies to debugging too!

## Next Steps

1. **Immediate** (1-2 hours):
   - Implement entry breakpoint workaround
   - Update tests to verify stopOnEntry behavior

2. **Short-term** (2-3 hours):
   - Complete remaining integration tests
   - Create Docker image
   - Update documentation

3. **Pull Request** (1 hour):
   - Final review
   - Create PR
   - Merge to main

4. **Future Enhancement** (v2.0):
   - Implement full multi-session support
   - Enable "true" native stopOnEntry
   - Support multiple concurrent debugging targets

## Conclusion

Through systematic investigation using direct experiments and source code analysis, we:

- ✅ Identified root cause (reverse requests + multi-session architecture)
- ✅ Fixed launch timeout issue (10s → immediate)
- ✅ Enabled Node.js process launching
- ✅ Documented complete architecture
- ✅ Planned pragmatic workaround

**Status**: Ready to implement entry breakpoint workaround and complete Node.js debugging support.

**Timeline**: 4-5 hours to completion

**Confidence**: High (proven pattern from Ruby implementation)

---

**Session Duration**: ~3 hours
**Lines of Code**: +400 analysis, +30 fixes
**Tests**: Launch sequence working, stopOnEntry workaround planned
**Knowledge Gained**: vscode-js-debug multi-session architecture, DAP reverse requests, pragmatic vs perfect trade-offs

