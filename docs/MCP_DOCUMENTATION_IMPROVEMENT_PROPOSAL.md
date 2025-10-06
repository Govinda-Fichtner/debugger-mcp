# MCP Documentation & Discoverability Improvement Proposal

**Date**: 2025-10-06
**Status**: Proposal for Review
**Priority**: HIGH - Improves Claude Code Integration

## Executive Summary

This proposal enhances the Debugger MCP server's self-documentation to help AI clients like Claude Code understand:

1. **What the server does** - Core purpose and capabilities
2. **How to use it** - Standard workflows and usage patterns
3. **When to use which tools** - Tool sequencing and dependencies
4. **What to expect** - State transitions and expected behaviors
5. **How to handle errors** - Common issues and recovery strategies

## Current State Analysis

### What We Provide Today

**Tools (7):**
- Basic `description` field
- JSON Schema for `inputSchema`
- Field-level descriptions
- Required field markers

**Resources (3 templates):**
- URI templates
- Basic descriptions
- MIME types

### What's Missing

1. **No workflow guidance** - Tools are documented in isolation
2. **No sequencing information** - Which tool to call after which
3. **No state transition documentation** - When tools succeed/fail
4. **No examples** - Abstract descriptions without concrete usage
5. **No error handling guidance** - What errors mean and how to recover
6. **No timing information** - Async operations not clearly marked
7. **No capability discovery** - Client doesn't know what's possible

## Proposal: Five-Tier Documentation Enhancement

### Tier 1: Enhanced Tool Descriptions ✨

**Add detailed, workflow-aware descriptions that include:**

```json
{
  "name": "debugger_start",
  "title": "Start Debugging Session",
  "description": "Starts a new debugging session for a program. Returns immediately with a sessionId while initialization happens in the background. IMPORTANT: After calling this, you MUST poll debugger_session_state until the state is 'Running' or 'Stopped' before setting breakpoints or continuing execution. Use stopOnEntry: true to pause at the first line of code.",
  "inputSchema": { ... },
  "annotations": {
    "async": true,
    "workflow": "initialization",
    "nextSteps": ["debugger_session_state"],
    "timing": "Returns immediately (< 100ms), initialization continues in background"
  }
}
```

**Reasoning:**
- Claude needs to understand async operations
- Explicit guidance on "what to do next"
- Timing information sets expectations
- Annotations provide machine-readable metadata

### Tier 2: Add Usage Examples 📚

**Provide concrete examples in tool descriptions:**

```json
{
  "name": "debugger_set_breakpoint",
  "description": "Sets a breakpoint in a source file at a specific line number. Can be called during initialization - breakpoints will be stored as pending and applied automatically when the session is ready.",
  "examples": [
    {
      "scenario": "Set breakpoint during initialization",
      "input": {
        "sessionId": "abc-123",
        "sourcePath": "/path/to/script.py",
        "line": 42
      },
      "output": {
        "verified": true,
        "sourcePath": "/path/to/script.py",
        "line": 42
      },
      "notes": "Returns verified: true immediately. Actual breakpoint applied after initialization."
    },
    {
      "scenario": "Set breakpoint when already running",
      "input": {
        "sessionId": "abc-123",
        "sourcePath": "/path/to/script.py",
        "line": 42
      },
      "output": {
        "verified": true,
        "sourcePath": "/path/to/script.py",
        "line": 42
      },
      "notes": "Breakpoint is set immediately via DAP protocol."
    }
  ]
}
```

**Reasoning:**
- Examples are concrete and immediately actionable
- Shows different scenarios (edge cases)
- Demonstrates expected input/output format
- Reduces trial-and-error

### Tier 3: Workflow Documentation Resource 🔄

**Add a new resource: `debugger://workflows`**

