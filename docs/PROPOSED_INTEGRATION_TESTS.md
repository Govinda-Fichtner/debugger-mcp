# Proposed Integration Tests Based on User Feedback

## Overview

User testing revealed documentation gaps and edge cases that should have test coverage. These tests would:
1. Verify documented behavior matches reality
2. Catch regressions in critical user workflows
3. Serve as executable documentation

---

## HIGH PRIORITY Tests

### Test 1: frameId Required for Local Variable Access ⭐

**Why**: #1 user pain point - unclear that frameId is practically required

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_evaluate_requires_frameid_for_local_variables() {
    // Setup: Start and stop at breakpoint inside function
    let session = start_and_hit_breakpoint_in_function().await;

    // TEST 1: Without frameId - should fail for local variables
    let eval_without_frame = debugger_evaluate({
        "sessionId": session_id,
        "expression": "local_var"  // Local variable in function
        // NO frameId
    }).await;

    // Should get NameError
    assert!(eval_without_frame.is_err());
    assert!(eval_without_frame.unwrap_err()
        .to_string()
        .contains("NameError"));

    // TEST 2: With frameId - should succeed
    let stack = debugger_stack_trace({session_id}).await.unwrap();
    let frame_id = stack["stackFrames"][0]["id"].as_i64().unwrap();

    let eval_with_frame = debugger_evaluate({
        "sessionId": session_id,
        "expression": "local_var",
        "frameId": frame_id
    }).await;

    // Should succeed
    assert!(eval_with_frame.is_ok());
    assert!(eval_with_frame.unwrap()["result"].is_string());
}
```

**Value**: Documents the frameId requirement with executable proof

---

### Test 2: Frame IDs Change Between Stops ⭐

**Why**: Not documented that frame IDs are unstable across stops

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_frame_ids_change_between_stops() {
    let session = start_session_with_breakpoint().await;

    // Hit breakpoint first time
    debugger_continue({session_id}).await;
    debugger_wait_for_stop({session_id}).await;

    let stack1 = debugger_stack_trace({session_id}).await.unwrap();
    let frame_id_1 = stack1["stackFrames"][0]["id"].as_i64().unwrap();

    // Continue and hit breakpoint second time
    debugger_continue({session_id}).await;
    debugger_wait_for_stop({session_id}).await;

    let stack2 = debugger_stack_trace({session_id}).await.unwrap();
    let frame_id_2 = stack2["stackFrames"][0]["id"].as_i64().unwrap();

    // Frame IDs should be DIFFERENT between stops
    assert_ne!(
        frame_id_1, frame_id_2,
        "Frame IDs should change between stops"
    );

    // Using old frame ID should fail
    let eval_with_old_frame = debugger_evaluate({
        "sessionId": session_id,
        "expression": "n",
        "frameId": frame_id_1  // Old frame ID from first stop
    }).await;

    assert!(eval_with_old_frame.is_err(),
        "Using stale frame ID should fail");
}
```

**Value**: Proves frame IDs are not stable, justifying "always get fresh stack trace" advice

---

### Test 3: debugger_list_breakpoints Functionality

**Why**: New tool with NO test coverage!

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_list_breakpoints_shows_all_breakpoints() {
    let session = start_session().await;

    // Set multiple breakpoints
    debugger_set_breakpoint({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz.py",
        "line": 18
    }).await.unwrap();

    debugger_set_breakpoint({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz.py",
        "line": 20
    }).await.unwrap();

    debugger_set_breakpoint({
        "sessionId": session_id,
        "sourcePath": "/workspace/fizzbuzz.py",
        "line": 31
    }).await.unwrap();

    // List breakpoints
    let bp_list = debugger_list_breakpoints({
        "sessionId": session_id
    }).await.unwrap();

    let breakpoints = bp_list["breakpoints"].as_array().unwrap();

    // Should have all 3 breakpoints
    assert_eq!(breakpoints.len(), 3);

    // Verify details
    let lines: Vec<i64> = breakpoints.iter()
        .map(|bp| bp["line"].as_i64().unwrap())
        .collect();

    assert!(lines.contains(&18));
    assert!(lines.contains(&20));
    assert!(lines.contains(&31));

    // All should be verified
    for bp in breakpoints {
        assert_eq!(bp["verified"].as_bool().unwrap(), true);
    }
}
```

**Value**: Tests new tool that has no coverage

---

## MEDIUM PRIORITY Tests

### Test 4: Complete Debugging Pattern - Inspect Variable at Breakpoint

**Why**: Tests most common user workflow end-to-end

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_pattern_inspect_variable_at_breakpoint() {
    // This test validates the #1 user pattern from feedback

    let session = debugger_start({
        "language": "python",
        "program": "/workspace/fizzbuzz.py",
        "stopOnEntry": true
    }).await.unwrap();

    let session_id = session["sessionId"].as_str().unwrap();

    // Wait for entry
    debugger_wait_for_stop({session_id, "timeoutMs": 5000}).await.unwrap();

    // Set breakpoint inside fizzbuzz function
    debugger_set_breakpoint({
        session_id,
        "sourcePath": "/workspace/fizzbuzz.py",
        "line": 20  // Inside function where 'n' is defined
    }).await.unwrap();

    // Continue to breakpoint
    debugger_continue({session_id}).await.unwrap();
    let stop = debugger_wait_for_stop({session_id}).await.unwrap();

    assert_eq!(stop["reason"].as_str().unwrap(), "breakpoint");

    // Get stack trace (THE RIGHT WAY per feedback)
    let stack = debugger_stack_trace({session_id}).await.unwrap();
    let frames = stack["stackFrames"].as_array().unwrap();
    assert!(!frames.is_empty());

    let frame_id = frames[0]["id"].as_i64().unwrap();

    // Evaluate variable (THE RIGHT WAY with frameId)
    let n_value = debugger_evaluate({
        session_id,
        "expression": "n",
        "frameId": frame_id
    }).await.unwrap();

    // Should successfully get the value
    assert!(n_value["result"].is_string() || n_value["result"].is_number());

    println!("✅ Pattern works: n = {}", n_value["result"]);
}
```

