# Getting Started with Debugger MCP

**Target Audience:** AI Agents (like Claude) interacting with the Debugger MCP Server

**Estimated Reading Time:** 5 minutes

---

## What is the Debugger MCP?

The Debugger MCP (Model Context Protocol) server provides debugging capabilities for programs through a standardized interface. It acts as a bridge between AI agents and Debug Adapter Protocol (DAP) debuggers, allowing programmatic debugging of Python, Ruby, and other supported languages.

### Key Concepts

1. **Asynchronous Operations**: Many operations return immediately while work continues in the background
2. **Session-Based**: Each debugging workflow starts by creating a session
3. **State Machine**: Sessions progress through well-defined states (Initializing → Running → Stopped → etc.)
4. **Polling Pattern**: You must poll `debugger_session_state` to detect state changes

---

## Your First Debugging Session

Here's the simplest possible debugging workflow:

### Step 1: Start a Session

```json
Tool: debugger_start
Parameters: {
  "language": "python",
  "program": "/path/to/script.py",
  "stopOnEntry": true
}

Response: {
  "sessionId": "abc-123-def",
  "status": "started"
}
```

**Important:** This returns immediately (< 100ms) but initialization continues in the background.

### Step 2: Wait for Initialization

```json
Tool: debugger_session_state
Parameters: {
  "sessionId": "abc-123-def"
}

Response: {
  "sessionId": "abc-123-def",
  "state": "Initializing",  // Keep polling!
  "details": {}
}
```

**Poll every 50-100ms until state changes to "Stopped"** (typically takes 200-500ms total).

```json
Response (after waiting): {
  "sessionId": "abc-123-def",
  "state": "Stopped",
  "details": {
    "threadId": 1,
    "reason": "entry"  // Stopped at program entry
  }
}
```

### Step 3: Set a Breakpoint

Now that execution is paused, set a breakpoint:

```json
Tool: debugger_set_breakpoint
Parameters: {
  "sessionId": "abc-123-def",
  "sourcePath": "/path/to/script.py",
  "line": 42
}

Response: {
  "verified": true,  // Breakpoint was accepted!
  "sourcePath": "/path/to/script.py",
  "line": 42
}
```

### Step 4: Continue Execution

```json
Tool: debugger_continue
Parameters: {
  "sessionId": "abc-123-def"
}

Response: {
  "status": "continued"
}
```

**Again, poll `debugger_session_state`** to detect when the breakpoint is hit.

### Step 5: Inspect When Stopped

```json
Tool: debugger_stack_trace
Parameters: {
  "sessionId": "abc-123-def"
}

Response: {
  "stackFrames": [
    {
      "id": 0,
      "name": "my_function",
      "source": {"path": "/path/to/script.py"},
      "line": 42,
      "column": 5
    }
  ]
}
```

```json
Tool: debugger_evaluate
Parameters: {
  "sessionId": "abc-123-def",
  "expression": "variable_name"
}

Response: {
  "result": "42"
}
```

### Step 6: Clean Up

```json
Tool: debugger_disconnect
Parameters: {
  "sessionId": "abc-123-def"
}

Response: {
  "status": "disconnected"
}
```

---

## Key Patterns You Must Follow

### 1. Always Use stopOnEntry for Breakpoints

❌ **Wrong:**
```
Start session with stopOnEntry: false
Try to set breakpoint
Breakpoint is missed because program already ran past that line
```

✅ **Correct:**
```
Start session with stopOnEntry: true
Wait for state "Stopped" with reason "entry"
Set breakpoints while paused
Continue execution
```

### 2. Always Poll for State Changes

❌ **Wrong:**
```
Call debugger_start
Immediately call debugger_set_breakpoint
Fails because session not ready yet
```

✅ **Correct:**
```
Call debugger_start
Loop: call debugger_session_state every 50-100ms
Wait until state is NOT "Initializing"
Then proceed with next operation
```

