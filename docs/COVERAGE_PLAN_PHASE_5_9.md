# Test Coverage Improvement Plan - Phases 5-9

## Current Status
- **Coverage**: 62.91% (346/550 lines)
- **Tests**: 83 passing
- **Target**: 75-80% coverage

## Goal
Increase coverage from 62.91% to 75%+ by adding targeted unit tests for uncovered code paths.

---

## Coverage Gap Analysis

### High-Value Targets (Easy Wins)

#### 1. mcp/transport.rs - 0/30 lines (0%) ❌
**Why untested**: Production read/write implementation never executed in tests (we only test mocks)
**Opportunity**: Add real transport tests with in-memory pipes
**Expected gain**: +5% coverage
**Difficulty**: Easy - can use tokio::io::duplex()

#### 2. mcp/tools/mod.rs - 96/129 lines (74%) ⚠️
**Uncovered**: Lines 76-79, 87-88, 90-91, 93, 95, 104-150 (error paths, edge cases)
**Why untested**: Missing error case tests
**Expected gain**: +6% coverage
**Difficulty**: Easy - add tests for invalid arguments, missing fields

#### 3. debug/session.rs - 18/66 lines (27%) ⚠️
**Uncovered**: Lines 51-139 (launch, set_breakpoint, continue, stack_trace, evaluate, disconnect)
**Why untested**: Complex async methods removed in Phase 3 due to timeout issues
**Expected gain**: +9% coverage
**Difficulty**: Medium - need better mock coordination

#### 4. debug/manager.rs - 17/33 lines (51%) ⚠️
**Uncovered**: Lines 30-61, 88 (create_session, error paths)
**Why untested**: Only error paths tested, not success paths
**Expected gain**: +3% coverage
**Difficulty**: Easy - mock adapter paths

#### 5. mcp/mod.rs - 9/20 lines (45%) ⚠️
**Uncovered**: Lines 37-51 (server run loop)
**Why untested**: Server startup and main loop
**Expected gain**: +2% coverage
**Difficulty**: Medium - async server testing

### Low-Value Targets (Skip)

- **dap/transport.rs** - 0/30 (0%) - I/O layer, tested via mocks ✓ Acceptable
- **main.rs** - 0/12 (0%) - Entry point ✓ Acceptable
- **lib.rs** - 0/3 (0%) - Module exports ✓ Acceptable
- **dap/client.rs** - 78/102 (76%) - Already well-tested ✓ Good enough

---

## Proposed Phases

### Phase 5: MCP Transport Real Implementation Tests ✅
**Goal**: Test actual MCP transport read/write (not just mocks)
**Target**: mcp/transport.rs 0% → 80%
**Expected gain**: +5% total coverage

**Approach**:
1. Use `tokio::io::duplex()` to create in-memory bidirectional pipe
2. Test read_message() with real line-based input
3. Test write_message() and verify newline-terminated output
4. Test error cases (EOF, empty lines, invalid JSON)

**Tests to add** (~6 tests):
- `test_real_transport_read_single_message`
- `test_real_transport_write_single_message`
- `test_real_transport_read_multiple_messages`
- `test_real_transport_error_eof`
- `test_real_transport_error_empty_line`
- `test_real_transport_error_invalid_json`

**Estimated time**: 1 hour
**Risk**: Low

---

### Phase 6: MCP Tools Error Path Tests ✅
**Goal**: Test error handling in mcp/tools/mod.rs
**Target**: mcp/tools/mod.rs 74% → 90%
**Expected gain**: +4% total coverage

**Approach**:
1. Test invalid tool arguments (missing fields, wrong types)
2. Test tool handler error paths
3. Test edge cases in argument deserialization

**Tests to add** (~8 tests):
- `test_debugger_start_missing_program`
- `test_debugger_start_invalid_args_type`
- `test_set_breakpoint_missing_source`
- `test_set_breakpoint_invalid_line`
- `test_continue_invalid_session_id`
- `test_stack_trace_invalid_session_id`
- `test_evaluate_missing_expression`
- `test_disconnect_invalid_session_id`

**Estimated time**: 1 hour
**Risk**: Low

---

### Phase 7: Debug Session Lifecycle Tests 🔶
**Goal**: Test session methods that were deferred in Phase 3
**Target**: debug/session.rs 27% → 70%
**Expected gain**: +6% total coverage

**Approach**:
1. Use simplified mocks with better coordination
2. Test each method independently (not full lifecycle)
3. Focus on state transitions, not full protocol

**Tests to add** (~8 tests):
- `test_session_launch_updates_state`
- `test_session_set_breakpoint_success`
- `test_session_set_breakpoint_failure`
- `test_session_continue_with_thread`
- `test_session_stack_trace_success`
- `test_session_evaluate_success`
- `test_session_disconnect_updates_state`
- `test_session_error_propagation`

**Estimated time**: 2 hours
**Risk**: Medium (async coordination can be tricky)