**Content:**
```json
{
  "workflows": [
    {
      "name": "basic-debugging",
      "description": "Standard debugging workflow for finding and fixing bugs",
      "steps": [
        {
          "order": 1,
          "tool": "debugger_start",
          "parameters": {
            "language": "python",
            "program": "/path/to/script.py",
            "stopOnEntry": true
          },
          "expectedResult": {
            "sessionId": "<uuid>",
            "status": "started"
          },
          "notes": "Returns immediately. Session initializes in background."
        },
        {
          "order": 2,
          "tool": "debugger_session_state",
          "parameters": {
            "sessionId": "<from_step_1>"
          },
          "expectedResult": {
            "state": "Initializing"
          },
          "notes": "Poll every 100ms until state is 'Running' or 'Stopped'",
          "loop": {
            "condition": "state === 'Initializing'",
            "delay": 100
          }
        },
        {
          "order": 3,
          "tool": "debugger_set_breakpoint",
          "parameters": {
            "sessionId": "<from_step_1>",
            "sourcePath": "/path/to/script.py",
            "line": 42
          },
          "expectedResult": {
            "verified": true
          },
          "notes": "Can be called during initialization. Breakpoint applied automatically."
        },
        {
          "order": 4,
          "tool": "debugger_continue",
          "parameters": {
            "sessionId": "<from_step_1>"
          },
          "expectedResult": {
            "status": "continued"
          },
          "notes": "Program continues until breakpoint hit or completion"
        },
        {
          "order": 5,
          "tool": "debugger_session_state",
          "parameters": {
            "sessionId": "<from_step_1>"
          },
          "expectedResult": {
            "state": "Stopped",
            "details": {
              "reason": "breakpoint",
              "threadId": 1
            }
          },
          "notes": "Poll until stopped at breakpoint"
        },
        {
          "order": 6,
          "tool": "debugger_stack_trace",
          "parameters": {
            "sessionId": "<from_step_1>"
          },
          "expectedResult": {
            "stackFrames": [...]
          },
          "notes": "Examine call stack to understand program state"
        },
        {
          "order": 7,
          "tool": "debugger_evaluate",
          "parameters": {
            "sessionId": "<from_step_1>",
            "expression": "variable_name"
          },
          "expectedResult": {
            "result": "..."
          },
          "notes": "Inspect variable values at breakpoint"
        },
        {
          "order": 8,
          "tool": "debugger_disconnect",
          "parameters": {
            "sessionId": "<from_step_1>"
          },
          "expectedResult": {
            "status": "disconnected"
          },
          "notes": "Clean up session when done"
        }
      ],
      "estimatedDuration": "5-30 seconds depending on program",
      "prerequisites": [
        "Program file must exist and be accessible",
        "Python interpreter must be available (for Python programs)"
      ]
    },
    {
      "name": "quick-inspection",
      "description": "Quick inspection without breakpoints (just run and see output)",
      "steps": [...]
    },
    {
      "name": "multiple-breakpoints",
      "description": "Set multiple breakpoints and step through code",
      "steps": [...]
    }
  ]
}
```

**Reasoning:**
- Provides end-to-end guidance
- Shows complete workflows, not just individual tools
- Includes timing and looping patterns
- Documents prerequisites and expectations
- Multiple workflows show flexibility

### Tier 4: State Transition Documentation 🔄

**Add a new resource: `debugger://state-machine`**

**Content:**
```json
{
  "states": {
    "NotStarted": {
      "description": "Session created but not yet initialized",
      "validTransitions": ["Initializing"],
      "validOperations": ["debugger_set_breakpoint"],
      "notes": "Breakpoints set in this state are stored as pending"
    },
    "Initializing": {
      "description": "DAP adapter starting, program launching",
      "validTransitions": ["Running", "Stopped", "Failed"],
      "validOperations": ["debugger_set_breakpoint", "debugger_session_state"],
      "notes": "Poll debugger_session_state to know when initialization completes",
      "typicalDuration": "200-500ms"
    },
    "Running": {
      "description": "Program executing normally",
      "validTransitions": ["Stopped", "Terminated"],
      "validOperations": [
        "debugger_set_breakpoint",
        "debugger_session_state",
        "debugger_disconnect"
      ],
      "notes": "Program will run until breakpoint, step, or completion"
    },
    "Stopped": {
      "description": "Program paused (breakpoint, entry, step, etc.)",
      "validTransitions": ["Running", "Terminated"],
      "validOperations": [
        "debugger_set_breakpoint",
        "debugger_continue",
        "debugger_stack_trace",
        "debugger_evaluate",
        "debugger_session_state",
        "debugger_disconnect"
      ],
      "stopReasons": [
        "entry - stopOnEntry was true",
        "breakpoint - Hit a breakpoint",
        "step - Completed a step operation",
        "exception - Uncaught exception"
      ],
      "notes": "Most operations available when stopped"
    },
    "Terminated": {
      "description": "Program exited, debugging complete",
      "validTransitions": [],
      "validOperations": ["debugger_disconnect"],
      "notes": "Session should be disconnected to clean up resources"
    },
    "Failed": {
      "description": "Initialization or execution error",
      "validTransitions": [],
      "validOperations": ["debugger_disconnect"],
      "errorDetails": "Check details.error for failure reason",
      "commonCauses": [
        "Program file not found",
        "Syntax error in program",
        "DAP adapter not available",
        "Permission denied"
      ]
    }
  },
  "diagram": "NotStarted → Initializing → Running ⇄ Stopped → Terminated"
}
```

**Reasoning:**
- Makes state machine explicit
- Shows valid operations per state
- Prevents invalid tool calls
- Documents error states
- Helps with error recovery

