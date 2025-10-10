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
}