**Alternative if blocked**: Skip to Phase 8-9, come back if time permits

---

### Phase 8: Debug Manager Tests ✅
**Goal**: Test session creation success paths
**Target**: debug/manager.rs 51% → 80%
**Expected gain**: +2% total coverage

**Approach**:
1. Mock adapter command/args via environment variables
2. Test successful session creation
3. Test adapter not found error

**Tests to add** (~4 tests):
- `test_create_session_python_success`
- `test_create_session_stores_in_map`
- `test_create_session_returns_valid_id`
- `test_create_session_language_not_found`

**Estimated time**: 30 minutes
**Risk**: Low

---

### Phase 9: MCP Server Initialization Tests 🔶
**Goal**: Test MCP server startup
**Target**: mcp/mod.rs 45% → 70%
**Expected gain**: +1.5% total coverage

**Approach**:
1. Test handler initialization
2. Test message dispatch logic
3. Mock transport for server testing

**Tests to add** (~3 tests):
- `test_mcp_server_initializes`
- `test_mcp_server_handles_request`
- `test_mcp_server_error_handling`

**Estimated time**: 1 hour
**Risk**: Medium (async server coordination)

**Alternative if blocked**: Skip, as this is low value (+1.5%)

---

## Execution Strategy

### Prioritized Order

1. **Phase 5** (MCP Transport) - Easy, high value ✅
2. **Phase 6** (MCP Tools) - Easy, high value ✅
3. **Phase 8** (Manager) - Easy, medium value ✅
4. **Phase 7** (Session) - Medium, high value 🔶
5. **Phase 9** (MCP Server) - Medium, low value 🔶

### Stop Conditions

**Stop at 75% coverage** - Good for async I/O projects
- If Phase 5+6+8 get us to 75%, declare victory
- Phase 7 and 9 are optional stretch goals

**Stop at 2 hours** - Diminishing returns
- Don't spend more than 2 hours chasing last few percentage points
- Focus on high-value, easy wins

### Risk Mitigation

**If Phase 7 or 9 get stuck**:
- Don't spend more than 30 minutes debugging
- Move to next phase or declare current coverage sufficient
- Document why those areas are hard to test

---

## Expected Outcomes

### Conservative Estimate
- Phase 5: +5% (68%)
- Phase 6: +4% (72%)
- Phase 8: +2% (74%)
- **Total: 74% coverage** ✅

### Optimistic Estimate
- Phase 5: +5% (68%)
- Phase 6: +4% (72%)
- Phase 8: +2% (74%)
- Phase 7: +6% (80%)
- Phase 9: +1% (81%)
- **Total: 81% coverage** 🎯

### Realistic Target
**75% coverage** - Excellent for async I/O project

---

## Success Criteria

- ✅ Coverage ≥ 75%
- ✅ All tests pass
- ✅ No flaky tests
- ✅ Tests run in < 5 seconds
- ✅ Clear, maintainable test code

---

## Timeline

- Phase 5: 1 hour
- Phase 6: 1 hour
- Phase 8: 30 minutes
- Phase 7: 2 hours (optional)
- Phase 9: 1 hour (optional)

**Total: 2.5 - 5.5 hours** depending on how far we go

---

## Implementation Notes

### Phase 5 - Real Transport Testing

```rust
#[tokio::test]
async fn test_real_transport_read_write() {
    let (client, server) = tokio::io::duplex(1024);
    let (read, write) = tokio::io::split(server);

    let mut transport = StdioTransport {
        stdin: BufReader::new(read),
        stdout: write,
    };

    // Write test
    let msg = JsonRpcMessage::Request(...);
    transport.write_message(&msg).await.unwrap();

    // Read test (from client side)
    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    assert!(buf.ends_with('\n'));
}
```

### Phase 6 - Error Path Testing

```rust
#[tokio::test]
async fn test_debugger_start_missing_program() {
    let manager = SessionManager::new();
    let handler = ToolsHandler::new(manager);

    let args = json!({
        // Missing "program" field
        "language": "python",
        "args": []
    });

    let result = handler.handle_debugger_start(args).await;
    assert!(result.is_err());
}
```

### Phase 7 - Session Testing

```rust
#[tokio::test]
async fn test_session_launch_simplified() {
    let mut mock = MockDapClient::new();
    mock.expect_launch().returning(|_| Ok(()));

    let session = DebugSession::new_with_client("python", "test.py", mock);
    session.launch(json!({"program": "test.py"})).await.unwrap();

    assert_eq!(session.get_state().await, DebugState::Running);
}
```

---

## Conclusion

This plan focuses on **high-value, low-risk improvements** that can realistically get us to **75% coverage** in **2.5 hours**.

The approach is pragmatic:
- Easy wins first (Phase 5, 6, 8)
- Optional stretch goals (Phase 7, 9)
- Stop at 75% (excellent for async I/O)
- Avoid getting stuck on hard-to-test areas

**Recommended**: Execute Phases 5, 6, 8 and call it done at ~74% coverage.
