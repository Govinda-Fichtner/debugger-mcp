use crate::debug::SessionManager;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

mod documentation;
pub use documentation::DocumentationHandler;

/// MCP Resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resource contents response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// Resource handler for MCP resources
pub struct ResourcesHandler {
    session_manager: Arc<RwLock<SessionManager>>,
    documentation_handler: DocumentationHandler,
}

impl ResourcesHandler {
    pub fn new(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self {
            session_manager,
            documentation_handler: DocumentationHandler::new(
                "Govinda-Fichtner",
                "debugger-mcp",
                "main",
            ),
        }
    }

    /// List all available resources
    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let manager = self.session_manager.read().await;
        let session_ids = manager.list_sessions().await;

        let mut resources = vec![
            Resource {
                uri: "debugger://sessions".to_string(),
                name: "Debug Sessions".to_string(),
                description: Some("List of all active debugging sessions".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "debugger://workflows".to_string(),
                name: "Common Debugging Workflows".to_string(),
                description: Some(
                    "Step-by-step workflows for common debugging scenarios".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "debugger://state-machine".to_string(),
                name: "Session State Machine".to_string(),
                description: Some(
                    "Complete state machine diagram showing all session states and transitions"
                        .to_string(),
                ),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "debugger://error-handling".to_string(),
                name: "Error Handling Guide".to_string(),
                description: Some(
                    "Error codes, recovery strategies, and troubleshooting tips".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
            },
        ];

        // Add documentation resources
        resources.extend(self.documentation_handler.list_resources());

        // Add per-session resources
        for session_id in session_ids {
            resources.push(Resource {
                uri: format!("debugger://sessions/{}", session_id),
                name: format!("Session {}", &session_id[..8]),
                description: Some(format!("Details for debug session {}", session_id)),
                mime_type: Some("application/json".to_string()),
            });

            resources.push(Resource {
                uri: format!("debugger://sessions/{}/stackTrace", session_id),
                name: format!("Stack Trace ({})", &session_id[..8]),
                description: Some(format!("Call stack for session {}", session_id)),
                mime_type: Some("application/json".to_string()),
            });
        }

        Ok(resources)
    }

    /// Read resource contents by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
        // Check for embedded documentation resources first (debugger-docs://)
        if uri.starts_with("debugger-docs://") {
            return self.documentation_handler.read_resource(uri).await;
        }

        // Parse debugger:// URIs
        if !uri.starts_with("debugger://") {
            return Err(Error::InvalidRequest(format!(
                "Invalid resource URI: {}",
                uri
            )));
        }

        let path = &uri["debugger://".len()..];

        if path == "sessions" {
            // List all sessions
            self.read_sessions_list().await
        } else if path == "workflows" {
            // Common debugging workflows
            Self::read_workflows()
        } else if path == "state-machine" {
            // Session state machine documentation
            Self::read_state_machine()
        } else if path == "error-handling" {
            // Error handling guide
            Self::read_error_handling()
        } else if let Some(rest) = path.strip_prefix("sessions/") {
            // Parse session-specific resources
            let parts: Vec<&str> = rest.split('/').collect();
            match parts.len() {
                1 => {
                    // debugger://sessions/{id} - session details
                    let session_id = parts[0];
                    self.read_session_details(session_id).await
                }
                2 if parts[1] == "stackTrace" => {
                    // debugger://sessions/{id}/stackTrace
                    let session_id = parts[0];
                    self.read_session_stack_trace(session_id).await
                }
                _ => Err(Error::InvalidRequest(format!(
                    "Unknown resource path: {}",
                    path
                ))),
            }
        } else {
            Err(Error::InvalidRequest(format!("Unknown resource: {}", uri)))
        }
    }

    /// Read workflows resource
    fn read_workflows() -> Result<ResourceContents> {
        let workflows = json!({
            "workflows": [
                {
                    "name": "basic_debugging",
                    "title": "Basic Debugging with Breakpoints",
                    "description": "Start a session, set a breakpoint, and inspect variables",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Start debugging session with stopOnEntry",
                            "tool": "debugger_start",
                            "parameters": {
                                "language": "python",
                                "program": "/path/to/script.py",
                                "stopOnEntry": true
                            },
                            "expectedResult": "Session ID returned immediately",
                            "timing": "< 100ms"
                        },
                        {
                            "step": 2,
                            "action": "Wait for session to initialize",
                            "tool": "debugger_session_state",
                            "parameters": {
                                "sessionId": "<from step 1>"
                            },
                            "expectedResult": "State transitions: Initializing → Launching → Stopped (reason: entry)",
                            "timing": "200-500ms total (poll every 50-100ms)",
                            "note": "Poll until state is 'Stopped' with reason 'entry'"
                        },
                        {
                            "step": 3,
                            "action": "Set breakpoint while stopped at entry",
                            "tool": "debugger_set_breakpoint",
                            "parameters": {
                                "sessionId": "<from step 1>",
                                "sourcePath": "/path/to/script.py",
                                "line": 42
                            },
                            "expectedResult": "verified: true",
                            "timing": "5-20ms"
                        },
                        {
                            "step": 4,
                            "action": "Continue execution to hit breakpoint",
                            "tool": "debugger_continue",
                            "parameters": {
                                "sessionId": "<from step 1>"
                            },
                            "expectedResult": "status: continued",
                            "timing": "< 10ms to return"
                        },
                        {
                            "step": 5,
                            "action": "Wait for breakpoint hit",
                            "tool": "debugger_session_state",
                            "parameters": {
                                "sessionId": "<from step 1>"
                            },
                            "expectedResult": "State: Stopped, reason: breakpoint",
                            "timing": "Depends on program execution (poll every 50-100ms)",
                            "note": "Poll until state changes to 'Stopped' with reason 'breakpoint'"
                        },
                        {
                            "step": 6,
                            "action": "Get stack trace at breakpoint",
                            "tool": "debugger_stack_trace",
                            "parameters": {
                                "sessionId": "<from step 1>"
                            },
                            "expectedResult": "Array of stack frames",
                            "timing": "10-50ms"
                        },
                        {
                            "step": 7,
                            "action": "Inspect variables",
                            "tool": "debugger_evaluate",
                            "parameters": {
                                "sessionId": "<from step 1>",
                                "expression": "variable_name"
                            },
                            "expectedResult": "Variable value as string",
                            "timing": "20-200ms"
                        },
                        {
                            "step": 8,
                            "action": "Clean up and disconnect",
                            "tool": "debugger_disconnect",
                            "parameters": {
                                "sessionId": "<from step 1>"
                            },
                            "expectedResult": "status: disconnected",
                            "timing": "50-200ms"
                        }
                    ],
                    "totalTime": "~1-2 seconds",
                    "tips": [
                        "Always use stopOnEntry: true to pause before execution",
                        "Poll debugger_session_state to detect state transitions",
                        "Set breakpoints while in 'Stopped' state for reliability",
                        "Frame ID 0 is always the current execution point"
                    ]
                },
                {
                    "name": "quick_inspection",
                    "title": "Quick Variable Inspection (No Breakpoints)",
                    "description": "Debug a program that crashes, inspecting state at crash point",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Start session (no stopOnEntry needed)",
                            "tool": "debugger_start",
                            "parameters": {
                                "language": "python",
                                "program": "/path/to/script.py",
                                "stopOnEntry": false
                            }
                        },
                        {
                            "step": 2,
                            "action": "Wait for initialization",
                            "tool": "debugger_session_state",
                            "note": "Poll until state is 'Running' or 'Terminated' or 'Failed'"
                        },
                        {
                            "step": 3,
                            "action": "If stopped (e.g., exception), inspect state",
                            "tool": "debugger_stack_trace",
                            "note": "Only if state is 'Stopped'"
                        },
                        {
                            "step": 4,
                            "action": "Evaluate expressions to understand crash",
                            "tool": "debugger_evaluate"
                        },
                        {
                            "step": 5,
                            "action": "Disconnect",
                            "tool": "debugger_disconnect"
                        }
                    ],
                    "useCase": "Debugging crashes or exceptions"
                },
                {
                    "name": "multi_breakpoint",
                    "title": "Multiple Breakpoints Workflow",
                    "description": "Set multiple breakpoints and iterate through them",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Start with stopOnEntry",
                            "tool": "debugger_start"
                        },
                        {
                            "step": 2,
                            "action": "Wait for stopped at entry",
                            "tool": "debugger_session_state"
                        },
                        {
                            "step": 3,
                            "action": "Set multiple breakpoints",
                            "tool": "debugger_set_breakpoint",
                            "note": "Call this tool multiple times for different lines"
                        },
                        {
                            "step": 4,
                            "action": "Continue to first breakpoint",
                            "tool": "debugger_continue"
                        },
                        {
                            "step": 5,
                            "action": "Loop: inspect, then continue",
                            "note": "Repeat: debugger_session_state → debugger_stack_trace → debugger_evaluate → debugger_continue"
                        },
                        {
                            "step": 6,
                            "action": "Disconnect when done",
                            "tool": "debugger_disconnect"
                        }
                    ],
                    "useCase": "Tracing program flow through multiple points"
                }
            ],
            "commonPatterns": {
                "polling": {
                    "description": "Poll debugger_session_state to detect async state changes",
                    "interval": "50-100ms",
                    "timeout": "5-10 seconds for initialization, longer for breakpoint hits"
                },
                "stateChecks": {
                    "beforeBreakpoint": "Verify state is 'Stopped' or 'Running'",
                    "beforeContinue": "Verify state is 'Stopped'",
                    "beforeStackTrace": "Verify state is 'Stopped'",
                    "beforeEvaluate": "Verify state is 'Stopped'"
                },
                "errorHandling": {
                    "initializationFailed": "Check state for 'Failed' and inspect details.error",
                    "breakpointNotVerified": "Check verified field, may need absolute path",
                    "sessionNotFound": "Session may have timed out or been disconnected"
                }
            }
        });

        Ok(ResourceContents {
            uri: "debugger://workflows".to_string(),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&workflows)?),
            blob: None,
        })
    }

    /// Read state machine resource
    fn read_state_machine() -> Result<ResourceContents> {
        let state_machine = json!({
            "states": [
                {
                    "name": "NotStarted",
                    "description": "Session created but initialization not yet begun",
                    "duration": "Very brief (< 1ms)",
                    "nextStates": ["Initializing"]
                },
                {
                    "name": "Initializing",
                    "description": "DAP adapter is being spawned and initialized",
                    "duration": "200-500ms",
                    "activities": [
                        "Spawn DAP adapter process (e.g., debugpy)",
                        "Send initialize request",
                        "Wait for initialized event",
                        "Send configurationDone"
                    ],
                    "nextStates": ["Launching", "Failed"]
                },
                {
                    "name": "Launching",
                    "description": "Program is being launched by the DAP adapter",
                    "duration": "50-200ms",
                    "activities": [
                        "Send launch request with program path and args",
                        "Wait for process start",
                        "Apply stopOnEntry if requested"
                    ],
                    "nextStates": ["Running", "Stopped", "Failed"]
                },
                {
                    "name": "Running",
                    "description": "Program is executing normally",
                    "duration": "Until breakpoint hit or program terminates",
                    "availableOperations": [
                        "debugger_set_breakpoint (can set breakpoints while running)",
                        "debugger_session_state (check status)"
                    ],
                    "nextStates": ["Stopped", "Terminated", "Failed"]
                },
                {
                    "name": "Stopped",
                    "description": "Program execution is paused",
                    "duration": "Until debugger_continue is called",
                    "details": {
                        "threadId": "ID of stopped thread",
                        "reason": "Why execution stopped (entry, breakpoint, step, pause, exception)"
                    },
                    "availableOperations": [
                        "debugger_set_breakpoint (recommended time to set breakpoints)",
                        "debugger_stack_trace (inspect call stack)",
                        "debugger_evaluate (inspect variables)",
                        "debugger_continue (resume execution)",
                        "debugger_session_state (check status)"
                    ],
                    "nextStates": ["Running", "Terminated", "Failed"]
                },
                {
                    "name": "Terminated",
                    "description": "Program has exited normally",
                    "duration": "Final state",
                    "availableOperations": [
                        "debugger_session_state (confirm termination)",
                        "debugger_disconnect (clean up)"
                    ],
                    "nextStates": []
                },
                {
                    "name": "Failed",
                    "description": "An error occurred during debugging",
                    "duration": "Final state",
                    "details": {
                        "error": "Error message describing what went wrong"
                    },
                    "availableOperations": [
                        "debugger_session_state (get error details)",
                        "debugger_disconnect (clean up)"
                    ],
                    "nextStates": [],
                    "commonCauses": [
                        "Program path not found",
                        "DAP adapter not installed (e.g., debugpy)",
                        "Invalid language specified",
                        "Program crashed during initialization"
                    ]
                }
            ],
            "transitions": [
                {
                    "from": "NotStarted",
                    "to": "Initializing",
                    "trigger": "Automatic (background task starts immediately)"
                },
                {
                    "from": "Initializing",
                    "to": "Launching",
                    "trigger": "DAP adapter ready"
                },
                {
                    "from": "Initializing",
                    "to": "Failed",
                    "trigger": "DAP adapter spawn failed or initialization timeout"
                },
                {
                    "from": "Launching",
                    "to": "Running",
                    "trigger": "Program started (stopOnEntry: false)"
                },
                {
                    "from": "Launching",
                    "to": "Stopped",
                    "trigger": "Program started with stopOnEntry: true (reason: entry)"
                },
                {
                    "from": "Launching",
                    "to": "Failed",
                    "trigger": "Launch failed (program not found, permission denied, etc.)"
                },
                {
                    "from": "Running",
                    "to": "Stopped",
                    "trigger": "Breakpoint hit, exception, or pause request"
                },
                {
                    "from": "Running",
                    "to": "Terminated",
                    "trigger": "Program exit"
                },
                {
                    "from": "Running",
                    "to": "Failed",
                    "trigger": "DAP adapter crashed or connection lost"
                },
                {
                    "from": "Stopped",
                    "to": "Running",
                    "trigger": "debugger_continue called"
                },
                {
                    "from": "Stopped",
                    "to": "Terminated",
                    "trigger": "Program execution completed after continue"
                },
                {
                    "from": "Stopped",
                    "to": "Failed",
                    "trigger": "DAP adapter crashed"
                }
            ],
            "bestPractices": {
                "initialization": "Always poll debugger_session_state until out of 'Initializing' state",
                "breakpoints": "Set breakpoints while in 'Stopped' state for best reliability",
                "stopOnEntry": "Use stopOnEntry: true when you need to set early breakpoints",
                "polling": "Poll every 50-100ms to detect state transitions",
                "cleanup": "Always call debugger_disconnect to free resources"
            }
        });

        Ok(ResourceContents {
            uri: "debugger://state-machine".to_string(),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&state_machine)?),
            blob: None,
        })
    }

    /// Read error handling resource
    fn read_error_handling() -> Result<ResourceContents> {
        let error_handling = json!({
            "errors": [
                {
                    "type": "SessionNotFound",
                    "code": "SESSION_NOT_FOUND",
                    "message": "Session ID not found or already disconnected",
                    "causes": [
                        "Invalid session ID provided",
                        "Session was disconnected via debugger_disconnect",
                        "Session timed out due to inactivity",
                        "Session crashed and was removed"
                    ],
                    "recovery": [
                        "Verify the session ID is correct",
                        "Check if session is listed in debugger://sessions resource",
                        "Create a new session with debugger_start"
                    ]
                },
                {
                    "type": "InvalidState",
                    "code": "INVALID_STATE",
                    "message": "Operation not allowed in current session state",
                    "causes": [
                        "Tried to set breakpoint before session initialized",
                        "Tried to continue when not stopped",
                        "Tried to get stack trace when not stopped",
                        "Tried to evaluate when not stopped"
                    ],
                    "recovery": [
                        "Check current state with debugger_session_state",
                        "Wait for appropriate state transition",
                        "Consult debugger://state-machine for valid operations per state"
                    ]
                },
                {
                    "type": "InitializationFailed",
                    "code": "INITIALIZATION_FAILED",
                    "stateDetails": "State will be 'Failed' with error message",
                    "causes": [
                        "DAP adapter not installed (e.g., pip install debugpy)",
                        "Unsupported language",
                        "Program path not found",
                        "Insufficient permissions"
                    ],
                    "recovery": [
                        "Install required DAP adapter for the language",
                        "Verify program path exists and is accessible",
                        "Check file permissions",
                        "Review session state details.error for specific message"
                    ]
                },
                {
                    "type": "BreakpointNotVerified",
                    "code": "BREAKPOINT_NOT_VERIFIED",
                    "indication": "verified: false in response",
                    "causes": [
                        "Source path doesn't match debugger's path resolution",
                        "Line number is invalid (beyond file length, empty line, etc.)",
                        "Source file changed since session started",
                        "Relative path resolution issue"
                    ],
                    "recovery": [
                        "Use absolute paths for sourcePath",
                        "Verify line number is valid and contains executable code",
                        "Restart session if source files changed",
                        "Check that sourcePath matches program's file system view"
                    ]
                },
                {
                    "type": "EvaluationFailed",
                    "code": "EVALUATION_FAILED",
                    "causes": [
                        "Invalid expression syntax for the language",
                        "Variable not in scope for the frame",
                        "Expression caused an exception in debugged program",
                        "Frame ID invalid or no longer exists"
                    ],
                    "recovery": [
                        "Verify expression syntax matches the programming language",
                        "Get stack trace to confirm frame ID is valid",
                        "Check variable is in scope (may need different frame)",
                        "Simplify expression to isolate the issue"
                    ]
                },
                {
                    "type": "DAPConnectionLost",
                    "code": "DAP_CONNECTION_LOST",
                    "stateDetails": "State will transition to 'Failed'",
                    "causes": [
                        "DAP adapter process crashed",
                        "Debugged program killed the adapter",
                        "Network issue (if remote debugging)",
                        "Resource exhaustion (OOM, etc.)"
                    ],
                    "recovery": [
                        "Create new session with debugger_start",
                        "Check DAP adapter logs for crash details",
                        "Verify debugged program doesn't interfere with adapter",
                        "Monitor system resources"
                    ]
                },
                {
                    "type": "Timeout",
                    "code": "TIMEOUT",
                    "causes": [
                        "Initialization took longer than expected",
                        "Polled session state and program didn't respond",
                        "Network latency (if remote)",
                        "Debugged program in infinite loop"
                    ],
                    "recovery": [
                        "Increase polling timeout",
                        "Check if program is actually running (CPU usage)",
                        "Verify no deadlocks in debugged program",
                        "Consider using breakpoints to interrupt long-running code"
                    ]
                }
            ],
            "troubleshooting": {
                "sessionWontInitialize": {
                    "symptoms": "State stuck in 'Initializing' for > 5 seconds",
                    "steps": [
                        "Verify DAP adapter installed (e.g., python -m debugpy --version)",
                        "Check debugger MCP server logs for errors",
                        "Try with a minimal test program first",
                        "Ensure no firewall blocking DAP adapter ports"
                    ]
                },
                "breakpointNotHitting": {
                    "symptoms": "Program runs but doesn't stop at breakpoint",
                    "steps": [
                        "Verify breakpoint verified: true in response",
                        "Check program actually executes that line (add print statement)",
                        "Ensure correct source file (not a copy)",
                        "Try setting breakpoint while stopped (use stopOnEntry)"
                    ]
                },
                "cannotInspectVariables": {
                    "symptoms": "debugger_evaluate returns errors",
                    "steps": [
                        "Confirm state is 'Stopped' (not 'Running')",
                        "Get stack trace first to verify frame IDs",
                        "Try evaluating simple expressions first (e.g., '1+1')",
                        "Check variable is in scope for current frame"
                    ]
                },
                "programTerminatesImmediately": {
                    "symptoms": "State goes from Launching → Terminated",
                    "steps": [
                        "Use stopOnEntry: true to pause at first line",
                        "Check if program has any code to execute",
                        "Verify program doesn't exit immediately (syntax errors, etc.)",
                        "Look for errors in program's stdout/stderr"
                    ]
                }
            },
            "bestPractices": [
                "Always check debugger_session_state before operations",
                "Use stopOnEntry: true for reliable breakpoint setup",
                "Poll state every 50-100ms to detect transitions",
                "Handle 'Failed' state by reading details.error",
                "Use absolute paths for source files",
                "Call debugger_disconnect to clean up resources",
                "Implement timeouts in your polling loops",
                "Check verified: true when setting breakpoints"
            ],
            "seeAlso": [
                "debugger://workflows (for correct usage patterns)",
                "debugger://state-machine (for valid operations per state)",
                "debugger-docs://troubleshooting (for more detailed guides)"
            ]
        });

        Ok(ResourceContents {
            uri: "debugger://error-handling".to_string(),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&error_handling)?),
            blob: None,
        })
    }

    /// Read sessions list resource
    async fn read_sessions_list(&self) -> Result<ResourceContents> {
        let manager = self.session_manager.read().await;
        let session_ids = manager.list_sessions().await;

        let mut sessions = Vec::new();
        for session_id in session_ids {
            if let Ok(session) = manager.get_session(&session_id).await {
                let state = session.get_state().await;
                sessions.push(json!({
                    "id": session.id,
                    "language": session.language,
                    "program": session.program,
                    "state": state,
                }));
            }
        }

        let content = json!({
            "sessions": sessions,
            "total": sessions.len(),
        });

        Ok(ResourceContents {
            uri: "debugger://sessions".to_string(),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&content)?),
            blob: None,
        })
    }

    /// Read session details resource
    async fn read_session_details(&self, session_id: &str) -> Result<ResourceContents> {
        let manager = self.session_manager.read().await;
        let session = manager.get_session(session_id).await?;

        let state = session.get_state().await;

        // Get breakpoints from session state
        let state_lock = session.state.read().await;
        let all_breakpoints: Vec<_> = state_lock
            .breakpoints
            .iter()
            .flat_map(|(source, bps)| {
                bps.iter().map(move |bp| {
                    json!({
                        "source": source,
                        "line": bp.line,
                        "id": bp.id,
                        "verified": bp.verified,
                    })
                })
            })
            .collect();
        drop(state_lock);

        let content = json!({
            "id": session.id,
            "language": session.language,
            "program": session.program,
            "state": state,
            "breakpoints": all_breakpoints,
        });

        Ok(ResourceContents {
            uri: format!("debugger://sessions/{}", session_id),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&content)?),
            blob: None,
        })
    }

    /// Read session stack trace resource
    async fn read_session_stack_trace(&self, session_id: &str) -> Result<ResourceContents> {
        let manager = self.session_manager.read().await;
        let session = manager.get_session(session_id).await?;

        let state = session.get_state().await;

        // Only get stack trace if stopped
        let frames: Vec<crate::dap::types::StackFrame> = match state {
            crate::debug::state::DebugState::Stopped { .. } => {
                session.stack_trace().await.unwrap_or_default()
            }
            _ => vec![],
        };

        let content = json!({
            "sessionId": session.id,
            "state": state,
            "stackFrames": frames,
        });

        Ok(ResourceContents {
            uri: format!("debugger://sessions/{}/stackTrace", session_id),
            mime_type: "application/json".to_string(),
            text: Some(serde_json::to_string_pretty(&content)?),
            blob: None,
        })
    }

    /// List available resource templates (for MCP discovery)
    pub fn list_resource_templates() -> Vec<Value> {
        let mut templates = vec![
            json!({
                "uriTemplate": "debugger://sessions",
                "name": "Debug Sessions",
                "description": "List all active debugging sessions",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "debugger://sessions/{sessionId}",
                "name": "Session Details",
                "description": "Get details for a specific debug session",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "debugger://sessions/{sessionId}/stackTrace",
                "name": "Session Stack Trace",
                "description": "Get the call stack for a stopped debug session",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "debugger://workflows",
                "name": "Common Workflows",
                "description": "Step-by-step debugging workflows with examples",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "debugger://state-machine",
                "name": "State Machine",
                "description": "Complete session state machine with all transitions",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "debugger://error-handling",
                "name": "Error Handling",
                "description": "Error codes, recovery strategies, and troubleshooting",
                "mimeType": "application/json"
            }),
        ];

        // Add documentation templates
        templates.extend(DocumentationHandler::list_resource_templates());

        templates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::SessionManager;

    #[tokio::test]
    async fn test_resources_handler_new() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);
        // Verify construction works and list_resources is callable
        let resources = handler.list_resources().await.unwrap();
        assert!(!resources.is_empty()); // At least the list resource itself
    }

    #[tokio::test]
    async fn test_list_resources_empty() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let resources = handler.list_resources().await.unwrap();

        // Should have: sessions + workflows + state-machine + error-handling + 5 docs = 9
        assert_eq!(resources.len(), 9);
        assert_eq!(resources[0].uri, "debugger://sessions");
        assert_eq!(resources[0].name, "Debug Sessions");

        // Verify workflow resources are present
        assert!(resources.iter().any(|r| r.uri == "debugger://workflows"));
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger://state-machine"));
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger://error-handling"));

        // Verify documentation resources are present
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger-docs://getting-started"));
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger-docs://troubleshooting"));
    }

    #[tokio::test]
    async fn test_read_sessions_list_empty() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let contents = handler.read_resource("debugger://sessions").await.unwrap();

        assert_eq!(contents.uri, "debugger://sessions");
        assert_eq!(contents.mime_type, "application/json");
        assert!(contents.text.is_some());

        let text = contents.text.unwrap();
        assert!(text.contains("\"sessions\""));
        assert!(text.contains("\"total\": 0"));
    }

    #[tokio::test]
    async fn test_read_invalid_uri_scheme() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let result = handler.read_resource("http://invalid").await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("Invalid resource URI"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[tokio::test]
    async fn test_read_unknown_resource_path() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let result = handler.read_resource("debugger://unknown").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_session_not_found() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let result = handler
            .read_resource("debugger://sessions/nonexistent-id")
            .await;
        assert!(result.is_err());

        match result {
            Err(Error::SessionNotFound(_)) => {}
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_read_stack_trace_not_found() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let result = handler
            .read_resource("debugger://sessions/nonexistent-id/stackTrace")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_resource_templates() {
        let templates = ResourcesHandler::list_resource_templates();

        // Should have: 3 session templates + 3 workflow templates + 4 docs templates = 10
        assert_eq!(templates.len(), 10);

        // Check first template (sessions)
        assert!(templates[0]["uriTemplate"]
            .as_str()
            .unwrap()
            .contains("sessions"));
        assert!(templates[0]["name"].as_str().is_some());
        assert!(templates[0]["mimeType"].as_str().unwrap() == "application/json");

        // Verify workflow templates are present
        assert!(templates
            .iter()
            .any(|t| t["uriTemplate"].as_str().unwrap() == "debugger://workflows"));

        // Verify documentation templates are present
        assert!(templates
            .iter()
            .any(|t| t["uriTemplate"].as_str().unwrap() == "debugger-docs://getting-started"));
    }

    #[tokio::test]
    async fn test_resource_uri_parsing() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        // Test various invalid URIs
        let invalid_uris = vec![
            "debugger://sessions/id/invalid/path",
            "debugger://sessions//",
            "debugger://",
        ];

        for uri in invalid_uris {
            let result = handler.read_resource(uri).await;
            assert!(result.is_err(), "URI should be invalid: {}", uri);
        }
    }

    #[test]
    fn test_resource_struct_serialization() {
        let resource = Resource {
            uri: "debugger://test".to_string(),
            name: "Test".to_string(),
            description: Some("Description".to_string()),
            mime_type: Some("application/json".to_string()),
        };

        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("debugger://test"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_resource_contents_serialization() {
        let contents = ResourceContents {
            uri: "debugger://test".to_string(),
            mime_type: "application/json".to_string(),
            text: Some("{\"test\": true}".to_string()),
            blob: None,
        };

        let json = serde_json::to_string(&contents).unwrap();
        assert!(json.contains("debugger://test"));
        assert!(json.contains("application/json"));
    }
}
