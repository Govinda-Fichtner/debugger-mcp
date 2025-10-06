# Troubleshooting Guide - Debugger MCP

**Target Audience:** AI Agents encountering issues with the Debugger MCP Server

---

## Quick Diagnosis

Use this flowchart to quickly identify your issue:

```
Is the session stuck in "Initializing" for > 5 seconds?
  ├─ YES → See "Session Won't Initialize"
  └─ NO  → Continue

Is the breakpoint not being hit?
  ├─ YES → See "Breakpoint Not Hitting"
  └─ NO  → Continue

Is debugger_evaluate returning errors?
  ├─ YES → See "Cannot Inspect Variables"
  └─ NO  → Continue

Does the program terminate immediately?
  ├─ YES → See "Program Terminates Too Soon"
  └─ NO  → See "Common Error Messages"
```

---

## Session Won't Initialize

### Symptoms
- State stuck at "Initializing" for > 5 seconds
- Eventually transitions to "Failed"
- Never reaches "Running" or "Stopped"

### Common Causes

#### 1. DAP Adapter Not Installed

**Check:**
```bash
# For Python
python -m debugpy --version

# For Ruby
gem list | grep debug
```

**Solution:**
```bash
# Python
pip install debugpy

# Ruby
gem install debug
```

#### 2. Program Path Not Found

**Check:**
```bash
ls -la /path/to/your/program.py
```

**Solution:**
- Verify the path exists
- Use absolute paths instead of relative
- Check file permissions (must be readable)

#### 3. Invalid Language Parameter

**Error in state details:**
```json
{
  "state": "Failed",
  "details": {
    "error": "Unsupported language: pythn"
  }
}
```

**Solution:**
- Use exact language name: "python" not "pythn" or "Python"
- Currently supported: "python", "ruby"

#### 4. Port Already in Use (rare)

**Symptoms:** DAP adapter fails to start

**Solution:**
```bash
# Find and kill conflicting process
lsof -ti:5678 | xargs kill -9  # debugpy default port
```

### Debugging Steps

1. **Check MCP Server Logs:**
   ```bash
   # Look for ERROR level logs
   grep "ERROR" mcp_server.log
   ```

2. **Test DAP Adapter Directly:**
   ```bash
   # Python
   python -m debugpy --listen 5678 --wait-for-client test.py
   
   # Ruby
   rdbg --open test.rb
   ```

3. **Try Minimal Program:**
   ```python
   # test.py
   print("Hello")
   ```

4. **Verify Permissions:**
   ```bash
   ls -la /path/to/program.py
   # Should show read permissions (r-- or rw-)
   ```

---

## Breakpoint Not Hitting

### Symptoms
- Breakpoint shows `verified: true`
- Program runs but never stops
- State transitions directly from "Running" to "Terminated"

### Common Causes

#### 1. Program Runs Past Breakpoint Before It's Set

**Wrong Approach:**
```javascript
debugger_start({ stopOnEntry: false })  // ❌ Program starts immediately
// By the time you set breakpoint, code already executed
debugger_set_breakpoint({ line: 5 })
```

**Correct Approach:**
```javascript
debugger_start({ stopOnEntry: true })  // ✅ Pauses at first line
// Wait for stopped state
debugger_set_breakpoint({ line: 5 })   // Set while paused
debugger_continue()                     // Now resume
```

#### 2. Breakpoint on Non-Executable Line

**Problem:** Line 5 is a comment or blank

```python
def foo():
    # This is a comment        ← Line 5 (can't break here!)
    x = 42                     ← Line 6 (can break here)
```

**Solution:**
- Set breakpoint on line with actual code (line 6)
- Avoid comments, blank lines, function definitions

#### 3. Source Path Mismatch

**Debugger sees:** `/home/user/project/script.py`
**You set breakpoint on:** `script.py` (relative path)

**Solution:**
- Always use absolute paths in `sourcePath`
- Match exactly what the debugger reports in stack traces

```json
// Check what the debugger sees:
debugger_stack_trace()
// Response shows: "source": {"path": "/home/user/project/script.py"}

// Use that exact path:
debugger_set_breakpoint({
  "sourcePath": "/home/user/project/script.py",  // ✅ Absolute
  "line": 42
})
```

#### 4. Code Never Executes

**Problem:** Breakpoint is in a function that's never called

```python
def unused_function():     # Never called
    x = 42                 # ← Breakpoint here will never hit
```

**Solution:**
- Verify the code path is actually executed
- Add a print statement to confirm
- Set breakpoint earlier in the execution flow

### Verification Steps

1. **Confirm Breakpoint Verified:**
   ```json
   Response should show: "verified": true
   ```

2. **Check Program Actually Runs That Line:**
   ```python
   # Add temporary print to test
   print("DEBUG: About to reach line 42")
   x = 42  # Your breakpoint line
   ```

3. **Use stopOnEntry:**
   ```javascript
   // This guarantees you can set breakpoints before execution
   debugger_start({ stopOnEntry: true })
   ```

---

## Cannot Inspect Variables

### Symptoms
- `debugger_evaluate` returns errors
- Variables show as "undefined" or "not found"
- Expressions cause exceptions

### Common Causes

#### 1. Not in Stopped State

**Error:** "Cannot evaluate while running"

**Check:**
```json
debugger_session_state()
// Must show: "state": "Stopped"
```

**Solution:** Wait for breakpoint to be hit before evaluating

#### 2. Variable Not in Scope

**Problem:** Trying to evaluate local variable from wrong frame

```python
def outer():
    x = 10
    def inner():
        y = 20  # ← Stopped here
        # x is accessible, but z is not
    z = 30
```