### Tier 5: Error Handling Guide 🚨

**Add a new resource: `debugger://error-handling`**

**Content:**
```json
{
  "commonErrors": [
    {
      "error": "SessionNotFound",
      "code": -32001,
      "causes": [
        "Session ID is invalid or misspelled",
        "Session was already disconnected",
        "Session timed out"
      ],
      "recovery": [
        "Verify session ID is correct",
        "Check debugger://sessions to see active sessions",
        "Start a new session with debugger_start"
      ],
      "example": {
        "error": {
          "code": -32001,
          "message": "Session not found: abc-123"
        }
      }
    },
    {
      "error": "InvalidState",
      "code": -32005,
      "causes": [
        "Called debugger_continue on a Running session",
        "Called debugger_stack_trace on a Running session",
        "Called operations on Terminated session"
      ],
      "recovery": [
        "Poll debugger_session_state to check current state",
        "Wait for session to reach appropriate state",
        "See debugger://state-machine for valid operations per state"
      ],
      "prevention": "Always check session state before operations"
    },
    {
      "error": "DAP initialization timeout",
      "symptoms": [
        "debugger_session_state shows 'Initializing' for > 5 seconds",
        "Session transitions to 'Failed' state"
      ],
      "causes": [
        "DAP adapter (debugpy) not installed",
        "Program has syntax errors",
        "Program requires unavailable dependencies"
      ],
      "recovery": [
        "Check error details in Failed state",
        "Verify program runs outside debugger",
        "Check DAP adapter availability"
      ]
    },
    {
      "error": "Breakpoint not hit",
      "symptoms": [
        "Program runs to completion without stopping",
        "debugger_session_state shows 'Terminated' instead of 'Stopped'"
      ],
      "causes": [
        "Breakpoint line never executed (conditional code)",
        "Breakpoint set on wrong file path",
        "Line number incorrect (e.g., comment or blank line)"
      ],
      "debugging": [
        "Use stopOnEntry: true to verify program starts",
        "Set breakpoint on early line (e.g., line 1)",
        "Check sourcePath matches program path exactly"
      ]
    }
  ],
  "bestPractices": [
    {
      "practice": "Always use stopOnEntry: true for initial debugging",
      "reason": "Ensures program pauses before running, allows setting breakpoints safely"
    },
    {
      "practice": "Poll debugger_session_state with 100ms interval",
      "reason": "Balance between responsiveness and API efficiency"
    },
    {
      "practice": "Always call debugger_disconnect when done",
      "reason": "Cleans up resources and terminates child processes"
    },
    {
      "practice": "Check session state before tool calls",
      "reason": "Prevents InvalidState errors"
    }
  ]
}
```

**Reasoning:**
- Documents real issues users face
- Provides actionable recovery steps
- Prevents common mistakes
- Shows debugging workflows
- Includes best practices

## Implementation Plan

### Phase 1: Enhanced Tool Descriptions (2 hours)

**File:** `src/mcp/tools/mod.rs`

1. Add comprehensive descriptions with workflow context
2. Add JSON Schema annotations for async/timing
3. Include "see also" references to related tools

**Example implementation:**

```rust
json!({
    "name": "debugger_start",
    "title": "Start Debugging Session",
    "description": "Starts a new debugging session for a program. RETURNS IMMEDIATELY with a sessionId while initialization happens in the background.\n\nIMPORTANT WORKFLOW:\n1. Call this tool first\n2. Poll debugger_session_state until state is 'Running' or 'Stopped'\n3. Then set breakpoints with debugger_set_breakpoint\n4. Continue with debugger_continue\n\nTIP: Use stopOnEntry: true to pause at the first line of code, giving you time to set breakpoints.\n\nSEE ALSO: debugger_session_state (required next step), debugger://workflows (complete examples)",
    "inputSchema": {
        "type": "object",
        "properties": {
            "language": {
                "type": "string",
                "description": "Programming language (currently supported: 'python')",
                "enum": ["python"]
            },
            "program": {
                "type": "string",
                "description": "Absolute path to the program file to debug (must exist and be readable)"
            },
            "args": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Command-line arguments to pass to the program (optional)"
            },
            "cwd": {
                "type": "string",
                "description": "Working directory for the program (defaults to program's directory)"
            },
            "stopOnEntry": {
                "type": "boolean",
                "description": "If true, pauses execution at the first line of code. RECOMMENDED for first-time debugging. Defaults to false.",
                "default": false
            }
        },
        "required": ["language", "program"]
    },
    "annotations": {
        "async": true,
        "returnsTiming": "< 100ms",
        "completionTiming": "200-500ms (background)",
        "workflow": "initialization",
        "requiredFollowUp": ["debugger_session_state"],
        "category": "session-management"
    }
})
```

