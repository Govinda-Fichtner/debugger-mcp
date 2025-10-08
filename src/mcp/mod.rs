pub mod protocol;
pub mod resources;
pub mod tools;
pub mod transport;
pub mod transport_trait;

use crate::debug::SessionManager;
use crate::Result;
use protocol::ProtocolHandler;
use resources::ResourcesHandler;
use std::sync::Arc;
use tokio::sync::RwLock;
use tools::ToolsHandler;
use tracing::{error, info};
use transport::StdioTransport;

pub struct McpServer {
    transport: StdioTransport,
    handler: ProtocolHandler,
}

impl McpServer {
    pub async fn new() -> Result<Self> {
        info!("Initializing MCP server");

        let session_manager = Arc::new(RwLock::new(SessionManager::new()));

        // Create tools handler
        let tools_handler = Arc::new(ToolsHandler::new(Arc::clone(&session_manager)));

        // Create resources handler
        let resources_handler = Arc::new(ResourcesHandler::new(Arc::clone(&session_manager)));

        let mut handler = ProtocolHandler::new();
        handler.set_tools_handler(tools_handler);
        handler.set_resources_handler(resources_handler);

        Ok(Self {
            transport: StdioTransport::new(),
            handler,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        info!("Starting MCP server");

        loop {
            match self.transport.read_message().await {
                Ok(msg) => {
                    let response = self.handler.handle_message(msg).await;
                    if let Err(e) = self.transport.write_message(&response).await {
                        error!("Failed to write response: {}", e);
                        return Err(e);
                    }
                }
                Err(e) => {
                    error!("Failed to read message: {}", e);
                    return Err(e);
                }
            }
        }
    }
}
