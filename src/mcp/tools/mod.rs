use crate::{Error, Result};
use crate::debug::SessionManager;
use serde::{Deserialize, Serialize};
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
            "debugger_set_breakpoint" => self.debugger_set_breakpoint(arguments).await,
            "debugger_continue" => self.debugger_continue(arguments).await,
            "debugger_stack_trace" => self.debugger_stack_trace(arguments).await,
            "debugger_evaluate" => self.debugger_evaluate(arguments).await,
            "debugger_disconnect" => self.debugger_disconnect(arguments).await,
            _ => Err(Error::MethodNotFound(name.to_string())),
        }
    }

    async fn debugger_start(&self, arguments: Value) -> Result<Value> {
        let args: DebuggerStartArgs = serde_json::from_value(arguments)?;
        
        let manager = self.session_manager.read().await;
        let session_id = manager
            .create_session(&args.language, args.program, args.args, args.cwd)
            .await?;

        Ok(json!({
            "sessionId": session_id,
            "status": "started"
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
        
        let frames = session.stack_trace().await?;

        Ok(json!({
            "stackFrames": frames
        }))
    }

    async fn debugger_evaluate(&self, arguments: Value) -> Result<Value> {
        let args: EvaluateArgs = serde_json::from_value(arguments)?;
        
        let manager = self.session_manager.read().await;
        let session = manager.get_session(&args.session_id).await?;
        
        let result = session.evaluate(&args.expression, args.frame_id).await?;

        Ok(json!({
            "result": result
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
                "description": "Start a debugging session for a program",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "language": {
                            "type": "string",
                            "description": "Programming language (e.g., 'python')"
                        },
                        "program": {
                            "type": "string",
                            "description": "Path to the program to debug"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Command-line arguments for the program"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory for the program"
                        }
                    },
                    "required": ["language", "program"]
                }
            }),
            json!({
                "name": "debugger_set_breakpoint",
                "description": "Set a breakpoint in a source file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Debug session ID"
                        },
                        "sourcePath": {
                            "type": "string",
                            "description": "Path to the source file"
                        },
                        "line": {
                            "type": "integer",
                            "description": "Line number (1-indexed)"
                        }
                    },
                    "required": ["sessionId", "sourcePath", "line"]
                }
            }),
            json!({
                "name": "debugger_continue",
                "description": "Continue execution after a breakpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Debug session ID"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_stack_trace",
                "description": "Get the current call stack",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Debug session ID"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
            json!({
                "name": "debugger_evaluate",
                "description": "Evaluate an expression in the debug context",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Debug session ID"
                        },
                        "expression": {
                            "type": "string",
                            "description": "Expression to evaluate"
                        },
                        "frameId": {
                            "type": "integer",
                            "description": "Stack frame ID (optional)"
                        }
                    },
                    "required": ["sessionId", "expression"]
                }
            }),
            json!({
                "name": "debugger_disconnect",
                "description": "Disconnect from a debugging session",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sessionId": {
                            "type": "string",
                            "description": "Debug session ID"
                        }
                    },
                    "required": ["sessionId"]
                }
            }),
        ]
    }
}
