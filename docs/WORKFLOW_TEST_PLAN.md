# Workflow-Level Test Plan for Ruby Debugging

**Date**: 2025-10-07
**Status**: 📋 Planned (Implementation pending)

## Background

Low-level socket tests (test_ruby_socket_adapter.rs) all pass ✅, but manual testing revealed bugs in the high-level workflow. See `TEST_COVERAGE_GAP_ANALYSIS.md` for details.

## The Gap

### What We Test Now ✅
- Socket infrastructure (DapTransport, socket_helper)
- DAP protocol communication (DapClient)
- rdbg spawning and connection

### What We DON'T Test ⚠️
- SessionManager::create_session()
- DebugSession state machine
- Async initialization flow
- Event coordination
- Timeout handling in workflows
- Real MCP tool handlers

## Required Tests

### Test File: `tests/test_ruby_workflow.rs`

#### 1. Full Session Lifecycle Test (CRITICAL)
**Would have caught the "stuck in Initializing" bug!**

```rust
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_full_session_lifecycle() {
    let manager = SessionManager::new();

    // Create session (via SessionManager, like MCP does)
    let session_id = manager.create_session(
        "ruby",
        "/tmp/test.rb".to_string(),
        vec![],
        Some("/tmp".to_string()),
        true // stop_on_entry
    ).await.expect("Failed to create session");

    // Wait for state transition
    let session = manager.get_session(&session_id).await.unwrap();

    // THIS check would have caught the bug!
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let state = session.get_state().await;
            if matches!(state, DebugState::Stopped(_)) {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    }).await.expect("Session stuck in Initializing!");

    // Verify we can interact
    // ... continue, evaluate, etc.
}
```

#### 2. State Transition Verification
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_state_transitions() {
    // Track all state transitions
    // Verify: Initializing → Stopped → Running → Exited
    // Check timing and order
}
```

#### 3. Breakpoint Workflow
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_breakpoint_workflow() {
    // 1. Create session
    // 2. Set breakpoint via DebugSession
    // 3. Continue
    // 4. Verify stopped at breakpoint
    // 5. Inspect variables
    // 6. Continue to completion
}
```

#### 4. Variable Evaluation
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_variable_evaluation() {
    // Test variable evaluation through full stack
    // SessionManager → DebugSession → DapClient
}
```

#### 5. Step Commands
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_step_commands() {
    // stepIn, stepOver, stepOut
    // Through DebugSession API
}
```

#### 6. Timeout Behavior
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_timeout_handling() {
    // Verify timeouts actually work
    // Initialize timeout (2s)
    // Disconnect timeout (2s)
}
```

#### 7. Error Handling
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_error_handling() {
    // Invalid program
    // Spawn failures
    // Connection failures
}
```

#### 8. Multiple Sessions
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_multiple_sessions() {
    // Concurrent sessions
    // Unique session IDs
    // Independent states
}
```

#### 9. Performance
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_session_performance() {
    // Startup time < 3s
    // Disconnect time < 2s
}
```

## Implementation Notes

### API Differences Found

The test draft used incorrect APIs. Correct APIs are:

```rust
// SessionManager
manager.create_session(language, program, args, cwd, stop_on_entry) → session_id
manager.get_session(session_id) → Arc<DebugSession>
manager.get_session_state(session_id) → DebugState
manager.remove_session(session_id)

// DebugSession
session.get_state() → DebugState
session.initialize_and_launch_async()
session.set_breakpoints(...)
session.continue_execution()
session.step_over(thread_id)
session.step_in(thread_id)
session.disconnect()
```

### Test Structure Pattern

```rust
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_name() {
    // 1. Setup - create test file
    // 2. Create SessionManager
    // 3. Create session via SessionManager
    // 4. Get DebugSession from manager
    // 5. Test workflow through DebugSession
    // 6. Verify state and results
    // 7. Cleanup
}
```

### Running Tests

```bash
# All workflow tests
cargo test --test test_ruby_workflow -- --ignored

# Specific test
cargo test --test test_ruby_workflow test_ruby_full_session_lifecycle -- --ignored

# In Docker
docker run --rm -v $(pwd):/app -w /app rust:1.83-alpine sh -c '
  apk add --no-cache musl-dev ruby ruby-dev make g++ &&
  gem install debug --no-document &&
  cargo test --test test_ruby_workflow -- --ignored
'
```

## Why These Tests Matter

### Current Situation
- ✅ Low-level tests pass
- ❌ High-level workflow has bugs
- ❌ Manual testing finds issues

### With Workflow Tests
- ✅ Low-level tests pass
- ✅ High-level workflow tests pass
- ✅ Bugs caught before manual testing

## Priority

**HIGH** - These tests would have caught the actual bugs we encountered in production:
1. "Stuck in Initializing" → Test 1 would catch this
2. State machine issues → Test 2 would catch this
3. Event coordination problems → Tests 1, 3, 4 would catch this

## Next Steps

1. **Fix the test file** (`tests/test_ruby_full_workflow.rs`):
   - Use correct SessionManager API
   - Use correct DebugSession API
   - Verify compilation

2. **Run tests** in Docker with rdbg:
   ```bash
   cargo test --test test_ruby_workflow -- --ignored
   ```

3. **Iterate** on any failures:
   - Update implementation if needed
   - Update tests if API understanding was wrong

4. **Document results** - Create test verification report

## Success Criteria

All 9 workflow tests should pass, proving:
- ✅ Session creation works
- ✅ State transitions work
- ✅ Breakpoints work
- ✅ Variable evaluation works
- ✅ Step commands work
- ✅ Timeouts work
- ✅ Error handling works
- ✅ Multiple sessions work
- ✅ Performance meets requirements

## Current Status

- **test_ruby_socket_adapter.rs**: ✅ All 6 tests passing
- **test_ruby_workflow.rs**: ⏳ Created but needs API fixes
- **Integration**: ⏳ Pending workflow test fixes

---

**Created**: 2025-10-07
**Requires**: Ruby debug gem (rdbg), proper SessionManager/DebugSession API usage
