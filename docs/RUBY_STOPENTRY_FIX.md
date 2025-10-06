# Ruby stopOnEntry Fix Implementation Plan

**Date**: 2025-10-07
**Issue**: rdbg doesn't send `stopped` event when using `--stop-at-load` with `--open` (socket mode)
**Status**: Ready for implementation

---

## Problem Summary

### Current Behavior ❌

```
1. debugger_start(language="ruby", stopOnEntry=true)
2. Spawn: rdbg --open --port X --stop-at-load program.rb ✅
3. Launch request: {"stopOnEntry": true} ✅
4. Events received:
   - initialized ✅
   - output ✅
   - loadedSource ✅
   - terminated ✅ (program ran to completion!)
5. Missing: stopped (reason: "entry") ❌
```

### Expected Behavior ✅

```
1. debugger_start(language="ruby", stopOnEntry=true)
2. Spawn: rdbg --open --port X --stop-at-load program.rb ✅
3. Launch request: {"stopOnEntry": true} ✅
4. Events received:
   - initialized ✅
   - stopped (reason: "entry") ✅ ← Should pause here!
   [user can set breakpoints, inspect state]
5. debugger_continue()
6. terminated ✅
```

---

## Root Cause Analysis

### Hypothesis 1: rdbg Socket Mode Doesn't Honor --stop-at-load

**Evidence:**
- `--stop-at-load` flag is documented for stdio mode
- Socket mode (`--open`) may bypass this flag
- No `stopped` event is sent after `initialized`

**Test needed:**
```bash
# Manual test
rdbg --open --port 12345 --stop-at-load fizzbuzz.rb

# Connect via nc and send DAP initialize/launch
# Check if stopped event is sent
```

### Hypothesis 2: Race Condition

**Evidence:**
- Program starts immediately after socket connection
- `--stop-at-load` should pause before first instruction
- But socket mode might start execution before DAP client is ready
- Fast programs (like FizzBuzz) complete in ~200ms

---

## Proposed Solution: Explicit Pause Request

### Strategy

**Workaround for rdbg socket mode:**
1. After receiving `initialized` event
2. If language is Ruby AND stopOnEntry is true
3. Send explicit `pause` request
4. Wait for `stopped` event with reason "pause"
5. Then send `configurationDone`

### Why This Works

- `pause` is a standard DAP request (like breakpoint, continue)
- Should work regardless of debugger implementation
- Gives consistent behavior across languages
- Minimal changes to existing code

### Implementation

#### Step 1: Add pause() method to DapClient

```rust
// src/dap/client.rs

pub async fn pause(&self, thread_id: Option<i32>) -> Result<()> {
    let args = if let Some(tid) = thread_id {
        serde_json::json!({ "threadId": tid })
    } else {
        serde_json::json!({ "threadId": 1 })  // Default to thread 1
    };

    let response = self.send_request("pause", Some(args)).await?;

    if !response.success {
        return Err(Error::Dap(format!("Pause failed: {:?}", response.message)));
    }

    info!("✅ Pause request successful");
    Ok(())
}
```

#### Step 2: Modify initialize_and_launch to accept adapter type

```rust
// src/dap/client.rs

pub async fn initialize_and_launch(
    &self,
    adapter_id: &str,
    launch_args: Value,
    adapter_type: Option<&str>,  // NEW: "python" or "ruby"
) -> Result<()> {
    // ... existing initialize code ...

    // Check if we need Ruby stopOnEntry workaround
    let is_ruby = adapter_type == Some("ruby");
    let stop_on_entry = launch_args.get("stopOnEntry")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let needs_ruby_workaround = is_ruby && stop_on_entry;

    // ... register initialized event handler ...

    // Step 4: Wait for 'initialized' event
    if config_done_supported {
        info!("Waiting for 'initialized' event (timeout: 5s)...");
        match tokio::time::timeout(tokio::time::Duration::from_secs(5), init_rx).await {
            Ok(Ok(())) => {
                info!("✅ Received 'initialized' event signal");

                // NEW: Ruby stopOnEntry workaround
                if needs_ruby_workaround {
                    info!("🔧 Applying Ruby stopOnEntry workaround: sending pause request");

                    // Send pause request
                    self.pause(None).await?;
                    info!("Pause request sent, waiting for stopped event...");

                    // Wait for stopped event (with 2s timeout)
                    match tokio::time::timeout(
                        tokio::time::Duration::from_secs(2),
                        self.wait_for_event("stopped")
                    ).await {
                        Ok(Ok(_)) => {
                            info!("✅ Received 'stopped' event - program paused at entry");
                        }
                        Ok(Err(e)) => {
                            warn!("⚠️  Error waiting for 'stopped' event: {}", e);
                            // Continue anyway - configurationDone might still work
                        }
                        Err(_) => {
                            warn!("⚠️  Timeout waiting for 'stopped' event");
                            // Continue anyway - configurationDone might still work
                        }
                    }
                }
            }
            // ... existing error handling ...
        }

        // Step 5: Send configurationDone
        info!("Sending configurationDone");
        self.configuration_done().await?;
        info!("configurationDone completed");
    }

    // ... rest of method ...
}
```