### Phase 2: Workflow Resource (3 hours)

**File:** `src/mcp/resources/mod.rs`

1. Add `WorkflowsHandler` struct
2. Implement `debugger://workflows` resource
3. Define 3-5 common workflows
4. Include timing and looping patterns

### Phase 3: State Machine Resource (2 hours)

**File:** `src/mcp/resources/mod.rs`

1. Add `StateMachineHandler`
2. Implement `debugger://state-machine` resource
3. Document all states and transitions
4. Map tools to valid states

### Phase 4: Error Handling Resource (2 hours)

**File:** `src/mcp/resources/mod.rs`

1. Add `ErrorGuideHandler`
2. Implement `debugger://error-handling` resource
3. Document all error codes
4. Provide recovery strategies

### Phase 5: Integration Testing (2 hours)

1. Update Claude Code integration test to use workflow resource
2. Verify Claude can discover and use workflows
3. Test error handling documentation
4. Validate state machine helps prevent errors

## Expected Benefits

### For AI Clients (Claude Code)

1. **Reduced Trial-and-Error**: Clear workflows eliminate guessing
2. **Better Error Recovery**: Explicit error handling reduces failures
3. **Faster Learning**: Examples and workflows are immediately actionable
4. **State Awareness**: State machine prevents invalid operations
5. **Improved User Experience**: Fewer errors = happier users

### For Human Developers

1. **Self-Documenting API**: Resources serve as API documentation
2. **Onboarding**: New users understand capabilities quickly
3. **Debugging**: Error guide helps troubleshoot issues
4. **Reference**: Workflows show best practices

### Measurable Improvements

**Before Enhancement:**
- Claude needs to guess tool order: ❌ High error rate
- No understanding of async operations: ❌ Timeouts
- Invalid state transitions: ❌ Frequent errors
- No error recovery guidance: ❌ Gives up easily

**After Enhancement:**
- Claude follows documented workflows: ✅ Low error rate
- Understands async with polling: ✅ No timeouts
- Respects state machine: ✅ Rare invalid operations
- Recovers from errors: ✅ Completes debugging tasks

## Backward Compatibility

All enhancements are **additive** and **non-breaking**:

- Existing tool schemas unchanged
- New fields are optional
- Resources are opt-in (clients can ignore them)
- Annotations don't affect core functionality

## Alternative Considered: Prompts

**Why not just add this to server capabilities/prompts?**

Prompts are good for:
- Global system behavior
- General instructions
- Context setting

Resources are better for:
- Detailed, structured information
- Machine-readable workflows
- Dynamic content (based on server state)
- Version-specific documentation
- Reference material clients can query on-demand

**Decision**: Use both. Prompts for high-level guidance, resources for detailed documentation.

## Success Metrics

### Quantitative

1. **Error Rate**: Reduce InvalidState errors by > 80%
2. **Tool Call Efficiency**: Reduce unnecessary tool calls by > 50%
3. **Task Completion**: Increase successful debugging sessions by > 60%
4. **Discovery Time**: Reduce time to first successful breakpoint by > 70%

### Qualitative

1. **Claude's Understanding**: Can Claude explain the workflow without prompting?
2. **Error Recovery**: Does Claude recover from errors without human intervention?
3. **Best Practices**: Does Claude follow recommended patterns?
4. **Documentation Usage**: Does Claude reference resources in its reasoning?

## Implementation Timeline

- **Phase 1**: 2 hours - Enhanced tool descriptions
- **Phase 2**: 3 hours - Workflow resource
- **Phase 3**: 2 hours - State machine resource
- **Phase 4**: 2 hours - Error handling resource
- **Phase 5**: 2 hours - Integration testing

**Total**: 11 hours over 2-3 days

## Conclusion

This proposal transforms the Debugger MCP server from a collection of tools into a **self-documenting, workflow-aware debugging platform**. By providing:

1. ✅ **Explicit workflows** - End-to-end guidance
2. ✅ **State machine** - Valid operation mapping
3. ✅ **Error handling** - Recovery strategies
4. ✅ **Concrete examples** - Actionable patterns
5. ✅ **Timing information** - Async operation clarity

We enable AI clients like Claude Code to:
- **Understand** the debugger's capabilities
- **Use** tools correctly on the first try
- **Recover** from errors automatically
- **Complete** debugging tasks successfully

The implementation is straightforward, backward-compatible, and provides immediate value to both AI clients and human developers.

**Recommendation**: Implement all 5 tiers for maximum impact. The investment of 11 hours will significantly improve the debugger's usability and reduce support burden.