**Solution:**
```json
// Get stack trace first
debugger_stack_trace()

// Try different frames
debugger_evaluate({
  "expression": "y",
  "frameId": 0  // Current frame (inner)
})

debugger_evaluate({
  "expression": "x",
  "frameId": 1  // Parent frame (outer)
})
```

#### 3. Invalid Expression Syntax

**Error:** "SyntaxError" or "NameError"

**Problem:** Expression syntax doesn't match language

```javascript
// Python debugger
debugger_evaluate({ expression: "arr.length" })  // ❌ JavaScript syntax

// Should be:
debugger_evaluate({ expression: "len(arr)" })  // ✅ Python syntax
```

**Solution:** Use language-specific syntax

#### 4. Expression Causes Exception

**Problem:** Expression itself throws an error

```python
x = None
# Evaluating: x.property
# Error: AttributeError: 'NoneType' has no attribute 'property'
```

**Solution:**
- Start with simple expressions: just variable names
- Build up complexity: `x`, then `x.prop`, then `x.prop.method()`

### Debugging Steps

1. **Test Simple Expressions First:**
   ```json
   debugger_evaluate({ "expression": "1+1" })
   // If this fails, debugger connection is broken
   ```

2. **Check Variable Exists:**
   ```json
   debugger_evaluate({ "expression": "dir()" })
   // Lists all local variables
   ```

3. **Try Frame 0 Explicitly:**
   ```json
   debugger_evaluate({
     "expression": "variable_name",
     "frameId": 0
   })
   ```

---

## Program Terminates Too Soon

### Symptoms
- State goes: Initializing → Running → Terminated
- Never stops at entry or breakpoints
- Total execution time < 100ms

### Common Causes

#### 1. stopOnEntry Not Set

**Problem:**
```json
debugger_start({
  "stopOnEntry": false  // or omitted
})
// Program runs to completion immediately
```

**Solution:**
```json
debugger_start({
  "stopOnEntry": true  // ✅ Pauses before first line
})
```

#### 2. Program Has No Code

**Problem:** Empty file or only comments/imports

```python
# test.py
import sys
# That's it!
```

**Solution:** Add actual code to execute
```python
import sys
print("Hello")  # ← Something to execute
```

#### 3. Program Exits Immediately

**Problem:** Syntax error or immediate exit

```python
import sys
sys.exit(0)  # ← Exits immediately
```

**Solution:**
- Fix syntax errors
- Remove immediate exits
- Add breakpoints before exit

#### 4. Wrong Interpreter

**Problem:** Program requires Python 3 but using Python 2

**Solution:**
- Verify interpreter version
- Check DAP adapter configuration
- Ensure environment is correct

---

## Common Error Messages

### "Session not found"

**Meaning:** Session ID is invalid or already disconnected

**Causes:**
- Typo in session ID
- Session was disconnected
- Session timed out

**Solution:**
- Verify session ID is correct
- Check session still exists: `debugger://sessions` resource
- Create new session if needed

### "Operation not allowed in current state"

**Meaning:** Trying to do something in wrong state

**Example:**
```json
// State is "Running"
debugger_continue()  // ❌ Can only continue when "Stopped"
```

**Solution:**
- Check state with `debugger_session_state`
- Consult `debugger://state-machine` for valid operations

### "Breakpoint not verified"

**Meaning:** Debugger couldn't set breakpoint

**Causes:**
- Line number invalid (beyond file length)
- Non-executable line (comment, blank)
- Source path doesn't match

**Solution:**
- Use absolute paths
- Check line has executable code
- Verify line number is within file

### "DAP connection lost"

**Meaning:** DAP adapter process crashed or disconnected

**Causes:**
- Debugged program killed the adapter
- Out of memory
- Adapter bug

**Solution:**
- Create new session
- Check system resources
- Try simpler program first

### "Evaluation failed"

**Meaning:** Expression couldn't be evaluated

**See:** "Cannot Inspect Variables" section above

---

## Performance Issues

### Session Initialization Takes Too Long (> 2 seconds)

**Normal:** 200-500ms
**Slow:** > 1 second
**Very Slow:** > 2 seconds

**Causes:**
- System under heavy load
- Large program with many imports
- Network issues (if remote)

**Solutions:**
- Increase polling interval to 100-200ms
- Monitor system resources
- Profile program startup time

### Breakpoint Operations Slow (> 100ms)

**Normal:** 5-20ms
**Slow:** > 50ms

**Causes:**
- Many existing breakpoints
- Large source files
- DAP adapter performance

**Solutions:**
- Remove unneeded breakpoints
- Set breakpoints while stopped
- Update DAP adapter

---

## Best Practices to Avoid Issues

1. **Always use stopOnEntry: true** for breakpoint-based debugging
2. **Always poll state** before operations
3. **Use absolute paths** for source files
4. **Check verified: true** after setting breakpoints
5. **Test with minimal programs** first
6. **Monitor state transitions** with logs
7. **Implement timeouts** in polling loops (5-10 seconds)
8. **Call debugger_disconnect** to clean up resources

---

## Still Having Issues?

1. **Check Resources:**
   - `debugger://workflows` - Correct usage patterns
   - `debugger://state-machine` - Valid state transitions
   - `debugger://error-handling` - Error codes and recovery

2. **Verify Prerequisites:**
   - DAP adapter installed (`debugpy` or `debug` gem)
   - Program file exists and is readable
   - Permissions are correct

3. **Test Components:**
   - Test DAP adapter directly (without MCP)
   - Test with minimal program
   - Check MCP server logs

4. **Common Quick Fixes:**
   - Restart session
   - Use absolute paths
   - Enable stopOnEntry
   - Wait for state transitions

---

**Remember:** Most issues are caused by not waiting for state transitions or forgetting stopOnEntry!