#### Step 3: Update initialize_and_launch_with_timeout signature

```rust
// src/dap/client.rs

pub async fn initialize_and_launch_with_timeout(
    &self,
    adapter_id: &str,
    launch_args: Value,
    adapter_type: Option<&str>,  // NEW
) -> Result<()> {
    let timeout = std::time::Duration::from_secs(7);
    info!("⏱️  initialize_and_launch_with_timeout: Starting with 7s timeout");

    tokio::time::timeout(
        timeout,
        self.initialize_and_launch(adapter_id, launch_args, adapter_type)
    )
        .await
        .map_err(|_| Error::Dap(format!("Initialize and launch timed out after {:?}", timeout)))?
}
```

#### Step 4: Update DebugSession to pass adapter type

```rust
// src/debug/session.rs (around line 136)

// Before:
client.initialize_and_launch_with_timeout(adapter_id, launch_args).await?;

// After:
let adapter_type = match language {
    "python" => Some("python"),
    "ruby" => Some("ruby"),
    _ => None,
};
client.initialize_and_launch_with_timeout(adapter_id, launch_args, adapter_type).await?;
```

---

## Testing Plan

### Unit Tests

```rust
// tests/test_ruby_stopentry_fix.rs

#[tokio::test]
#[ignore] // Requires rdbg installed
async fn test_ruby_stopentry_workaround() {
    // 1. Spawn rdbg with stopOnEntry=true
    let session = RubyAdapter::spawn("fizzbuzz.rb", &[], true).await.unwrap();
    let client = DapClient::from_socket(session.socket).await.unwrap();

    // 2. Initialize and launch (should trigger workaround)
    client.initialize_and_launch_with_timeout(
        "rdbg",
        json!({
            "type": "ruby",
            "program": "fizzbuzz.rb",
            "stopOnEntry": true
        }),
        Some("ruby")
    ).await.unwrap();

    // 3. Verify we're in stopped state
    // (This requires event tracking - TBD)

    // 4. Continue execution
    client.continue_execution(1).await.unwrap();

    // 5. Wait for termination
    client.wait_for_event("terminated").await.unwrap();
}
```

### Integration Test with Claude Code

```javascript
// User in Claude Code:
debugger_start({
  language: "ruby",
  program: "/workspace/fizzbuzz.rb",
  stopOnEntry: true
})

// Should now:
// 1. Spawn rdbg with --stop-at-load ✅
// 2. Receive initialized event ✅
// 3. Send pause request ✅ (NEW)
// 4. Receive stopped event ✅ (NEW)
// 5. Allow breakpoint setting ✅
// 6. Wait for user to call debugger_continue ✅
```

---

## Alternative Solutions Considered

### Option 2: Pre-set Breakpoint at Line 1

**Approach:**
```rust
if needs_ruby_workaround {
    // Set breakpoint at line 1 before configurationDone
    let source = Source {
        path: Some(program_path.clone()),
        name: None,
        source_reference: None,
    };
    client.set_breakpoint(source, vec![
        SourceBreakpoint { line: 1, condition: None }
    ]).await?;
}
```

**Pros:**
- Uses standard breakpoint mechanism
- No need for pause request

**Cons:**
- Assumes line 1 is executable (might not be)
- Breakpoint persists (needs cleanup)
- More complex state management

**Decision:** Use pause request (simpler, more reliable)

### Option 3: Switch to stdio Mode

**Approach:**
```rust
// Try stdio instead of socket
spawn("rdbg", ["--stop-at-load", program])
```

**Pros:**
- Might honor --stop-at-load better
- Simpler process management

