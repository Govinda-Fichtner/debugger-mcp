pub mod transport;
pub mod transport_trait;
pub mod protocol;
pub mod resources;
pub mod tools;

use crate::{Error, Result};
use crate::debug::SessionManager;
use transport::StdioTransport;
use protocol::ProtocolHandler;
use tools::ToolsHandler;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

pub struct McpServer {
    transport: StdioTransport,
    handler: ProtocolHandler,
}

impl McpServer {
    pub async fn new() -> Result<Self> {
        info!("Initializing MCP server");

        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let tools_handler = Arc::new(ToolsHandler::new(session_manager));

        let mut handler = ProtocolHandler::new();
        handler.set_tools_handler(tools_handler);

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