### 3. Check State Before Operations

Different operations require different states:

| Operation | Required State |
|-----------|---------------|
| `debugger_set_breakpoint` | Running or Stopped (Stopped recommended) |
| `debugger_continue` | Stopped |
| `debugger_stack_trace` | Stopped |
| `debugger_evaluate` | Stopped |

**Always call `debugger_session_state` first to verify!**

---

## Common Mistakes

### Mistake 1: Not Waiting for Initialization

```javascript
// BAD: No polling!
let session = debugger_start(...)
debugger_set_breakpoint(session.sessionId, ...)  // FAILS!
```

```javascript
// GOOD: Poll until ready
let session = debugger_start(...)
while (true) {
  let state = debugger_session_state(session.sessionId)
  if (state.state === "Stopped" && state.details.reason === "entry") break
  await sleep(50ms)
}
debugger_set_breakpoint(session.sessionId, ...)  // SUCCESS!
```

### Mistake 2: Forgetting stopOnEntry

```javascript
// BAD: Can't set early breakpoints
debugger_start({
  language: "python",
  program: "script.py",
  stopOnEntry: false  // Program will run immediately!
})
```

```javascript
// GOOD: Pauses at entry
debugger_start({
  language: "python",
  program: "script.py",
  stopOnEntry: true  // Gives you time to set breakpoints
})
```

### Mistake 3: Not Polling After Continue

```javascript
// BAD: No way to know when stopped!
debugger_continue(sessionId)
debugger_stack_trace(sessionId)  // Might fail if still running!
```

```javascript
// GOOD: Poll to detect stop
debugger_continue(sessionId)
while (true) {
  let state = debugger_session_state(sessionId)
  if (state.state === "Stopped") break
  await sleep(50ms)
}
debugger_stack_trace(sessionId)  // SUCCESS!
```

---

## Understanding the Resources

The debugger MCP provides several resources to help you:

### Interactive Resources (JSON)
- `debugger://workflows` - Complete step-by-step workflows with timing info
- `debugger://state-machine` - Full state diagram with all transitions
- `debugger://error-handling` - Error codes and recovery strategies

### Documentation Resources (Markdown)
- `debugger-docs://getting-started` - This guide
- `debugger-docs://guide/async-initialization` - Deep dive on async behavior
- `debugger-docs://troubleshooting` - Common problems and solutions

**Access these via MCP resources/read!**

---

## Quick Reference: Typical Workflow

```
1. debugger_start (stopOnEntry: true)
   ↓ returns immediately
2. Poll debugger_session_state
   ↓ until state = "Stopped", reason = "entry"
3. debugger_set_breakpoint (while stopped)
   ↓ check verified: true
4. debugger_continue
   ↓ returns immediately
5. Poll debugger_session_state
   ↓ until state = "Stopped", reason = "breakpoint"
6. debugger_stack_trace (inspect context)
7. debugger_evaluate (inspect variables)
8. debugger_continue (or debugger_disconnect if done)
```

**Total time:** ~1-2 seconds for typical workflow

---

## Next Steps

- **Try it yourself:** Use this guide to debug a simple "Hello World" program
- **Explore workflows:** Read `debugger://workflows` for more complex scenarios
- **Understand state machine:** Read `debugger://state-machine` to see all possible states
- **Troubleshooting:** If something goes wrong, check `debugger://error-handling`

---

## Prerequisites

### For Python Debugging
```bash
pip install debugpy
```

### For Ruby Debugging
```bash
gem install debug
```

The debugger MCP will automatically detect and use the appropriate DAP adapter for your language.

---

## Support

- **Workflow Examples:** `debugger://workflows`
- **State Reference:** `debugger://state-machine`
- **Error Help:** `debugger://error-handling`
- **Troubleshooting:** `debugger-docs://troubleshooting`
- **Advanced Topics:** `debugger-docs://guide/async-initialization`

**Ready to debug? Start with the workflow above and experiment!**