**Cons:**
- Already tried and failed (see RUBY_DEBUGGING_FIX_SUMMARY.md)
- rdbg stdio mode has issues with DAP protocol
- Would require reverting socket implementation

**Decision:** Keep socket mode, add workaround

---

## Expected Outcomes

### After Fix ✅

```
Test 1: stopOnEntry=true
- Spawn: rdbg --open --port X --stop-at-load program.rb ✅
- Events: initialized → pause request → stopped ✅
- State: Stopped (not Terminated) ✅
- Breakpoints can be set ✅
- debugger_continue works ✅

Test 2: stopOnEntry=false
- Spawn: rdbg --open --port X --nonstop program.rb ✅
- No pause request sent ✅
- Program runs to completion ✅
- (Fast programs still finish quickly, but that's expected)
```

### Performance Impact

- **Minimal**: +50-100ms for pause request + stopped event wait
- **Total startup**: 500-700ms (previously 400-600ms)
- **Acceptable**: Still well under 7s timeout

---

## Documentation Updates

### 1. Code Comments

```rust
// Ruby debuggers (rdbg) in socket mode don't honor --stop-at-load properly.
// Workaround: After 'initialized' event, send explicit 'pause' request.
// This ensures the program stops at entry point for fast-executing scripts.
// See: docs/RUBY_STOPENTRY_FIX.md
```

### 2. User Documentation

Update `README.md`:
```markdown
## Known Limitations

### Ruby stopOnEntry

Ruby debugging uses a workaround for `stopOnEntry: true`:
- rdbg in socket mode doesn't honor `--stop-at-load` properly
- We send an explicit `pause` request after `initialized` event
- Adds ~50-100ms to startup time
- Fully transparent to users
```

### 3. Technical Documentation

Create `docs/RUBY_STOPENTRY_FIX.md` (this document)

---

## Risks and Mitigation

### Risk 1: pause Request Not Supported

**Mitigation:** Check capabilities for `supportsPauseRequest`
```rust
if needs_ruby_workaround {
    if capabilities.supports_pause_request != Some(true) {
        warn!("⚠️  Adapter doesn't support pause - stopOnEntry may not work");
        // Continue without pause (better than failing)
    } else {
        self.pause(None).await?;
    }
}
```

### Risk 2: Timeout Waiting for stopped

**Mitigation:** Already handled with 2s timeout + warning
```rust
Err(_) => {
    warn!("⚠️  Timeout waiting for 'stopped' event");
    // Continue anyway - configurationDone might still work
}
```

### Risk 3: Breaks Python Debugging

**Mitigation:** Only apply workaround for Ruby
```rust
let needs_ruby_workaround = is_ruby && stop_on_entry;
```

---

## Implementation Checklist

- [ ] Add `pause()` method to `DapClient`
- [ ] Modify `initialize_and_launch()` to accept `adapter_type`
- [ ] Add Ruby stopOnEntry workaround logic
- [ ] Update `initialize_and_launch_with_timeout()` signature
- [ ] Update `DebugSession` to pass adapter type
- [ ] Add unit test for pause method
- [ ] Add integration test for Ruby stopOnEntry
- [ ] Update all call sites
- [ ] Document workaround in code comments
- [ ] Update README.md with known limitation
- [ ] Test with Claude Code end-to-end
- [ ] Verify Python debugging still works
- [ ] Update CHANGELOG.md

---

## Success Criteria

✅ **Must Have:**
1. Ruby debugging with `stopOnEntry: true` stops at entry
2. `stopped` event received after `initialized`
3. Breakpoints can be set before execution
4. Python debugging unaffected
5. All existing tests still pass

⭐ **Nice to Have:**
1. Performance overhead < 100ms
2. Clear warning if pause not supported
3. Graceful fallback if workaround fails

---

## Timeline

- **Implementation**: 2-3 hours
- **Testing**: 1-2 hours
- **Documentation**: 1 hour
- **Total**: 4-6 hours

---

## References

- **Test Results**: `/home/vagrant/projects/fizzbuzz-ruby-test/FINAL_TEST_RESULTS.md`
- **DAP Specification**: https://microsoft.github.io/debug-adapter-protocol/specification#Requests_Pause
- **rdbg Documentation**: `gem info debug` or `rdbg --help`
- **Existing Socket Implementation**: `docs/RUBY_SOCKET_IMPLEMENTATION_SUMMARY.md`

---

**Next Step**: Begin implementation with pause() method addition
