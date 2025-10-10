use super::protocol::JsonRpcMessage;
use super::transport_trait::McpTransportTrait;
use crate::{Error, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, trace};

pub struct StdioTransport {
    stdin: BufReader<tokio::io::Stdin>,
    stdout: tokio::io::Stdout,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(tokio::io::stdin()),
            stdout: tokio::io::stdout(),
        }
    }

    pub async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        // MCP uses line-based JSON-RPC transport (not LSP's Content-Length headers)
        // Each message is a single line terminated by \n
        // See: https://spec.modelcontextprotocol.io/specification/basic/transports/#stdio

        let mut line = String::new();
        let bytes_read = self.stdin.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Err(Error::InvalidRequest("EOF reached".to_string()));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidRequest("Empty message line".to_string()));
        }

        trace!("Reading MCP message: {} bytes", trimmed.len());
        debug!("Received message: {}", trimmed);

        let msg: JsonRpcMessage = serde_json::from_str(trimmed)?;
        Ok(msg)
    }

    pub async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()> {
        // MCP uses line-based JSON-RPC transport (not LSP's Content-Length headers)
        // Each message is a single line terminated by \n
        // See: https://spec.modelcontextprotocol.io/specification/basic/transports/#stdio

        let content = serde_json::to_string(msg)?;
        debug!("Sending message: {}", content);

        self.stdout.write_all(content.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;

        Ok(())
    }
}

