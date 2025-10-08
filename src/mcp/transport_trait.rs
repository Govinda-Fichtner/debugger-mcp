use super::protocol::JsonRpcMessage;
use crate::Result;
use async_trait::async_trait;

/// Trait for MCP transport layer to enable testing with mocks
#[async_trait]
pub trait McpTransportTrait: Send + Sync {
    /// Read a JSON-RPC message from the transport
    async fn read_message(&mut self) -> Result<JsonRpcMessage>;

    /// Write a JSON-RPC message to the transport
    async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()>;
}
