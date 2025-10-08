use super::types::Message;
use crate::Result;
use async_trait::async_trait;

/// Trait for DAP transport layer to enable testing with mocks
#[async_trait]
pub trait DapTransportTrait: Send + Sync {
    /// Read a DAP protocol message from the transport
    async fn read_message(&mut self) -> Result<Message>;

    /// Write a DAP protocol message to the transport
    async fn write_message(&mut self, msg: &Message) -> Result<()>;
}
