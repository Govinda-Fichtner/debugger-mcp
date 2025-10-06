# Test Coverage Gap Analysis

## Question

**Why didn't the Ruby integration tests catch the problems that manual testing uncovered?**

## Problems Uncovered by Manual Testing

1. ❌ **rdbg not running** - Only plain `ruby` process visible
2. ❌ **Session stuck in "Initializing"** - No DAP communication
3. ❌ **Disconnect hangs forever** - No timeout
4. ❌ **Stdio vs Socket mismatch** - rdbg doesn't support stdio DAP

## What the Integration Tests Actually Tested

### Test 1: `test_ruby_adapter_spawn_real_rdbg`
```rust
async fn test_ruby_adapter_spawn_real_rdbg() {
    let result = RubyAdapter::spawn(test_script, &[], true).await;
    assert!(result.is_ok());  // ← ONLY checks spawn succeeds

    let session = result.unwrap();
    assert!(session.port > 1024);  // ← ONLY checks port valid
    assert!(session.socket.peer_addr().is_ok());  // ← ONLY checks socket connected
}
```

**What it TESTED**:
✅ Process spawns without error
✅ Port is allocated
✅ Socket connects

**What it DIDN'T TEST**:
❌ DAP protocol communication
❌ Initialize handshake
❌ Launch request
❌ Breakpoint setting
❌ Session state transitions
❌ Actual debugging workflow

### Test 2: `test_ruby_e2e_dap_communication`
```rust
async fn test_ruby_e2e_dap_communication() {
    let session = RubyAdapter::spawn(test_script, &[], true).await;
    let client = DapClient::from_socket(session.socket).await;

    let init_response = client.send_request("initialize", ...).await;
    assert!(init_response.success);  // ← Should catch issues!

    let launch_response = client.send_request("launch", ...).await;
    assert!(launch_response.success);  // ← Should catch issues!
}
```

**What happened**:
- Test created DAP client from socket ✅
- Sent initialize request ✅
- **Got response back** ✅

**But**: The test used `send_request()` which is a lower-level method that doesn't go through the full session initialization flow!

## The Critical Gap

### Integration Tests Used Low-Level APIs

```rust
// Integration test (INCOMPLETE)
let client = DapClient::from_socket(socket).await;
client.send_request("initialize", args).await;  // Direct API call
```

### Manual Testing Used High-Level APIs

```rust
// Real usage (COMPLETE)
let session_id = manager.start_session("ruby", program, args).await;
// ^ This goes through DebugSession::initialize_and_launch_async()
```

## Why This Matters

### The Missing Link: `DebugSession`

The integration tests never created a `DebugSession` object!

**What `DebugSession::initialize_and_launch_async()` does**:
1. Sends initialize request
2. Waits for initialized event
3. Sends launch request
4. Waits for process to start
5. Updates session state
6. **Handles timeouts**
7. **Manages state machine**

### Integration Tests Bypassed This

```rust
// What tests did (LOW-LEVEL)
RubyAdapter::spawn() → DapClient → send_request()

// What manual testing did (HIGH-LEVEL)
SessionManager → DebugSession → initialize_and_launch_async()
                                 ↑
                        This is where bugs were!
```

## What We Should Have Tested

### Missing Test: Full Session Lifecycle

```rust
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_full_session_lifecycle() {
    // 1. Create SessionManager (like real MCP server)
    let manager = SessionManager::new();

    // 2. Start session (like Claude Code does)
    let session_id = manager.start_session(
        "ruby",
        "/workspace/fizzbuzz.rb",
        vec![],
        None,
        true // stopOnEntry
    ).await.expect("Failed to start session");

    // 3. Wait for initialization (THIS is where bugs occurred!)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 4. Check session state
    let session = manager.get_session(&session_id).await.unwrap();
    let state = session.get_state().await;

    // ← THIS would have caught "stuck in Initializing"!
    assert!(matches!(state, DebugState::Stopped { .. }),
            "Expected Stopped, got {:?}", state);

    // 5. Set breakpoint
    let bp_result = session.set_breakpoints("fizzbuzz.rb", vec![9]).await;
    assert!(bp_result.is_ok());

    // 6. Continue execution
    session.continue_execution(1).await.expect("Continue failed");

    // 7. Wait for breakpoint
    tokio::time::sleep(Duration::from_millis(200)).await;
    let state = session.get_state().await;
    assert!(matches!(state, DebugState::Stopped { .. }));

    // 8. Evaluate variable
    let frames = session.stack_trace(1).await.unwrap();
    let result = session.evaluate("n", frames[0].id).await;
    assert!(result.is_ok());

    // 9. Disconnect (THIS would have caught infinite hang!)
    let disconnect_result = tokio::time::timeout(
        Duration::from_secs(2),
        manager.remove_session(&session_id)
    ).await;

    assert!(disconnect_result.is_ok(), "Disconnect timed out!");
}
```

### Missing Test: State Transitions

```rust
#[tokio::test]
#[ignore]
async fn test_ruby_session_state_transitions() {
    let manager = SessionManager::new();
    let session_id = manager.start_session(...).await.unwrap();

    // Check initial state
    let state = manager.get_session_state(&session_id).await;
    assert_eq!(state, "Initializing");  // ← Would catch if stuck here!

    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check stopped state
    let state = manager.get_session_state(&session_id).await;
    assert_eq!(state, "Stopped");  // ← Would verify state machine works!

    // Continue
    manager.continue_execution(&session_id).await.unwrap();

    // Check running state
    let state = manager.get_session_state(&session_id).await;
    assert!(matches!(state, "Running" | "Stopped"));
}
```

