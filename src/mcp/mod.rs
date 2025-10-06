pub mod transport;
pub mod protocol;
pub mod resources;
pub mod tools;

use crate::{Error, Result};
use transport::StdioTransport;
use protocol::{JsonRpcMessage, ProtocolHandler};
use tracing::{info, error};

pub struct McpServer {
    transport: StdioTransport,
    handler: ProtocolHandler,
}

impl McpServer {
    pub async fn new() -> Result<Self> {
        info!("Initializing MCP server");
        Ok(Self {
            transport: StdioTransport::new(),
            handler: ProtocolHandler::new(),
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