**Value**: Tests the most common user workflow as documented in patterns

---

### Test 5: Step Commands - Complete Workflow

**Why**: New step tools have minimal testing

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_step_commands_comprehensive() {
    let session = start_at_function_call().await;

    // TEST step_into - should enter function
    debugger_step_into({session_id}).await.unwrap();
    debugger_wait_for_stop({session_id}).await.unwrap();

    let stack1 = debugger_stack_trace({session_id}).await.unwrap();
    let frame_name_1 = stack1["stackFrames"][0]["name"].as_str().unwrap();

    // Should be inside called function
    assert_eq!(frame_name_1, "fizzbuzz");

    // TEST step_over - should execute line without entering nested calls
    let line_before = stack1["stackFrames"][0]["line"].as_i64().unwrap();

    debugger_step_over({session_id}).await.unwrap();
    debugger_wait_for_stop({session_id}).await.unwrap();

    let stack2 = debugger_stack_trace({session_id}).await.unwrap();
    let line_after = stack2["stackFrames"][0]["line"].as_i64().unwrap();

    // Should advance to next line
    assert!(line_after > line_before);

    // Should still be in same function
    assert_eq!(
        stack2["stackFrames"][0]["name"].as_str().unwrap(),
        "fizzbuzz"
    );

    // TEST step_out - should return to caller
    debugger_step_out({session_id}).await.unwrap();
    debugger_wait_for_stop({session_id}).await.unwrap();

    let stack3 = debugger_stack_trace({session_id}).await.unwrap();
    let frame_name_3 = stack3["stackFrames"][0]["name"].as_str().unwrap();

    // Should be back in caller (main)
    assert_eq!(frame_name_3, "main");
}
```

**Value**: Comprehensive testing of new step tools

---

### Test 6: wait_for_stop Timing Behavior

**Why**: User feedback highlighted timing as important

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_wait_for_stop_timing_behavior() {
    let session = start_session_stopped_at_entry().await;

    // TEST 1: Immediate return when already stopped
    let start_time = std::time::Instant::now();

    debugger_wait_for_stop({
        session_id,
        "timeoutMs": 5000
    }).await.unwrap();

    let elapsed = start_time.elapsed();

    // Should return in < 100ms (per user feedback)
    assert!(
        elapsed.as_millis() < 100,
        "wait_for_stop should return immediately when already stopped, took {}ms",
        elapsed.as_millis()
    );

    // TEST 2: Blocking behavior when running
    debugger_continue({session_id}).await.unwrap();

    let start_time = std::time::Instant::now();

    // This should block until program completes or hits breakpoint
    let result = debugger_wait_for_stop({
        session_id,
        "timeoutMs": 5000
    }).await;

    let elapsed = start_time.elapsed();

    // Should either succeed (hit breakpoint/terminated) or timeout
    assert!(result.is_ok() || elapsed.as_millis() >= 5000);

    // TEST 3: Timeout behavior
    let session2 = start_infinite_loop().await;  // Program that never stops

    let start_time = std::time::Instant::now();

    let result = debugger_wait_for_stop({
        "sessionId": session2_id,
        "timeoutMs": 1000  // Short timeout
    }).await;

    let elapsed = start_time.elapsed();

    // Should timeout
    assert!(result.is_err());
    assert!(elapsed.as_millis() >= 1000);
    assert!(elapsed.as_millis() < 1500);  // Not too much longer

    // Error message should mention timeout
    assert!(result.unwrap_err().to_string().contains("Timeout"));
}
```

