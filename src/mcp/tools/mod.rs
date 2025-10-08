use crate::debug::SessionManager;
use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerStartArgs {
    pub language: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub stop_on_entry: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointArgs {
    pub session_id: String,
    pub source_path: String,
    pub line: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArgs {
    pub session_id: String,
    pub expression: String,
    pub frame_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForStopArgs {
    pub session_id: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBreakpointsArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepArgs {
    pub session_id: String,
    pub thread_id: Option<i32>,
}

pub struct ToolsHandler {
    session_manager: Arc<RwLock<SessionManager>>,
}

impl ToolsHandler {
    pub fn new(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { session_manager }
    }

    pub async fn handle_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            "debugger_start" => self.debugger_start(arguments).await,
            "debugger_session_state" => self.debugger_session_state(arguments).await,
            "debugger_set_breakpoint" => self.debugger_set_breakpoint(arguments).await,
            "debugger_continue" => self.debugger_continue(arguments).await,
            "debugger_stack_trace" => self.debugger_stack_trace(arguments).await,
            "debugger_evaluate" => self.debugger_evaluate(arguments).await,
            "debugger_disconnect" => self.debugger_disconnect(arguments).await,
            "debugger_wait_for_stop" => self.debugger_wait_for_stop(arguments).await,
            "debugger_list_breakpoints" => self.debugger_list_breakpoints(arguments).await,
            "debugger_step_over" => self.debugger_step_over(arguments).await,
            "debugger_step_into" => self.debugger_step_into(arguments).await,
            "debugger_step_out" => self.debugger_step_out(arguments).await,
            _ => Err(Error::MethodNotFound(name.to_string())),
        }
    }

    async fn debugger_start(&self, arguments: Value) -> Result<Value> {
        let args: DebuggerStartArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session_id = manager
            .create_session(
                &args.language,
                args.program,
                args.args,
                args.cwd,
                args.stop_on_entry,
            )
            .await?;

        Ok(json!({
            "sessionId": session_id,
            "status": "started"
        }))
    }

    async fn debugger_session_state(&self, arguments: Value) -> Result<Value> {
        let args: SessionStateArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let state = manager.get_session_state(&args.session_id).await?;

        // Convert DebugState to JSON-friendly format
        let (state_str, details) = match state {
            crate::debug::state::DebugState::NotStarted => ("NotStarted", json!({})),
            crate::debug::state::DebugState::Initializing => ("Initializing", json!({})),
            crate::debug::state::DebugState::Initialized => ("Initialized", json!({})),
            crate::debug::state::DebugState::Launching => ("Launching", json!({})),
            crate::debug::state::DebugState::Running => ("Running", json!({})),
            crate::debug::state::DebugState::Stopped { thread_id, reason } => (
                "Stopped",
                json!({
                    "threadId": thread_id,
                    "reason": reason
                }),
            ),
            crate::debug::state::DebugState::Terminated => ("Terminated", json!({})),
            crate::debug::state::DebugState::Failed { error } => (
                "Failed",
                json!({
                    "error": error
                }),
            ),
        };

        Ok(json!({
            "sessionId": args.session_id,
            "state": state_str,
            "details": details
        }))
    }

    async fn debugger_set_breakpoint(&self, arguments: Value) -> Result<Value> {
        let args: SetBreakpointArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        let verified = session
            .set_breakpoint(args.source_path.clone(), args.line)
            .await?;

        Ok(json!({
            "verified": verified,
            "sourcePath": args.source_path,
            "line": args.line
        }))
    }

    async fn debugger_continue(&self, arguments: Value) -> Result<Value> {
        let args: ContinueArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        session.continue_execution().await?;

        Ok(json!({
            "status": "continued"
        }))
    }

    async fn debugger_stack_trace(&self, arguments: Value) -> Result<Value> {
        let args: StackTraceArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        // Validate we're in a stopped state
        let state = session.get_state().await;
        if !matches!(state, crate::debug::state::DebugState::Stopped { .. }) {
            return Err(Error::InvalidState(
                "Cannot get stack trace while program is running. The program must be stopped at a breakpoint, entry point, or step. Use debugger_wait_for_stop() to wait for the program to stop.".to_string()
            ));
        }

        let frames = session.stack_trace().await?;

        Ok(json!({
            "stackFrames": frames
        }))
    }

    async fn debugger_evaluate(&self, arguments: Value) -> Result<Value> {
        let args: EvaluateArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        // Validate we're in a stopped state
        let state = session.get_state().await;
        if !matches!(state, crate::debug::state::DebugState::Stopped { .. }) {
            return Err(Error::InvalidState(
                "Cannot evaluate expressions while program is running. The program must be stopped at a breakpoint, entry point, or step. Use debugger_wait_for_stop() to wait for the program to stop.".to_string()
            ));
        }

        let result = session.evaluate(&args.expression, args.frame_id).await?;

        Ok(json!({
            "result": result
        }))
    }

    async fn debugger_wait_for_stop(&self, arguments: Value) -> Result<Value> {
        let args: WaitForStopArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        let timeout = tokio::time::Duration::from_millis(args.timeout_ms);
        let start = tokio::time::Instant::now();

        loop {
            let state = session.get_state().await;

            // Check if we're stopped
            if let crate::debug::state::DebugState::Stopped { thread_id, reason } = state {
                return Ok(json!({
                    "state": "Stopped",
                    "threadId": thread_id,
                    "reason": reason
                }));
            }

            // Check if program terminated
            if matches!(state, crate::debug::state::DebugState::Terminated) {
                return Ok(json!({
                    "state": "Terminated",
                    "reason": "Program exited"
                }));
            }

            // Check if program failed
            if let crate::debug::state::DebugState::Failed { error } = state {
                return Err(Error::Dap(format!("Session failed: {}", error)));
            }

            // Check timeout
            if start.elapsed() > timeout {
                return Err(Error::InvalidState(format!(
                    "Timeout waiting for program to stop ({}ms). Current state: {:?}",
                    args.timeout_ms, state
                )));
            }

            // Sleep briefly before checking again
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    async fn debugger_list_breakpoints(&self, arguments: Value) -> Result<Value> {
        let args: ListBreakpointsArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        let full_state = session.get_full_state().await;

        // Collect all breakpoints from all source files
        let mut all_breakpoints = Vec::new();
        for (source_path, breakpoints) in full_state.breakpoints.iter() {
            for bp in breakpoints {
                all_breakpoints.push(json!({
                    "id": bp.id,
                    "verified": bp.verified,
                    "line": bp.line,
                    "sourcePath": source_path
                }));
            }
        }

        Ok(json!({
            "breakpoints": all_breakpoints
        }))
    }

    async fn debugger_step_over(&self, arguments: Value) -> Result<Value> {
        let args: StepArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        // Validate we're in a stopped state
        let state = session.get_state().await;
        let thread_id = if let crate::debug::state::DebugState::Stopped { thread_id, .. } = state {
            thread_id
        } else {
            return Err(Error::InvalidState(
                "Cannot step while program is running. The program must be stopped first."
                    .to_string(),
            ));
        };

        let thread_id = args.thread_id.unwrap_or(thread_id);
        session.step_over(thread_id).await?;

        Ok(json!({
            "status": "stepping",
            "threadId": thread_id
        }))
    }

    async fn debugger_step_into(&self, arguments: Value) -> Result<Value> {
        let args: StepArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        // Validate we're in a stopped state
        let state = session.get_state().await;
        let thread_id = if let crate::debug::state::DebugState::Stopped { thread_id, .. } = state {
            thread_id
        } else {
            return Err(Error::InvalidState(
                "Cannot step while program is running. The program must be stopped first."
                    .to_string(),
            ));
        };

        let thread_id = args.thread_id.unwrap_or(thread_id);
        session.step_into(thread_id).await?;

        Ok(json!({
            "status": "stepping",
            "threadId": thread_id
        }))
    }

    async fn debugger_step_out(&self, arguments: Value) -> Result<Value> {
        let args: StepArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;

        // Validate we're in a stopped state
        let state = session.get_state().await;
        let thread_id = if let crate::debug::state::DebugState::Stopped { thread_id, .. } = state {
            thread_id
        } else {
            return Err(Error::InvalidState(
                "Cannot step while program is running. The program must be stopped first."
                    .to_string(),
            ));
        };

        let thread_id = args.thread_id.unwrap_or(thread_id);
        session.step_out(thread_id).await?;

        Ok(json!({
            "status": "stepping",
            "threadId": thread_id
        }))
    }

    async fn debugger_disconnect(&self, arguments: Value) -> Result<Value> {
        let args: DisconnectArgs = serde_json::from_value(arguments)?;

        let manager = self.session_manager.write().await;
        manager.remove_session(&args.session_id).await?;

        Ok(json!({
            "status": "disconnected"
        }))
    }

    pub fn list_tools() -> Vec<Value> {
        vec![
            json!({
                "name": "debugger_start",
                "title": "Start Debugging Session",
                "description": "Starts a new debugging session for a program. RETURNS IMMEDIATELY with a sessionId while initialization happens asynchronously in the background.\n\nIMPORTANT WORKFLOW:\n1. Call this tool first to create a session\n2. Use debugger_wait_for_stop to wait for entry point (if stopOnEntry: true)\n3. Once stopped, set breakpoints with debugger_set_breakpoint\n4. Control execution with debugger_continue\n\nTIMING: Returns in <100ms. Background initialization takes 200-500ms.\n\n⭐ CRITICAL: stopOnEntry Parameter\n=================================\nFor reliable breakpoint debugging, ALWAYS use stopOnEntry: true:\n\n✅ RECOMMENDED (with stopOnEntry: true):\n  - Program pauses at first executable line\n  - Gives you time to set breakpoints before execution\n  - Prevents program from completing before breakpoints are set\n  - Required for debugging programs that execute quickly\n\n❌ NOT RECOMMENDED (stopOnEntry: false or omitted):\n  - Program runs immediately upon start\n  - May complete before breakpoints can be set\n  - Breakpoints might be missed\n  - Only use if you don't need breakpoints\n\nEXAMPLE WORKFLOW:\n  debugger_start({program: \"app.py\", stopOnEntry: true})\n  debugger_wait_for_stop()  // Wait for entry point\n  debugger_set_breakpoint({line: 20})  // Set while paused ✓\n  debugger_continue()  // Now resume to breakpoint\n\nSEE ALSO: debugger_wait_for_stop (efficient waiting), debugger_session_state (state checking), debugger://workflows (complete examples)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "language": {
                            "type": "string",
                            "description": "Programming language (e.g., 'python', 'ruby')"
                        },
                        "program": {
                            "type": "string",
                            "description": "Absolute or relative path to the program file to debug"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Command-line arguments passed to the program (optional, defaults to empty array)"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory for the program execution (optional, defaults to program's directory)"
                        },
                        "stopOnEntry": {
                            "type": "boolean",
                            "description": "If true, pauses execution at the program's first line (recommended for setting early breakpoints)"
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
                    "category": "session-management",
                    "priority": 1.0
                }
            }),
            json!({
                "name": "debugger_session_state",
                "title": "Check Session State",
                "description": "Retrieves the current state of a debugging session. Essential for tracking async initialization progress.\n\nWORKFLOW USAGE:\n- After debugger_start: Poll this until state is 'Running' or 'Stopped' (not 'Initializing')\n- Before setting breakpoints: Verify state is 'Stopped' (with stopOnEntry) or 'Running'\n- After operations: Check state to verify success or detect failures\n\nSTATES:\n- NotStarted: Session created but not yet initialized\n- Initializing: DAP adapter starting (wait for this to complete)\n- Launching: Program starting\n- Running: Program executing (can set breakpoints)\n- Stopped: Hit breakpoint or paused (details.reason shows why)\n- Terminated: Program exited normally\n- Failed: Error occurred (details.error shows message)\n\nTIMING: Returns immediately (<10ms)\n\nTIP: When state is 'Stopped', check details.reason to understand why (e.g., 'entry', 'breakpoint', 'step')\n\nSEE ALSO: debugger://state-machine (complete state diagram), debugger-docs://guide/async-initialization",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID returned from debugger_start"
                        }
                    },
                    "required": ["sessionId"]
                },
                "annotations": {
                    "async": false,
                    "returnsTiming": "< 10ms",
                    "workflow": "state-checking",
                    "category": "session-management",
                    "pollable": true,
                    "priority": 0.9
                }
            }),
            json!({
                "name": "debugger_set_breakpoint",
                "title": "Set Breakpoint",
                "description": "Sets a breakpoint at a specific line in a source file. The debugger will pause execution when this line is about to execute.\n\nWORKFLOW:\n1. Ensure session state is 'Stopped' (recommended) or 'Running'\n2. Call this tool with the source file path and line number\n3. Check the 'verified' field in response (true = breakpoint accepted)\n4. Use debugger_continue to resume execution until breakpoint is hit\n\nTIMING: Returns in 5-20ms\n\nIMPORTANT: Use stopOnEntry: true when starting the session to pause before code execution, giving you time to set breakpoints.\n\nTIP: The sourcePath must match the path used by the debugger. For best results, use absolute paths.\n\nRETURNS:\n- verified: true if breakpoint was successfully set and recognized by the debugger\n- sourcePath: echo of the source file path\n- line: echo of the line number\n\nSEE ALSO: debugger_continue (to hit the breakpoint), debugger://workflows (breakpoint examples)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "sourcePath": {
                            "type": "string",
                            "description": "Absolute or relative path to the source file (must match debugger's path resolution)"
                        },
                        "line": {
                            "type": "integer",
                            "description": "Line number where breakpoint should be set (1-indexed, i.e., first line is 1)"
                        }
                    },
                    "required": ["sessionId", "sourcePath", "line"]
                },
                "annotations": {
                    "async": false,
                    "returnsTiming": "5-20ms",
                    "workflow": "breakpoint-management",
                    "category": "debugging",
                    "requiresState": ["Running", "Stopped"],
                    "priority": 0.8
                }
            }),
            json!({
                "name": "debugger_continue",
                "title": "Continue Execution",
                "description": "Resumes program execution after being paused (e.g., at a breakpoint or entry point). Execution continues until the next breakpoint, exception, or program termination.\n\nWORKFLOW:\n1. Session must be in 'Stopped' state (verify with debugger_session_state)\n2. Call this tool to resume execution\n3. Poll debugger_session_state to detect when execution stops again\n4. When state returns to 'Stopped', check details.reason:\n   - 'breakpoint': Hit a breakpoint (use debugger_stack_trace to inspect)\n   - 'exception': Uncaught exception occurred\n   - 'pause': Manual pause requested\n   - 'step': Completed a step operation\n\nTIMING: Returns in <10ms (but program continues running asynchronously)\n\nTIP: After calling continue, immediately poll debugger_session_state in a loop to detect when the program stops again.\n\nRETURNS: {\"status\": \"continued\"}\n\nSEE ALSO: debugger_stack_trace (inspect state when stopped), debugger://workflows (execution control patterns)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        }
                    },
                    "required": ["sessionId"]
                },
                "annotations": {
                    "async": true,
                    "returnsTiming": "< 10ms",
                    "completionTiming": "unknown (until next stop)",
                    "workflow": "execution-control",
                    "category": "debugging",
                    "requiresState": ["Stopped"],
                    "priority": 0.7
                }
            }),
            json!({
                "name": "debugger_stack_trace",
                "title": "Get Stack Trace",
                "description": "Retrieves the current call stack when execution is paused. Shows the sequence of function calls that led to the current execution point.\n\n⭐ PRIMARY PURPOSE: Get Frame IDs for debugger_evaluate\n======================================================\nThe 'id' field in each frame is CRITICAL - use it with debugger_evaluate to access variables:\n\nRETURNS: Array of stack frames, each containing:\n- id: Frame identifier → USE THIS as frameId in debugger_evaluate ⭐\n- name: Function/method name\n- source: {path: \"file path\", name: \"filename\"}\n- line: Current line number in this frame\n- column: Column number (if available)\n\n⚠️ Frame IDs Change Between Stops!\n================================\nFrame IDs are NOT stable across different stop events:\n- After EACH stop (breakpoint, step, continue), frame IDs change\n- ALWAYS call debugger_stack_trace fresh after each stop\n- NEVER reuse frame IDs from previous stops\n\nEXAMPLE PATTERN:\n  // Stop 1: Hit breakpoint\n  debugger_wait_for_stop()\n  stack1 = debugger_stack_trace()\n  frameId1 = stack1.stackFrames[0].id  // e.g., id = 5\n  debugger_evaluate({expression: \"x\", frameId: frameId1})  ✓\n  \n  // Stop 2: After continue and hit another breakpoint\n  debugger_continue()\n  debugger_wait_for_stop()\n  stack2 = debugger_stack_trace()  // GET FRESH TRACE!\n  frameId2 = stack2.stackFrames[0].id  // e.g., id = 8 (DIFFERENT!)\n  \n  // Using old frameId1 here would FAIL ❌\n  debugger_evaluate({expression: \"x\", frameId: frameId2})  ✓ Correct\n\nWORKFLOW:\n1. Session must be in 'Stopped' state (e.g., at a breakpoint)\n2. Call this tool to get current stack frames\n3. Extract the 'id' field from desired frame\n4. Pass that 'id' as frameId to debugger_evaluate\n5. Repeat steps 2-4 after each new stop event\n\nTIMING: Returns in 10-50ms depending on stack depth\n\nTIP: The first frame (index 0) is the current execution point. Higher indices are caller frames.\n\nCOMMON USE CASES:\n- Get frame IDs for debugger_evaluate (primary use)\n- Inspect where a breakpoint was hit\n- Understand call hierarchy\n- Diagnose unexpected execution paths\n\nSEE ALSO: debugger_evaluate (requires frame IDs from this tool), debugger://patterns (frame ID usage examples)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        }
                    },
                    "required": ["sessionId"]
                },
                "annotations": {
                    "async": false,
                    "returnsTiming": "10-50ms",
                    "workflow": "inspection",
                    "category": "debugging",
                    "requiresState": ["Stopped"],
                    "priority": 0.6
                }
            }),
            json!({
                "name": "debugger_evaluate",
                "title": "Evaluate Expression",
                "description": "Evaluates an expression in the context of the paused program. Can access variables, call functions, and perform computations using the program's current state.\n\n⚠️ CRITICAL: frameId Requirement\n================================\nWhile technically optional, frameId is REQUIRED in practice for accessing local variables:\n\n❌ WITHOUT frameId:\n  debugger_evaluate({expression: \"local_var\"})\n  → Result: NameError: name 'local_var' is not defined\n  \n  Why: Evaluates in global/default context where local variables don't exist\n\n✅ WITH frameId (REQUIRED WORKFLOW):\n  1. Get stack trace: stack = debugger_stack_trace()\n  2. Extract frame ID: frameId = stack.stackFrames[0].id\n  3. Evaluate with frameId:\n     debugger_evaluate({expression: \"local_var\", frameId: frameId})\n  → Result: Successfully accesses local variable ✓\n\n⚠️ Frame IDs Change Between Stops!\n  - Frame IDs are NOT stable across different stop events\n  - ALWAYS get a fresh stack trace after each stop\n  - NEVER reuse frame IDs from previous stops\n\nEXAMPLE PATTERN (Correct Way):\n  // After hitting breakpoint:\n  const stack = debugger_stack_trace()\n  const frameId = stack.stackFrames[0].id  // Current frame\n  const value = debugger_evaluate({expression: \"n\", frameId: frameId})\n  \n  // After next stop, get NEW frame ID:\n  const stack2 = debugger_stack_trace()  // Fresh trace!\n  const frameId2 = stack2.stackFrames[0].id  // New frame ID\n  const value2 = debugger_evaluate({expression: \"n\", frameId: frameId2})\n\nWORKFLOW:\n1. Session must be in 'Stopped' state\n2. Call debugger_stack_trace to get current stack frames\n3. Extract frame ID from desired frame (usually frame[0] for current location)\n4. Call this tool with expression AND frameId\n5. Examine the result value\n\nTIMING: Returns in 20-200ms depending on expression complexity\n\nEXPRESSION EXAMPLES:\n- Variable access: \"x\", \"obj.property\", \"array[0]\"\n- Arithmetic: \"x + y\", \"count * 2\"\n- Comparisons: \"x > 10\", \"status == 'ready'\"\n- Function calls: \"len(array)\", \"obj.method()\"\n- Complex: \"[item for item in list if item > 0]\" (Python)\n\nRETURNS: {\"result\": \"string representation of evaluation result\"}\n\nCOMMON ERROR:\n  \"NameError: name 'variable' is not defined\"\n  → Solution: Add frameId parameter from debugger_stack_trace\n\nSEE ALSO: debugger_stack_trace (get frame IDs), debugger://patterns (cookbook examples)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "expression": {
                            "type": "string",
                            "description": "Expression to evaluate (syntax depends on programming language being debugged)"
                        },
                        "frameId": {
                            "type": "integer",
                            "description": "Stack frame ID from debugger_stack_trace (optional, defaults to current frame)"
                        }
                    },
                    "required": ["sessionId", "expression"]
                },
                "annotations": {
                    "async": false,
                    "returnsTiming": "20-200ms",
                    "workflow": "inspection",
                    "category": "debugging",
                    "requiresState": ["Stopped"],
                    "priority": 0.5
                }
            }),
            json!({
                "name": "debugger_disconnect",
                "title": "Disconnect Session",
                "description": "Terminates a debugging session and cleans up all associated resources. The debugged program will be stopped if still running.\n\nWORKFLOW:\n1. Call this when debugging is complete\n2. Session and all breakpoints are removed\n3. Debugged program is terminated gracefully\n\nTIMING: Returns in 50-200ms (includes cleanup time)\n\nIMPORTANT: Always disconnect when finished to free resources. The session cannot be resumed after disconnection.\n\nRETURNS: {\"status\": \"disconnected\"}\n\nTIP: If the program is still running, it will be terminated. If you want to let the program finish naturally, you can skip calling this tool, but resources will not be cleaned up immediately.\n\nSEE ALSO: debugger://workflows (complete debugging workflows showing disconnect)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        }
                    },
                    "required": ["sessionId"]
                },
                "annotations": {
                    "async": false,
                    "returnsTiming": "50-200ms",
                    "workflow": "cleanup",
                    "category": "session-management",
                    "destructive": true,
                    "priority": 0.4
                }
            }),
            json!({
                "name": "debugger_wait_for_stop",
                "title": "Wait For Program To Stop",
                "description": "Blocks until the debugger stops (at breakpoint, step, or entry point), or times out. More efficient than polling debugger_session_state.\n\n⭐ EFFICIENT ALTERNATIVE TO POLLING\n==================================\nReplaces old pattern of repeated sleep + state check with single blocking call:\n\n❌ OLD PATTERN (slow, inefficient):\n  debugger_continue()\n  sleep(200ms)  // Arbitrary delay\n  state = debugger_session_state()\n  if state != \"Stopped\":\n    sleep(500ms)  // More waiting\n    state = debugger_session_state()  // Still might be Running\n  // Takes 500-3000ms with multiple polls\n\n✅ NEW PATTERN (fast, efficient):\n  debugger_continue()\n  debugger_wait_for_stop({timeoutMs: 5000})\n  // Returns immediately when stopped (typically <100ms)\n  // No wasted polling cycles!\n\n⭐ TIMING BEHAVIOR\n=================\n- If ALREADY stopped: Returns immediately (<10ms)\n- If running: Blocks until stop event or timeout\n- If program terminated: Returns with state \"Terminated\"\n- If timeout expires: Returns error\n\nTypical return times:\n- Entry point (stopOnEntry): <100ms\n- Breakpoint hit: <100ms  \n- Step completion: <50ms\n\nCOMMON PATTERNS:\n\n1. Wait for entry after start:\n   debugger_start({stopOnEntry: true})\n   debugger_wait_for_stop()  // Immediate return when at entry\n\n2. Wait for breakpoint:\n   debugger_continue()\n   debugger_wait_for_stop()  // Blocks until breakpoint hit\n\n3. Wait for step completion:\n   debugger_step_over()\n   debugger_wait_for_stop()  // Blocks until step completes\n\n4. Loop through multiple stops:\n   for (i = 0; i < 5; i++):\n     debugger_continue()\n     result = debugger_wait_for_stop()\n     // Process each stop...\n\nWORKFLOW:\n1. Call debugger_continue(), debugger_step_*, or debugger_start()\n2. Call this tool to wait for the next stop event\n3. Returns immediately when program stops\n4. Check result.reason to understand why it stopped\n\nRETURNS:\n{\n  \"state\": \"Stopped\",\n  \"threadId\": 1,\n  \"reason\": \"breakpoint\"  // or \"entry\", \"step\", \"pause\", etc.\n}\n\nPERFORMANCE:\n~5x faster than polling approach\nNo wasted CPU cycles\nImmediate notification of state changes\n\nSEE ALSO: debugger_session_state (check current state), debugger_continue (resume execution)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "timeoutMs": {
                            "type": "integer",
                            "default": 5000,
                            "description": "Maximum time to wait in milliseconds (default: 5000)"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_list_breakpoints",
                "title": "List All Breakpoints",
                "description": "Lists all breakpoints currently set across all source files.\n\nUSEFUL FOR:\n- Verifying which breakpoints are active\n- Checking breakpoint verification status\n- Debugging why a breakpoint might not be hit\n\nTIMING: Returns immediately (<10ms)\n\nRETURNS: Array of breakpoints with id, verified status, line, and sourcePath",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_step_over",
                "title": "Step Over (Next Line)",
                "description": "Executes the current line and stops at the next line. Does NOT step into function calls.\n\nREQUIRES: Program must be stopped (at breakpoint, entry, or previous step)\n\nWORKFLOW:\n1. Ensure program is stopped\n2. Call this tool to execute one line\n3. Use debugger_wait_for_stop to wait for the step to complete\n4. Inspect state with debugger_stack_trace and debugger_evaluate\n\nTIMING: Returns quickly; use debugger_wait_for_stop to detect completion\n\nSEE ALSO: debugger_step_into (to step into functions), debugger_step_out (to step out)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "threadId": {
                            "type": "integer",
                            "description": "Thread ID (optional, uses stopped thread if not specified)"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_step_into",
                "title": "Step Into (Enter Function)",
                "description": "Steps into function calls on the current line. If no function call, behaves like step_over.\n\nREQUIRES: Program must be stopped\n\nUSEFUL FOR: Debugging function implementations line by line\n\nWORKFLOW: Same as debugger_step_over\n\nSEE ALSO: debugger_step_over (to skip functions), debugger_step_out (to exit function)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "threadId": {
                            "type": "integer",
                            "description": "Thread ID (optional)"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_step_out",
                "title": "Step Out (Exit Function)",
                "description": "Continues execution until the current function returns, then stops at the caller.\n\nREQUIRES: Program must be stopped inside a function\n\nUSEFUL FOR: Quickly exiting from deep call stacks\n\nWORKFLOW: Same as debugger_step_over\n\nSEE ALSO: debugger_step_into (to enter function), debugger_step_over (to skip line)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Session ID from debugger_start"
                        },
                        "threadId": {
                            "type": "integer",
                            "description": "Thread ID (optional)"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::SessionManager;

    #[test]
    fn test_debugger_start_args_deserialization() {
        let json = json!({
            "language": "python",
            "program": "/path/to/script.py",
            "args": ["arg1", "arg2"],
            "cwd": "/working/dir"
        });

        let args: DebuggerStartArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.language, "python");
        assert_eq!(args.program, "/path/to/script.py");
        assert_eq!(args.args.len(), 2);
        assert_eq!(args.cwd, Some("/working/dir".to_string()));
    }

    #[test]
    fn test_debugger_start_args_without_cwd() {
        let json = json!({
            "language": "python",
            "program": "test.py",
            "args": []
        });

        let args: DebuggerStartArgs = serde_json::from_value(json).unwrap();
        assert!(args.cwd.is_none());
        assert!(args.args.is_empty());
    }

    #[test]
    fn test_set_breakpoint_args_deserialization() {
        let json = json!({
            "sessionId": "session-123",
            "sourcePath": "/path/to/file.py",
            "line": 42
        });

        let args: SetBreakpointArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.session_id, "session-123");
        assert_eq!(args.source_path, "/path/to/file.py");
        assert_eq!(args.line, 42);
    }

    #[test]
    fn test_continue_args_deserialization() {
        let json = json!({"sessionId": "test-session"});
        let args: ContinueArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.session_id, "test-session");
    }

    #[test]
    fn test_stack_trace_args_deserialization() {
        let json = json!({"sessionId": "trace-session"});
        let args: StackTraceArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.session_id, "trace-session");
    }

    #[test]
    fn test_evaluate_args_deserialization() {
        let json = json!({
            "sessionId": "eval-session",
            "expression": "x + y",
            "frameId": 5
        });

        let args: EvaluateArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.session_id, "eval-session");
        assert_eq!(args.expression, "x + y");
        assert_eq!(args.frame_id, Some(5));
    }

    #[test]
    fn test_evaluate_args_without_frame_id() {
        let json = json!({
            "sessionId": "eval-session",
            "expression": "x + y"
        });

        let args: EvaluateArgs = serde_json::from_value(json).unwrap();
        assert!(args.frame_id.is_none());
    }

    #[test]
    fn test_disconnect_args_deserialization() {
        let json = json!({"sessionId": "disconnect-session"});
        let args: DisconnectArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.session_id, "disconnect-session");
    }

    #[test]
    fn test_list_tools() {
        let tools = ToolsHandler::list_tools();
        assert_eq!(tools.len(), 12); // Updated from 7 to 12

        // Verify tool names
        let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

        // Original tools
        assert!(tool_names.contains(&"debugger_start"));
        assert!(tool_names.contains(&"debugger_session_state"));
        assert!(tool_names.contains(&"debugger_set_breakpoint"));
        assert!(tool_names.contains(&"debugger_continue"));
        assert!(tool_names.contains(&"debugger_stack_trace"));
        assert!(tool_names.contains(&"debugger_evaluate"));
        assert!(tool_names.contains(&"debugger_disconnect"));

        // New tools
        assert!(tool_names.contains(&"debugger_wait_for_stop"));
        assert!(tool_names.contains(&"debugger_list_breakpoints"));
        assert!(tool_names.contains(&"debugger_step_over"));
        assert!(tool_names.contains(&"debugger_step_into"));
        assert!(tool_names.contains(&"debugger_step_out"));
    }

    #[test]
    fn test_list_tools_schema_validation() {
        let tools = ToolsHandler::list_tools();

        // Check first tool structure
        let start_tool = &tools[0];
        assert_eq!(start_tool["name"], "debugger_start");
        assert!(start_tool["description"].is_string());
        assert!(start_tool["inputSchema"]["type"].is_string());
        assert!(start_tool["inputSchema"]["properties"].is_object());
        assert!(start_tool["inputSchema"]["required"].is_array());
    }

    #[tokio::test]
    async fn test_tools_handler_new() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let _handler = ToolsHandler::new(manager);
        // Verify list_tools returns expected tools
        let tools = ToolsHandler::list_tools();
        assert!(tools.iter().any(|t| t["name"] == "debugger_start"));
    }

    #[tokio::test]
    async fn test_handle_tool_unknown_method() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        let result = handler.handle_tool("unknown_tool", json!({})).await;
        assert!(result.is_err());

        match result {
            Err(Error::MethodNotFound(name)) => {
                assert_eq!(name, "unknown_tool");
            }
            _ => panic!("Expected MethodNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_handle_tool_invalid_arguments() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Invalid JSON for debugger_start
        let result = handler
            .handle_tool("debugger_start", json!({"invalid": "data"}))
            .await;
        assert!(result.is_err());
    }

    // Phase 6: Error path tests for missing required fields and invalid types

    #[test]
    fn test_debugger_start_missing_language() {
        let json = json!({
            "program": "/path/to/script.py"
        });

        let result = serde_json::from_value::<DebuggerStartArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_debugger_start_missing_program() {
        let json = json!({
            "language": "python"
        });

        let result = serde_json::from_value::<DebuggerStartArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_debugger_start_invalid_args_type() {
        let json = json!({
            "language": "python",
            "program": "test.py",
            "args": "not an array"  // Should be array, not string
        });

        let result = serde_json::from_value::<DebuggerStartArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_breakpoint_missing_session_id() {
        let json = json!({
            "sourcePath": "/path/to/file.py",
            "line": 42
        });

        let result = serde_json::from_value::<SetBreakpointArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_breakpoint_missing_source_path() {
        let json = json!({
            "sessionId": "session-123",
            "line": 42
        });

        let result = serde_json::from_value::<SetBreakpointArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_breakpoint_missing_line() {
        let json = json!({
            "sessionId": "session-123",
            "sourcePath": "/path/to/file.py"
        });

        let result = serde_json::from_value::<SetBreakpointArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_breakpoint_invalid_line_type() {
        let json = json!({
            "sessionId": "session-123",
            "sourcePath": "/path/to/file.py",
            "line": "not a number"  // Should be integer
        });

        let result = serde_json::from_value::<SetBreakpointArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_continue_args_missing_session_id() {
        let json = json!({});

        let result = serde_json::from_value::<ContinueArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_trace_args_missing_session_id() {
        let json = json!({});

        let result = serde_json::from_value::<StackTraceArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_missing_session_id() {
        let json = json!({
            "expression": "x + y"
        });

        let result = serde_json::from_value::<EvaluateArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_missing_expression() {
        let json = json!({
            "sessionId": "eval-session"
        });

        let result = serde_json::from_value::<EvaluateArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_invalid_frame_id_type() {
        let json = json!({
            "sessionId": "eval-session",
            "expression": "x + y",
            "frameId": "not a number"  // Should be integer
        });

        let result = serde_json::from_value::<EvaluateArgs>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnect_missing_session_id() {
        let json = json!({});

        let result = serde_json::from_value::<DisconnectArgs>(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_debugger_start_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler
            .handle_tool("debugger_start", json!({"language": "python"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_set_breakpoint_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler
            .handle_tool("debugger_set_breakpoint", json!({"sessionId": "test"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_continue_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler.handle_tool("debugger_continue", json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_stack_trace_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler.handle_tool("debugger_stack_trace", json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_evaluate_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler
            .handle_tool("debugger_evaluate", json!({"sessionId": "test"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_disconnect_invalid_json() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ToolsHandler::new(manager);

        // Missing required fields
        let result = handler.handle_tool("debugger_disconnect", json!({})).await;
        assert!(result.is_err());
    }
}
