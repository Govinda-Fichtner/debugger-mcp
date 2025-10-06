use crate::{Error, Result};
use crate::debug::SessionManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

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
}

impl ResourcesHandler {
    pub fn new(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { session_manager }
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
        ];

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
        // Parse URI
        if !uri.starts_with("debugger://") {
            return Err(Error::InvalidRequest(format!("Invalid resource URI: {}", uri)));
        }

        let path = &uri["debugger://".len()..];

        if path == "sessions" {
            // List all sessions
            self.read_sessions_list().await
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
                _ => Err(Error::InvalidRequest(format!("Unknown resource path: {}", path))),
            }
        } else {
            Err(Error::InvalidRequest(format!("Unknown resource: {}", uri)))
        }
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
        vec![
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
        ]
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
        assert!(resources.len() >= 1); // At least the list resource itself
    }

    #[tokio::test]
    async fn test_list_resources_empty() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let resources = handler.list_resources().await.unwrap();

        // Should always have the sessions list resource
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "debugger://sessions");
        assert_eq!(resources[0].name, "Debug Sessions");
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

        let result = handler.read_resource("debugger://sessions/nonexistent-id").await;
        assert!(result.is_err());

        match result {
            Err(Error::SessionNotFound(_)) => {},
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_read_stack_trace_not_found() {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let handler = ResourcesHandler::new(manager);

        let result = handler.read_resource("debugger://sessions/nonexistent-id/stackTrace").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_resource_templates() {
        let templates = ResourcesHandler::list_resource_templates();

        assert_eq!(templates.len(), 3);

        // Check first template
        assert!(templates[0]["uriTemplate"].as_str().unwrap().contains("sessions"));
        assert!(templates[0]["name"].as_str().is_some());
        assert!(templates[0]["mimeType"].as_str().unwrap() == "application/json");
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
