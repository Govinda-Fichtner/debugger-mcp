use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{Error, Result};
use super::tools::ToolsHandler;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub struct ProtocolHandler {
    initialized: bool,
    tools_handler: Option<Arc<ToolsHandler>>,
}

impl ProtocolHandler {
    pub fn new() -> Self {
        Self {
            initialized: false,
            tools_handler: None,
        }
    }

    pub fn set_tools_handler(&mut self, handler: Arc<ToolsHandler>) {
        self.tools_handler = Some(handler);
    }

    pub async fn handle_message(&mut self, msg: JsonRpcMessage) -> JsonRpcMessage {
        match msg {
            JsonRpcMessage::Request(req) => {
                JsonRpcMessage::Response(self.handle_request(req).await)
            }
            JsonRpcMessage::Notification(notif) => {
                self.handle_notification(notif).await;
                // Notifications don't get responses, return a dummy response
                // In practice, we'd need to handle this differently
                JsonRpcMessage::Response(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: "Notifications not yet supported".to_string(),
                        data: None,
                    }),
                })
            }
            JsonRpcMessage::Response(_) => {
                warn!("Received response message, ignoring");
                JsonRpcMessage::Response(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: "Server does not accept response messages".to_string(),
                        data: None,
                    }),
                })
            }
        }
    }

    async fn handle_request(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling request: {}", req.method);

        match req.method.as_str() {
            "initialize" => self.handle_initialize(req).await,
            "tools/list" => self.handle_tools_list(req).await,
            "tools/call" => self.handle_tools_call(req).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling initialize request");

        self.initialized = true;

        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
            },
            "serverInfo": {
                "name": "debugger_mcp",
                "version": "0.1.0",
            },
        });

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(result),
            error: None,
        }
    }

    async fn handle_notification(&mut self, _notif: JsonRpcNotification) {
        // Handle notifications here
    }

    async fn handle_tools_list(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling tools/list request");

        let tools = ToolsHandler::list_tools();

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(serde_json::json!({
                "tools": tools
            })),
            error: None,
        }
    }

    async fn handle_tools_call(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling tools/call request");

        let params = match req.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        let handler = match &self.tools_handler {
            Some(h) => h,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "Tools handler not initialized".to_string(),
                        data: None,
                    }),
                };
            }
        };

        match handler.handle_tool(name, arguments).await {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                    }]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: e.error_code(),
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }
}
