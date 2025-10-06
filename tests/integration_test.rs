use debugger_mcp::McpServer;
use serde_json::json;

#[tokio::test]
async fn test_mcp_server_initializes() {
    // Test that we can create an MCP server
    let server = McpServer::new().await;
    assert!(server.is_ok(), "Server should initialize successfully");
}

#[tokio::test]
async fn test_mcp_initialize_request() {
    // This test will verify the initialize handshake
    // For now, we just test basic server creation
    let server = McpServer::new().await.unwrap();
    
    // In a real test, we'd send initialize request and verify response
    // This requires refactoring the server to be testable
    // TODO: Add proper testing infrastructure
}