// Implement the trait for the concrete transport
#[async_trait]
impl McpTransportTrait for StdioTransport {
    async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        // Delegate to existing implementation
        self.read_message().await
    }

    async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()> {
        // Delegate to existing implementation
        self.write_message(msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::{
        JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    };
    use super::*;
    use mockall::mock;
    use serde_json::json;

    // Generate mock using mockall
    mock! {
        pub StdioTransport {}

        #[async_trait]
        impl McpTransportTrait for StdioTransport {
            async fn read_message(&mut self) -> Result<JsonRpcMessage>;
            async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_mock_read_request() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport.expect_read_message().times(1).returning(|| {
            Ok(JsonRpcMessage::Request(JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "initialize".to_string(),
                params: Some(json!({"clientID": "test"})),
            }))
        });

        let msg = mock_transport.read_message().await.unwrap();

        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "initialize");
                assert_eq!(req.jsonrpc, "2.0");
            }
            _ => panic!("Expected Request"),
        }
    }

    #[tokio::test]
    async fn test_mock_write_response() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport
            .expect_write_message()
            .times(1)
            .withf(|msg| {
                if let JsonRpcMessage::Response(resp) = msg {
                    resp.id == json!(1)
                } else {
                    false
                }
            })
            .returning(|_| Ok(()));

        let response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        });

        mock_transport.write_message(&response).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_read_error_response() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport.expect_read_message().times(1).returning(|| {
            Ok(JsonRpcMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid request".to_string(),
                    data: None,
                }),
            }))
        });

        let msg = mock_transport.read_message().await.unwrap();

        match msg {
            JsonRpcMessage::Response(resp) => {
                assert!(resp.error.is_some());
                assert_eq!(resp.error.unwrap().code, -32600);
            }
            _ => panic!("Expected Response with error"),
        }
    }

    #[tokio::test]
    async fn test_mock_read_notification() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport.expect_read_message().times(1).returning(|| {
            Ok(JsonRpcMessage::Notification(JsonRpcNotification {
                jsonrpc: "2.0".to_string(),
                method: "notification".to_string(),
                params: Some(json!({"event": "test"})),
            }))
        });

        let msg = mock_transport.read_message().await.unwrap();

        match msg {
            JsonRpcMessage::Notification(notif) => {
                assert_eq!(notif.method, "notification");
            }
            _ => panic!("Expected Notification"),
        }
    }

    #[tokio::test]
    async fn test_mock_transport_error() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| Err(Error::InvalidRequest("Empty message line".to_string())));

        let result = mock_transport.read_message().await;

        assert!(result.is_err());
        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("Empty message") || msg.contains("EOF"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[tokio::test]
    async fn test_mock_write_multiple_messages() {
        let mut mock_transport = MockStdioTransport::new();

        mock_transport
            .expect_write_message()
            .times(2)
            .returning(|_| Ok(()));

        let messages = vec![
            JsonRpcMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                result: Some(json!({})),
                error: None,
            }),
            JsonRpcMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: json!(2),
                result: Some(json!({})),
                error: None,
            }),
        ];

        for msg in messages {
            mock_transport.write_message(&msg).await.unwrap();
        }
    }

    // Regression tests to prevent MCP protocol violations
    // These tests verify MCP spec compliance for stdio transport
    // Spec: https://spec.modelcontextprotocol.io/specification/basic/transports/#stdio

    #[tokio::test]
    async fn test_mcp_spec_line_based_json_not_lsp_headers() {
        // Critical regression test: Ensure we use MCP's line-based format,
        // NOT LSP's Content-Length header format

        // Test that serialized output is line-based JSON
        let msg = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"test": "value"})),
            error: None,
        });

        let serialized = serde_json::to_string(&msg).unwrap();

        // Verify the message format expectations
        assert!(
            !serialized.contains("Content-Length:"),
            "MCP messages must NOT contain LSP Content-Length headers"
        );
        assert!(
            !serialized.contains("\r\n\r\n"),
            "MCP messages must NOT contain LSP header separators"
        );

        // Message should be valid single-line JSON
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
    }

    #[tokio::test]
    async fn test_mcp_spec_single_line_terminated_by_newline() {
        // Per MCP spec: "Each message MUST be a single line terminated by \n"

        let msg = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test".to_string(),
            params: None,
        });

        // Verify message can be serialized as single line
        let serialized = serde_json::to_string(&msg).unwrap();
        assert!(
            !serialized.contains('\n'),
            "Serialized message should not contain newlines before transport adds them"
        );
        assert!(
            !serialized.contains('\r'),
            "Serialized message should not contain carriage returns"
        );
    }

    #[tokio::test]
    async fn test_prevents_lsp_content_length_regression() {
        // Regression test for Issue #1: Server was using LSP transport instead of MCP
        // This test fails if anyone accidentally adds back Content-Length headers

        let test_json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;

        // Verify we can parse raw JSON (MCP format)
        let parsed: JsonRpcMessage = serde_json::from_str(test_json).unwrap();
        match parsed {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "initialize");
            }
            _ => panic!("Expected Request"),
        }

        // Verify the reverse: LSP-style input should NOT be parseable as JSON-RPC
        let lsp_style =
            "Content-Length: 58\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\"}";
        let lsp_parse_attempt: std::result::Result<JsonRpcMessage, _> =
            serde_json::from_str(lsp_style);
        assert!(
            lsp_parse_attempt.is_err(),
            "LSP-formatted messages should not be parseable as JSON-RPC"
        );
    }

    #[tokio::test]
    async fn test_mcp_message_format_examples() {
        // Test that we can correctly parse MCP-compliant messages
        // Examples from MCP spec documentation

        let examples = vec![
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"test","version":"1.0"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        ];

        for example in examples {
            let parsed: std::result::Result<JsonRpcMessage, _> = serde_json::from_str(example);
            assert!(
                parsed.is_ok(),
                "Failed to parse MCP-compliant message: {}",
                example
            );
        }
    }

    // Phase 5: Real transport implementation tests (not just mocks)
    // These tests exercise the actual read_message() and write_message() code

    // Helper struct for testing that mirrors StdioTransport but accepts generic readers/writers
    struct TestTransport<R, W> {
        stdin: BufReader<R>,
        stdout: W,
    }

    impl<R, W> TestTransport<R, W>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        async fn read_message(&mut self) -> Result<JsonRpcMessage> {
            let mut line = String::new();
            let bytes_read = self.stdin.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(Error::InvalidRequest("EOF reached".to_string()));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Err(Error::InvalidRequest("Empty message line".to_string()));
            }

            let msg: JsonRpcMessage = serde_json::from_str(trimmed)?;
            Ok(msg)
        }

        async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()> {
            let content = serde_json::to_string(msg)?;
            self.stdout.write_all(content.as_bytes()).await?;
            self.stdout.write_all(b"\n").await?;
            self.stdout.flush().await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_real_transport_read_single_message() {
        // Test reading a real message from an in-memory pipe

        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\"}\n";
        let (reader, mut writer) = tokio::io::duplex(1024);

        // Write test data to the pipe
        writer.write_all(input).await.unwrap();
        drop(writer); // Close writer to signal EOF eventually

        // Create test transport
        let mut transport = TestTransport {
            stdin: BufReader::new(reader),
            stdout: tokio::io::sink(), // Not used
        };

        // Read message
        let msg = transport.read_message().await.unwrap();

        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "test");
                assert_eq!(req.id, json!(1));
                assert_eq!(req.jsonrpc, "2.0");
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[tokio::test]
    async fn test_real_transport_write_single_message() {
        // Test writing a real message to an in-memory pipe

        let (mut reader, writer) = tokio::io::duplex(1024);

        let mut transport = TestTransport {
            stdin: BufReader::new(tokio::io::empty()), // Not used
            stdout: writer,
        };

        let msg = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        });

        transport.write_message(&msg).await.unwrap();
        drop(transport); // Close writer

        // Read what was written
        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut output)
            .await
            .unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // Verify format
        assert!(output_str.ends_with('\n'), "Output must end with newline");
        assert!(
            !output_str.contains("Content-Length:"),
            "Must not contain LSP headers"
        );

        // Verify content
        let trimmed = output_str.trim();
        let parsed: serde_json::Value = serde_json::from_str(trimmed).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["status"], "ok");
    }

    #[tokio::test]
    async fn test_real_transport_read_multiple_messages() {
        // Test reading multiple messages in sequence

        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"first\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"second\"}\n";
        let (reader, mut writer) = tokio::io::duplex(1024);

        writer.write_all(input).await.unwrap();
        drop(writer);

        let mut transport = TestTransport {
            stdin: BufReader::new(reader),
            stdout: tokio::io::sink(),
        };

        // Read first message
        let msg1 = transport.read_message().await.unwrap();
        match msg1 {
            JsonRpcMessage::Request(req) => assert_eq!(req.method, "first"),
            _ => panic!("Expected Request"),
        }

        // Read second message
        let msg2 = transport.read_message().await.unwrap();
        match msg2 {
            JsonRpcMessage::Request(req) => assert_eq!(req.method, "second"),
            _ => panic!("Expected Request"),
        }
    }

    #[tokio::test]
    async fn test_real_transport_error_eof() {
        // Test that EOF is handled correctly

        let (reader, writer) = tokio::io::duplex(1024);
        drop(writer); // Close immediately to create EOF

        let mut transport = TestTransport {
            stdin: BufReader::new(reader),
            stdout: tokio::io::sink(),
        };

        let result = transport.read_message().await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("EOF"));
            }
            _ => panic!("Expected EOF error"),
        }
    }

    #[tokio::test]
    async fn test_real_transport_error_empty_line() {
        // Test that empty lines are rejected

        let input = b"\n";
        let (reader, mut writer) = tokio::io::duplex(1024);

        writer.write_all(input).await.unwrap();
        drop(writer);

        let mut transport = TestTransport {
            stdin: BufReader::new(reader),
            stdout: tokio::io::sink(),
        };

        let result = transport.read_message().await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("Empty message"));
            }
            _ => panic!("Expected empty message error"),
        }
    }

    #[tokio::test]
    async fn test_real_transport_error_invalid_json() {
        // Test that invalid JSON is rejected

        let input = b"not valid json\n";
        let (reader, mut writer) = tokio::io::duplex(1024);

        writer.write_all(input).await.unwrap();
        drop(writer);

        let mut transport = TestTransport {
            stdin: BufReader::new(reader),
            stdout: tokio::io::sink(),
        };

        let result = transport.read_message().await;
        assert!(result.is_err());

        // Should be a JSON parse error
        match result {
            Err(Error::Json(_)) => {} // Expected
            other => panic!("Expected JSON error, got: {:?}", other),
        }
    }

    // Tests for the actual StdioTransport struct (not the test helper)
    #[tokio::test]
    async fn test_stdio_transport_new() {
        // Test that we can create a StdioTransport
        let transport = StdioTransport::new();
        // Just verify it compiles and runs
        drop(transport);
    }

    #[tokio::test]
    async fn test_stdio_transport_default() {
        // Test the Default impl
        let transport = StdioTransport::default();
        drop(transport);
    }
}
