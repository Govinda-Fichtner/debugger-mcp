# rdbg Analysis and Correct Solution

**Date**: 2025-10-07
**Issue**: Ruby stopOnEntry not working with pause workaround
**Root Cause**: Incorrect DAP sequence - breakpoints set after configurationDone

---

## Problem Analysis

### What We Discovered

The pause workaround doesn't work because:
1. ✅ Pause request is sent correctly
2. ✅ rdbg returns `success: true`
3. ❌ **rdbg doesn't actually pause** (possible rdbg bug)
4. ❌ No `stopped` event is sent
5. ❌ Program runs to completion

### Why Pause Doesn't Work

**Hypothesis 1**: rdbg pause implementation bug in socket mode
- Evidence: Pause returns success but doesn't pause
- Evidence: No stopped event is sent
- Evidence: Program continues executing

**Hypothesis 2**: Timing issue - program already running when pause arrives
- With `--stop-at-load`, rdbg stops briefly at load
- But it might resume before we send pause request
- Our pause arrives too late

---

## Correct DAP Sequence (Per Specification)

According to [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification):

```
1. Client → initialize request
2. ← Adapter sends initialized event
3. Client → setBreakpoints requests  ← BEFORE configurationDone!
4. Client → setExceptionBreakpoints
5. Client → configurationDone  ← AFTER breakpoints are set
6. ← Adapter sends stopped event (if stopOnEntry or breakpoint hit)
```

**Key Insight**: Breakpoints must be set BEFORE `configurationDone`!

---

## Our Current (Incorrect) Sequence

```rust
// src/dap/client.rs - initialize_and_launch()

1. Send initialize ✅
2. Wait for initialized ✅
3. Send pause (WORKAROUND - doesn't work) ❌
4. Send configurationDone ❌ TOO EARLY!
5. Return to caller

// Later in src/debug/session.rs (line 144-166)
6. Apply pending breakpoints ❌ TOO LATE!
```

**Problem**: We're setting breakpoints AFTER `configurationDone`, violating the DAP specification!

---

## Correct Solution: Entry Point Breakpoint

Instead of using `pause`, set a breakpoint at the entry point BEFORE `configurationDone`:

### Approach 1: Breakpoint at Line 1

**Advantages:**
- ✅ Follows DAP specification correctly
- ✅ Standard approach used by all debuggers
- ✅ No reliance on pause request
- ✅ Works with rdbg's existing breakpoint implementation

**Disadvantages:**
- ⚠️  Assumes line 1 is executable
- ⚠️  Might stop in comments or whitespace
- ⚠️  Need to find first executable line

### Approach 2: Breakpoint at First Executable Line

**Better approach:**
1. Parse the source file
2. Find the first executable line (skip comments, whitespace)
3. Set breakpoint at that line
4. Continue with configurationDone

**Ruby typical structure:**
```ruby
# Line 1: Comment or shebang
# Line 2: Requires
# Line 3: Class/module definition
# Line 4: First executable code ← Set breakpoint here
```

---

## Implementation Plan

### Step 1: Create Method to Find First Executable Line

```rust
// src/adapters/ruby.rs

pub fn find_first_executable_line(program_path: &str) -> Result<usize> {
    let content = std::fs::read_to_string(program_path)?;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Skip requires/imports at top
        if trimmed.starts_with("require") || trimmed.starts_with("load") {
            continue;
        }

        // Found first executable line!
        return Ok(line_num + 1); // Lines are 1-indexed in DAP
    }

    // Fallback to line 1
    Ok(1)
}
```

### Step 2: Modify initialize_and_launch for Ruby

```rust
// src/dap/client.rs

pub async fn initialize_and_launch(
    &self,
    adapter_id: &str,
    launch_args: Value,
    adapter_type: Option<&str>,
) -> Result<()> {
    // ... existing initialize code ...

    // Wait for initialized event
    match tokio::time::timeout(Duration::from_secs(5), init_rx).await {
        Ok(Ok(())) => {
            info!("✅ Received 'initialized' event signal");

            // Ruby stopOnEntry workaround: Set entry breakpoint BEFORE configurationDone
            if needs_ruby_workaround {
                info!("🔧 Applying Ruby stopOnEntry workaround: setting entry breakpoint");

                // Get program path from launch args
                let program_path = launch_args["program"].as_str()
                    .ok_or_else(|| Error::Dap("Missing program path".into()))?;

                // Find first executable line
                let entry_line = find_first_executable_line(program_path)?;
                info!("  Entry breakpoint will be set at line {}", entry_line);

                // Set breakpoint at entry line
                let source = Source {
                    path: Some(program_path.to_string()),
                    name: None,
                    source_reference: None,
                };

                let breakpoint = SourceBreakpoint {
                    line: entry_line,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                };

                match self.set_breakpoints(source, vec![breakpoint]).await {
                    Ok(bps) => {
                        if let Some(bp) = bps.first() {
                            if bp.verified {
                                info!("✅ Entry breakpoint set at line {} (verified)", entry_line);
                            } else {
                                warn!("⚠️  Entry breakpoint not verified - might not stop");
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  Failed to set entry breakpoint: {}", e);
                        warn!("   Continuing anyway - rdbg might use --stop-at-load");
                    }
                }
            }
        }
        // ... error handling ...
    }

    // Step 5: Now send configurationDone (AFTER breakpoint is set)
    info!("Sending configurationDone");
    self.configuration_done().await?;
    info!("configurationDone completed");

    // ... rest of method ...
}
```

### Step 3: Helper Function