### Missing Test: Wait for Stop

```rust
#[tokio::test]
#[ignore]
async fn test_ruby_wait_for_stop_works() {
    let manager = SessionManager::new();
    let session_id = manager.start_session(..., stopOnEntry: true).await.unwrap();

    // THIS is what Claude Code does!
    let result = manager.wait_for_stop(&session_id, 5000).await;

    // ← Would have caught timeout issues!
    assert!(result.is_ok(), "wait_for_stop failed: {:?}", result.err());
    assert_eq!(result.unwrap().state, "Stopped");
}
```

## Root Cause Summary

| Aspect | Integration Tests | Manual Testing |
|--------|------------------|----------------|
| **Level** | Low (DapClient) | High (SessionManager) |
| **Scope** | Socket connection | Full workflow |
| **State** | Not tested | State machine tested |
| **Async** | Simple awaits | Complex event handling |
| **Timeouts** | Not tested | Critical path |
| **Bugs Found** | 0 | All of them |

## Lessons Learned

### 1. Test at Multiple Levels

```
Unit Tests     ✅ Socket helper functions
               ✅ DapTransport modes
               ✅ Adapter configuration

Integration    ❌ MISSING: DapClient → send_request() [had this]
Tests          ❌ MISSING: SessionManager → full workflow [needed this!]

E2E Tests      ❌ MISSING: Claude Code → MCP → debugging
```

### 2. Test the Real API Surface

❌ **Don't test**:
```rust
// Internal implementation details
let client = DapClient::from_socket(socket);
client.send_request("initialize", ...).await;
```

✅ **Do test**:
```rust
// Public API that users actually call
let tools_handler = ToolsHandler::new(session_manager);
tools_handler.handle_tool("debugger_start", args).await;
```

### 3. Test State Transitions

❌ **Don't just test**:
- Process spawns ✓
- Socket connects ✓

✅ **Also test**:
- Session initializes ✓
- State changes correctly ✓
- Async operations complete ✓
- Timeouts work ✓

### 4. Test the Whole Path

```
User Action (Claude Code)
    ↓
MCP Tool Call (debugger_start)
    ↓
SessionManager::start_session()
    ↓
DebugSession::new()
    ↓
DebugSession::initialize_and_launch_async()  ← BUGS WERE HERE!
    ↓
DapClient::send_request()  ← We only tested here!
    ↓
DapTransport (socket)
    ↓
rdbg
```

## Recommended Additions

### High Priority

1. **Full Session Lifecycle Test** (critical path)
2. **State Transition Test** (catches stuck states)
3. **Wait for Stop Test** (timeout issues)
4. **Disconnect Timeout Test** (infinite hangs)

### Medium Priority

5. **Breakpoint Workflow Test** (set → continue → hit)
6. **Evaluate Workflow Test** (frameId requirements)
7. **Step Commands Test** (over/into/out)

### Low Priority

8. **Error Recovery Test** (rdbg crashes)
9. **Multiple Sessions Test** (isolation)
10. **Performance Stress Test** (many operations)

## Implementation Plan

### 1. Create `test_ruby_full_workflow.rs`

```rust
/// Full end-to-end Ruby debugging workflow tests
///
/// These tests use the actual SessionManager and ToolsHandler
/// that Claude Code uses, catching integration issues that
/// low-level tests miss.

#[tokio::test]
#[ignore]
async fn test_ruby_full_debugging_workflow() {
    // Complete test as outlined above
}
```

### 2. Update CI/CD

```yaml
# Run high-level integration tests
- name: Ruby Full Workflow Tests
  run: cargo test --test test_ruby_full_workflow -- --ignored
```

### 3. Document in TESTING.md

```markdown
## Test Levels

1. **Unit Tests**: Functions and modules in isolation
2. **Integration Tests**: Components working together
3. **Workflow Tests**: Full user scenarios (SessionManager level) ← NEW!
4. **E2E Tests**: Through Claude Code UI ← Future
```

## The Fix Forward

### Immediate Action

Add `test_ruby_full_workflow.rs` with the critical missing tests:

1. Full session lifecycle
2. State transitions
3. Wait for stop behavior
4. Disconnect timeout
5. Breakpoint workflow

These tests should use:
- ✅ `SessionManager`
- ✅ `ToolsHandler`
- ✅ Real state machine
- ✅ Async event handling
- ✅ Timeout verification

### Why This Will Catch Bugs

The new tests will:
- Use the **same code path** as Claude Code
- Test **state transitions** (where bugs were)
- Verify **async operations** complete
- Check **timeouts** work
- Validate **event handling**

## Conclusion

**The integration tests tested the right things at the wrong level.**

They verified:
✅ Socket infrastructure works
✅ rdbg spawns correctly
✅ Connection succeeds

But they missed:
❌ Session state machine
❌ Async initialization flow
❌ Timeout handling
❌ Event-driven communication

**Solution**: Add workflow-level tests that mirror actual Claude Code usage.

**Impact**: Would have caught all the bugs that manual testing found.

---

**This is a valuable lesson in test design!**
- Low-level tests = necessary but insufficient
- High-level tests = catch real-world integration issues
- Both levels = comprehensive coverage