**Value**: Validates timing guarantees mentioned in user feedback

---

### Test 7: Error Messages Accuracy

**Why**: User feedback shows errors need to be clear and helpful

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_error_messages_are_helpful() {
    let session = start_session().await;

    // TEST 1: Stack trace while running
    debugger_continue({session_id}).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;  // Let it run

    let result = debugger_stack_trace({session_id}).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Should mention running state and suggest wait_for_stop
    assert!(error_msg.contains("running"));
    assert!(error_msg.contains("wait_for_stop"));

    // TEST 2: Evaluate while running
    let result = debugger_evaluate({
        session_id,
        "expression": "x"
    }).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Should mention stopped state requirement
    assert!(error_msg.contains("stopped") || error_msg.contains("running"));

    // TEST 3: Evaluate without frameId (documented in feedback)
    debugger_set_breakpoint({session_id, "line": 20}).await.unwrap();
    debugger_continue({session_id}).await.unwrap();
    debugger_wait_for_stop({session_id}).await.unwrap();

    let result = debugger_evaluate({
        session_id,
        "expression": "local_var"  // Local variable
        // No frameId
    }).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Should get NameError
    assert!(error_msg.contains("NameError") || error_msg.contains("not defined"));
}
```

**Value**: Ensures error messages match documentation

---

## LOW PRIORITY Tests

### Test 8: Multiple Breakpoints Different Files

**Why**: Edge case not currently tested

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_breakpoints_across_multiple_files() {
    // Test setting breakpoints in different source files
    // Verify list_breakpoints shows all
    // Verify they all hit correctly
}
```

---

### Test 9: Evaluate Complex Expressions

**Why**: User feedback showed evaluating expressions

**What to Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_evaluate_complex_expressions() {
    // Test evaluating:
    // - Arithmetic: "1 + 1", "n * 2"
    // - Comparisons: "n % 5 == 0"
    // - Function calls: "len(result)"
    // - Object access: "obj.property"
}
```

---

## Test Infrastructure Improvements

### Helper Functions to Add

```rust
// Helper: Start session and hit breakpoint in function
async fn start_and_hit_breakpoint_in_function() -> Session {
    // Returns session stopped at breakpoint inside a function
    // Where local variables are available
}

// Helper: Start session at function call site
async fn start_at_function_call() -> Session {
    // Returns session stopped at line that calls a function
    // For testing step_into
}

// Helper: Start infinite loop program
async fn start_infinite_loop() -> Session {
    // For testing timeout behavior
}
```

---

## Summary Table

| Test | Priority | Coverage Gap | User Impact |
|------|----------|--------------|-------------|
| frameId requirement | HIGH | Documents #1 pain point | Reduces confusion |
| Frame ID stability | HIGH | Undocumented behavior | Explains "get fresh stack" |
| list_breakpoints | HIGH | New tool, 0% coverage | Tests new feature |
| Pattern: inspect variable | MEDIUM | Common workflow | Validates documentation |
| Step commands comprehensive | MEDIUM | Minimal coverage | Tests new tools |
| wait_for_stop timing | MEDIUM | Performance claims | Validates speed |
| Error messages | MEDIUM | User experience | Helpful errors |
| Multi-file breakpoints | LOW | Edge case | Completeness |
| Complex expressions | LOW | Nice to have | Robustness |

---

## Recommendation

**Implement in This Order:**

**Phase 1** (Critical - addresses user feedback):
1. frameId requirement test
2. Frame ID stability test
3. list_breakpoints test

**Phase 2** (Important - validates new features):
4. Pattern: inspect variable test
5. Step commands comprehensive test
6. wait_for_stop timing test

**Phase 3** (Nice to have):
7. Error messages test
8. Multi-file breakpoints test
9. Complex expressions test

---

## Estimated Effort

- Each test: ~30-60 minutes to write
- Phase 1: ~2-3 hours
- Phase 2: ~2-3 hours
- Phase 3: ~1-2 hours

**Total: ~6-8 hours** for complete test coverage addressing all user feedback.

---

## Benefits

1. **Prevents Regressions**: Catch if frameId becomes truly optional
2. **Living Documentation**: Tests prove documented behavior
3. **CI/CD Integration**: Automated verification on every commit
4. **User Confidence**: Demonstrates professional quality
5. **Onboarding**: New contributors see expected behavior in tests