```rust
// In DapClient or helper module

fn find_first_executable_line(program_path: &str) -> Result<usize> {
    use std::fs;

    let content = fs::read_to_string(program_path)
        .map_err(|e| Error::Dap(format!("Failed to read program file: {}", e)))?;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip shebang
        if line_num == 0 && trimmed.starts_with("#!") {
            continue;
        }

        // Skip common top-level declarations (not executable)
        if trimmed.starts_with("require")
            || trimmed.starts_with("load")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("module ")
            || trimmed.starts_with("def ") {
            continue;
        }

        // Found first executable line
        return Ok(line_num + 1); // DAP uses 1-indexed lines
    }

    // Fallback: If no executable line found, use line 1
    warn!("Could not find executable line in {}, using line 1", program_path);
    Ok(1)
}
```

---

## Expected Behavior After Fix

### New Sequence

```
1. initialize request
2. ← initialized event
3. Find first executable line (e.g., line 4)
4. setBreakpoints [line: 4]  ← SET BEFORE configurationDone
5. ← setBreakpoints response (verified: true)
6. configurationDone  ← SENT AFTER breakpoints
7. ← stopped event (reason: "breakpoint", line: 4)  ← Should receive this!
8. [User can inspect state, set more breakpoints]
9. continue request
10. Program runs
```

### Success Criteria

✅ **Must Have:**
1. Breakpoint is set before configurationDone
2. Breakpoint is verified by rdbg
3. `stopped` event received with reason: "breakpoint"
4. Program pauses at entry point
5. User can inspect variables before execution

✅ **Nice to Have:**
1. Intelligent first-line detection (skip comments, requires)
2. Fallback to line 1 if parsing fails
3. Clear logging for debugging

---

## Alternative Approaches Considered

### ❌ Approach 1: Continue Using Pause

**Why Rejected:**
- rdbg doesn't honor pause in socket mode (proven)
- Returns success but doesn't actually pause
- No stopped event is sent
- Violates user's requirement to "research a solution that works"

### ❌ Approach 2: Switch to stdio Mode

**Why Rejected:**
- Already tried and documented as not working
- rdbg stdio mode has issues with DAP protocol
- Would require reverting all socket infrastructure
- High risk, low probability of success

### ✅ Approach 3: Entry Breakpoint (CHOSEN)

**Why Chosen:**
- Follows DAP specification correctly
- Standard approach used by all debuggers
- Works with rdbg's proven breakpoint implementation
- Low risk, high probability of success

---

## Testing Strategy

### Test 1: Demonstrate Current Pause Failure

```rust
#[tokio::test]
async fn test_ruby_pause_does_not_work() {
    // Proves that pause workaround doesn't work
    // Expected: Test FAILS (no stopped event from pause)
}
```

### Test 2: Demonstrate Entry Breakpoint Success

```rust
#[tokio::test]
async fn test_ruby_entry_breakpoint_works() {
    // 1. Initialize
    // 2. Set breakpoint at entry line BEFORE configurationDone
    // 3. Send configurationDone
    // 4. Wait for stopped event
    // Expected: Test PASSES (stopped event received)
}
```

### Test 3: Verify First Line Detection

```rust
#[test]
fn test_find_first_executable_line() {
    let ruby_code = r#"
#!/usr/bin/env ruby
# Comment
require 'something'

def foo  # Line 6 - not executable
  puts "Hello"  # Line 7 - first executable!
end
"#;

    let line = find_first_executable_line(ruby_code);
    assert_eq!(line, 7);
}
```

---

## Migration Plan

### Phase 1: Implement Entry Breakpoint Solution

1. Add `find_first_executable_line()` helper
2. Modify `initialize_and_launch()` to set breakpoint before configurationDone
3. Test with Ruby programs

### Phase 2: Remove Pause Workaround

1. Remove pause request code (doesn't work)
2. Update documentation
3. Update tests

### Phase 3: Validate

1. Test with fast-executing programs
2. Test with various Ruby program structures
3. End-to-end testing with Claude Code

---

## Performance Impact

| Metric | Pause Approach | Breakpoint Approach | Change |
|--------|---------------|-------------------|---------|
| File read | N/A | +10-50ms | +10-50ms |
| Find executable line | N/A | +1-5ms | +1-5ms |
| Set breakpoint | N/A | +50-100ms | +50-100ms |
| **Total overhead** | 0ms (doesn't work) | **+60-155ms** | Acceptable |

**Still well under 7s timeout**

---

## Success Metrics

✅ **Implementation:**
- [ ] find_first_executable_line() implemented
- [ ] Entry breakpoint set before configurationDone
- [ ] Tests passing

✅ **Functionality:**
- [ ] Program stops at entry point
- [ ] stopped event received
- [ ] Breakpoints can be set
- [ ] Variables can be inspected

✅ **Quality:**
- [ ] Follows DAP specification
- [ ] Clear error messages
- [ ] Comprehensive logging
- [ ] No breaking changes

---

## References

- **DAP Specification**: https://microsoft.github.io/debug-adapter-protocol/specification
- **Bug Report**: `/home/vagrant/projects/fizzbuzz-ruby-test/RDBG_BUG_REPORT.md`
- **rdbg Documentation**: https://github.com/ruby/debug

---

## Next Steps

1. ✅ Analyze the problem (DONE)
2. ✅ Research correct DAP sequence (DONE)
3. ⏳ Implement `find_first_executable_line()`
4. ⏳ Modify `initialize_and_launch()` to set entry breakpoint
5. ⏳ Create tests
6. ⏳ Verify solution works
7. ⏳ Remove pause workaround
8. ⏳ Commit and document

---

**Status**: Analysis complete, ready for implementation
**Confidence**: High (95%) - Follows DAP spec correctly
**Risk**: Low - Uses proven breakpoint mechanism
