pub mod error;
pub mod mcp;
pub mod debug;
pub mod dap;
pub mod adapters;
pub mod process;

pub use error::Error;
pub use mcp::McpServer;

pub type Result<T> = std::result::Result<T, Error>;

pub async fn serve() -> Result<()> {
    let server = McpServer::new().await?;
    server.run().await
}
