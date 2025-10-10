pub mod adapters;
pub mod dap;
pub mod debug;
pub mod error;
pub mod mcp;
pub mod process;

pub use error::Error;
pub use mcp::McpServer;

pub type Result<T> = std::result::Result<T, Error>;

pub async fn serve() -> Result<()> {
    let server = McpServer::new().await?;
    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_type_alias() {
        // Test that our Result type works correctly
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());
        if let Ok(value) = ok_result {
            assert_eq!(value, 42);
        }

        let err_result: Result<i32> = Err(Error::InvalidRequest("test error".to_string()));
        assert!(err_result.is_err());
    }

    #[test]
    fn test_error_reexport() {
        // Test that Error is properly re-exported
        let error = Error::SessionNotFound("test_session".to_string());
        assert!(matches!(error, Error::SessionNotFound(_)));
    }

    #[tokio::test]
    async fn test_serve_creates_server() {
        // Test that serve() initializes an MCP server
        // This test verifies the server initialization but doesn't run the event loop
        // We can't fully test serve() because it blocks on server.run() which never returns
        let server_result = McpServer::new().await;
        assert!(server_result.is_ok(), "Should be able to create MCP server");
    }

    #[test]
    fn test_module_structure() {
        // Verify all public modules are accessible
        // This ensures our module exports are correct

        // These are just compile-time checks that the modules exist
        let _: Option<McpServer> = None;
        let _: Option<Error> = None;
        let _: Option<Result<()>> = None;
    }
}
