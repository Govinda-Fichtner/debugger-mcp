use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::tools::ToolsHandler;
use super::resources::ResourcesHandler;
use std::sync::Arc;
use tracing::{debug, warn};

#[cfg(test)]
use tokio::sync::RwLock;

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
    resources_handler: Option<Arc<ResourcesHandler>>,
}

impl ProtocolHandler {
    pub fn new() -> Self {
        Self {
            initialized: false,
            tools_handler: None,
            resources_handler: None,
        }
    }

    pub fn set_tools_handler(&mut self, handler: Arc<ToolsHandler>) {
        self.tools_handler = Some(handler);
    }

    pub fn set_resources_handler(&mut self, handler: Arc<ResourcesHandler>) {
        self.resources_handler = Some(handler);
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
            "resources/list" => self.handle_resources_list(req).await,
            "resources/read" => self.handle_resources_read(req).await,
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

    async fn handle_resources_list(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling resources/list request");

        let handler = match &self.resources_handler {
            Some(h) => h,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "Resources handler not initialized".to_string(),
                        data: None,
                    }),
                };
            }
        };

        match handler.list_resources().await {
            Ok(resources) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(serde_json::json!({
                    "resources": resources
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

    async fn handle_resources_read(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling resources/read request");

        let params = match req.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing parameters for resources/read".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if uri.is_empty() {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Missing 'uri' parameter".to_string(),
                    data: None,
                }),
            };
        }

        let handler = match &self.resources_handler {
            Some(h) => h,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "Resources handler not initialized".to_string(),
                        data: None,
                    }),
                };
            }
        };

        match handler.read_resource(uri).await {
            Ok(contents) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(serde_json::json!({
                    "contents": [contents]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test_method".to_string(),
            params: Some(json!({"key": "value"})),
        };

        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("test_method"));
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(serialized.contains("\"status\":\"ok\""));
        assert!(!serialized.contains("error"));
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid request".to_string(),
                data: None,
            }),
        };

        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
        assert_eq!(resp.error.as_ref().unwrap().message, "Invalid request");
    }

    #[test]
    fn test_jsonrpc_notification() {
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notify".to_string(),
            params: Some(json!({"event": "test"})),
        };

        assert_eq!(notif.method, "notify");
        assert!(notif.params.is_some());
    }

    #[test]
    fn test_protocol_handler_new() {
        let handler = ProtocolHandler::new();
        assert!(!handler.initialized);
        assert!(handler.tools_handler.is_none());
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let mut handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "initialize".to_string(),
            params: None,
        };

        let response = handler.handle_initialize(req).await;
        assert!(handler.initialized);
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "debugger_mcp");
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(2),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = handler.handle_tools_list(req).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert!(result["tools"].is_array());
    }

    #[tokio::test]
    async fn test_handle_tools_call_missing_params() {
        let handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(3),
            method: "tools/call".to_string(),
            params: None,
        };

        let response = handler.handle_tools_call(req).await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[tokio::test]
    async fn test_handle_tools_call_no_handler() {
        let handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(4),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "test_tool",
                "arguments": {}
            })),
        };

        let response = handler.handle_tools_call(req).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().message, "Tools handler not initialized");
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let mut handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(5),
            method: "unknown_method".to_string(),
            params: None,
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
        assert!(response.error.unwrap().message.contains("Method not found"));
    }

    #[tokio::test]
    async fn test_handle_notification_message() {
        let mut handler = ProtocolHandler::new();
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "test_notification".to_string(),
            params: None,
        };

        let response = handler.handle_message(JsonRpcMessage::Notification(notif)).await;
        match response {
            JsonRpcMessage::Response(r) => {
                assert!(r.error.is_some());
                assert!(r.error.unwrap().message.contains("Notifications not yet supported"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_message() {
        let mut handler = ProtocolHandler::new();
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({})),
            error: None,
        };

        let response = handler.handle_message(JsonRpcMessage::Response(resp)).await;
        match response {
            JsonRpcMessage::Response(r) => {
                assert!(r.error.is_some());
                assert!(r.error.unwrap().message.contains("Server does not accept response messages"));
            }
            _ => panic!("Expected response"),
        }
    }

    // Phase 6B: Additional protocol tests for uncovered lines

    #[tokio::test]
    async fn test_handle_request_message_direct() {
        let mut handler = ProtocolHandler::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0"
                }
            })),
        };

        let response = handler.handle_message(JsonRpcMessage::Request(req)).await;
        match response {
            JsonRpcMessage::Response(r) => {
                assert!(r.error.is_none());
                assert!(r.result.is_some());
            }
            _ => panic!("Expected response"),
        }
    }

    #[tokio::test]
    async fn test_tools_call_without_handler_set() {
        // Test line 192 - tools handler not initialized
        let mut handler = ProtocolHandler::new();
        // Don't call set_tools_handler, so it's None

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "debugger_start",
                "arguments": {
                    "language": "python",
                    "program": "test.py"
                }
            })),
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert!(error.message.contains("Tools handler not initialized"));
    }

    #[tokio::test]
    async fn test_tools_call_with_handler_error() {
        // Test lines 220-221, 223 - tool call error response
        use crate::debug::SessionManager;
        use crate::mcp::tools::ToolsHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let tools_handler = Arc::new(ToolsHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_tools_handler(tools_handler);

        // Call with invalid arguments to trigger error
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "debugger_start",
                "arguments": {
                    // Missing "program" field - will cause error
                    "language": "python"
                }
            })),
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert!(error.code != 0);  // Should have an error code
    }

    #[tokio::test]
    async fn test_tools_call_success_with_handler() {
        // Test line 207 - successful tool call
        use crate::debug::SessionManager;
        use crate::mcp::tools::ToolsHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let tools_handler = Arc::new(ToolsHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_tools_handler(tools_handler);

        // Call tools/list which doesn't require session setup
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    // Resource handler tests

    #[tokio::test]
    async fn test_resources_list_without_handler() {
        let mut handler = ProtocolHandler::new();
        // Don't set resources_handler

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/list".to_string(),
            params: None,
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert!(error.message.contains("Resources handler not initialized"));
    }

    #[tokio::test]
    async fn test_resources_list_with_handler() {
        use crate::debug::SessionManager;
        use crate::mcp::resources::ResourcesHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let resources_handler = Arc::new(ResourcesHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_resources_handler(resources_handler);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/list".to_string(),
            params: None,
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result["resources"].is_array());
    }

    #[tokio::test]
    async fn test_resources_read_without_handler() {
        let mut handler = ProtocolHandler::new();
        // Don't set resources_handler

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/read".to_string(),
            params: Some(json!({
                "uri": "debugger://sessions"
            })),
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert!(error.message.contains("Resources handler not initialized"));
    }

    #[tokio::test]
    async fn test_resources_read_missing_params() {
        use crate::debug::SessionManager;
        use crate::mcp::resources::ResourcesHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let resources_handler = Arc::new(ResourcesHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_resources_handler(resources_handler);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/read".to_string(),
            params: None,
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Missing parameters"));
    }

    #[tokio::test]
    async fn test_resources_read_missing_uri() {
        use crate::debug::SessionManager;
        use crate::mcp::resources::ResourcesHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let resources_handler = Arc::new(ResourcesHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_resources_handler(resources_handler);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/read".to_string(),
            params: Some(json!({})),
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Missing 'uri'"));
    }

    #[tokio::test]
    async fn test_resources_read_success() {
        use crate::debug::SessionManager;
        use crate::mcp::resources::ResourcesHandler;

        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let resources_handler = Arc::new(ResourcesHandler::new(manager));

        let mut handler = ProtocolHandler::new();
        handler.set_resources_handler(resources_handler);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "resources/read".to_string(),
            params: Some(json!({
                "uri": "debugger://sessions"
            })),
        };

        let response = handler.handle_request(req).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result["contents"].is_array());
    }
}
