# Testing Strategy Implementation Example

This document provides concrete code examples for implementing the testing strategy outlined in `TESTING_STRATEGY.md`.

## Example 1: Mock DAP Transport with Mockall

### Step 1: Add Dependencies

```toml
# Cargo.toml
[dev-dependencies]
mockall = "0.13.1"
assert_matches = "1.5.0"
tokio-test = "0.4.4"  # Already present
```

### Step 2: Create Transport Trait

```rust
// src/dap/transport_trait.rs
use crate::{Error, Result};
use super::types::Message;
use async_trait::async_trait;

#[async_trait]
pub trait DapTransportTrait: Send + Sync {
    async fn read_message(&mut self) -> Result<Message>;
    async fn write_message(&mut self, msg: &Message) -> Result<()>;
}
```

### Step 3: Implement Trait for Existing Transport

```rust
// src/dap/transport.rs
use super::transport_trait::DapTransportTrait;
use async_trait::async_trait;

// Existing struct stays the same
pub struct DapTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

// Add trait implementation
#[async_trait]
impl DapTransportTrait for DapTransport {
    async fn read_message(&mut self) -> Result<Message> {
        // Move existing implementation here (no changes)
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).await?;
            if line == "\r\n" || line == "\n" {
                break;
            }
            headers.push_str(&line);
        }

        let content_length = headers
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .ok_or_else(|| Error::Dap("Missing Content-Length header".to_string()))?;

        let mut buffer = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.stdout, &mut buffer).await?;

        let content = String::from_utf8(buffer)
            .map_err(|e| Error::Dap(format!("Invalid UTF-8: {}", e)))?;

        let msg: Message = serde_json::from_str(&content)
            .map_err(|e| Error::Dap(format!("Failed to parse DAP message: {}", e)))?;

        Ok(msg)
    }

    async fn write_message(&mut self, msg: &Message) -> Result<()> {
        // Move existing implementation here (no changes)
        let content = serde_json::to_string(msg)
            .map_err(|e| Error::Dap(format!("Failed to serialize DAP message: {}", e)))?;

        let headers = format!("Content-Length: {}\r\n\r\n", content.len());

        self.stdin.write_all(headers.as_bytes()).await?;
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }
}
```

### Step 4: Add Tests with Mock

```rust
// src/dap/transport.rs - Add at end of file
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use serde_json::json;

    // Generate mock using mockall
    mock! {
        pub DapTransport {}

        #[async_trait]
        impl DapTransportTrait for DapTransport {
            async fn read_message(&mut self) -> Result<Message>;
            async fn write_message(&mut self, msg: &Message) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_mock_transport_read_response() {
        let mut mock_transport = MockDapTransport::new();

        // Setup expectation
        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| {
                Ok(Message::Response(Response {
                    seq: 1,
                    request_seq: 1,
                    command: "initialize".to_string(),
                    success: true,
                    message: None,
                    body: Some(json!({
                        "capabilities": {
                            "supportsConfigurationDoneRequest": true
                        }
                    })),
                }))
            });

        // Execute
        let msg = mock_transport.read_message().await.unwrap();

        // Verify
        match msg {
            Message::Response(resp) => {
                assert_eq!(resp.command, "initialize");
                assert!(resp.success);
                assert!(resp.body.is_some());
            }
            _ => panic!("Expected Response"),
        }
    }

    #[tokio::test]
    async fn test_mock_transport_write_request() {
        let mut mock_transport = MockDapTransport::new();

        // Setup expectation with argument matching
        mock_transport
            .expect_write_message()
            .times(1)
            .withf(|msg| {
                if let Message::Request(req) = msg {
                    req.command == "launch"
                } else {
                    false
                }
            })
            .returning(|_| Ok(()));

        // Execute
        let request = Message::Request(Request {
            seq: 1,
            command: "launch".to_string(),
            arguments: Some(json!({"program": "test.py"})),
        });

        mock_transport.write_message(&request).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_transport_error_handling() {
        let mut mock_transport = MockDapTransport::new();

        // Setup expectation to return error
        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| Err(Error::Dap("Connection closed".to_string())));

        // Execute
        let result = mock_transport.read_message().await;

        // Verify error
        assert!(result.is_err());
        match result {
            Err(Error::Dap(msg)) => assert_eq!(msg, "Connection closed"),
            _ => panic!("Expected Dap error"),
        }
    }
}
```

---

## Example 2: Testing DapClient with In-Memory Channels

This example shows how to test the DAP client without spawning real processes.

### Step 1: Refactor DapClient Constructor

```rust
// src/dap/client.rs
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream};

impl DapClient {
    // Existing production constructor (unchanged API)
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        // ... existing implementation
    }

    // New: Testable constructor using in-memory I/O
    #[cfg(test)]
    pub async fn new_with_transport(
        stdin: impl AsyncWrite + Send + Unpin + 'static,
        stdout: impl AsyncRead + Send + Unpin + 'static,
    ) -> Result<Self> {
        use tokio::process::{ChildStdin, ChildStdout};
        use std::os::unix::io::{AsRawFd, FromRawFd};

        // Wrap in-memory streams as DapTransport
        let transport = Arc::new(RwLock::new(DapTransport::new(
            stdin,
            stdout,
        )));

        let seq_counter = Arc::new(AtomicI32::new(1));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let client = Self {
            transport: transport.clone(),
            seq_counter,
            pending_requests: pending_requests.clone(),
            event_tx,
        };

        // Spawn message handler
        tokio::spawn(Self::message_handler(
            transport,
            pending_requests,
            event_rx,
        ));

        Ok(client)
    }
}
```

### Step 2: Write Tests with Simulated DAP Adapter

```rust
// src/dap/client.rs - tests module
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncWriteExt};
    use serde_json::json;

    // Helper: Simulate DAP adapter that responds to requests
    async fn simulate_dap_adapter(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
    ) {
        use tokio::io::AsyncReadExt;

        loop {
            // Read Content-Length header
            let mut headers = String::new();
            let mut buf = [0u8; 1];
            loop {
                reader.read_exact(&mut buf).await.unwrap();
                headers.push(buf[0] as char);
                if headers.ends_with("\r\n\r\n") {
                    break;
                }
            }

            // Parse content length
            let content_length: usize = headers
                .lines()
                .find(|line| line.starts_with("Content-Length:"))
                .and_then(|line| line.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .unwrap();

            // Read message body
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).await.unwrap();

            let request: Request = serde_json::from_slice(&body).unwrap();

            // Respond based on command
            let response = match request.command.as_str() {
                "initialize" => Response {
                    seq: 1,
                    request_seq: request.seq,
                    command: "initialize".to_string(),
                    success: true,
                    message: None,
                    body: Some(json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsFunctionBreakpoints": false,
                    })),
                },
                "launch" => Response {
                    seq: 2,
                    request_seq: request.seq,
                    command: "launch".to_string(),
                    success: true,
                    message: None,
                    body: None,
                },
                _ => Response {
                    seq: 0,
                    request_seq: request.seq,
                    command: request.command.clone(),
                    success: false,
                    message: Some("Unknown command".to_string()),
                    body: None,
                },
            };

            // Write response
            let response_msg = Message::Response(response);
            let response_json = serde_json::to_string(&response_msg).unwrap();
            let header = format!("Content-Length: {}\r\n\r\n", response_json.len());

            writer.write_all(header.as_bytes()).await.unwrap();
            writer.write_all(response_json.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_dap_client_initialize() {
        // Create bidirectional in-memory channels
        let (client_writer, adapter_reader) = duplex(4096);
        let (adapter_writer, client_reader) = duplex(4096);

        // Spawn simulated DAP adapter
        tokio::spawn(simulate_dap_adapter(adapter_reader, adapter_writer));

        // Create client with in-memory transport
        let client = DapClient::new_with_transport(client_writer, client_reader)
            .await
            .unwrap();

        // Test initialize
        let capabilities = client.initialize("test-adapter").await.unwrap();

        assert_eq!(
            capabilities.supports_configuration_done_request,
            Some(true)
        );
        assert_eq!(
            capabilities.supports_function_breakpoints,
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_dap_client_launch() {
        let (client_writer, adapter_reader) = duplex(4096);
        let (adapter_writer, client_reader) = duplex(4096);

        tokio::spawn(simulate_dap_adapter(adapter_reader, adapter_writer));

        let client = DapClient::new_with_transport(client_writer, client_reader)
            .await
            .unwrap();

        let launch_args = json!({
            "program": "/path/to/program.py",
            "args": ["--verbose"],
        });

        // Should succeed
        client.launch(launch_args).await.unwrap();
    }

    #[tokio::test]
    async fn test_dap_client_concurrent_requests() {
        let (client_writer, adapter_reader) = duplex(4096);
        let (adapter_writer, client_reader) = duplex(4096);

        tokio::spawn(simulate_dap_adapter(adapter_reader, adapter_writer));

        let client = Arc::new(
            DapClient::new_with_transport(client_writer, client_reader)
                .await
                .unwrap(),
        );

        // Send multiple requests concurrently
        let client1 = client.clone();
        let client2 = client.clone();

        let (result1, result2) = tokio::join!(
            client1.initialize("adapter1"),
            client2.send_request("configurationDone", None),
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
```

---

## Example 3: MCP Transport Testing with tokio-test

```rust
// src/mcp/transport.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::io::Builder;
    use serde_json::json;

    #[tokio::test]
    async fn test_read_json_rpc_request() {
        // Prepare mock stdin data
        let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientID":"test"}}"#;
        let header = format!("Content-Length: {}\r\n\r\n", json_content.len());

        let mock_stdin = Builder::new()
            .read(header.as_bytes())
            .read(json_content.as_bytes())
            .build();

        let mock_stdout = Builder::new().build();

        let mut transport = StdioTransport {
            stdin: BufReader::new(mock_stdin),
            stdout: mock_stdout,
        };

        // Read message
        let msg = transport.read_message().await.unwrap();

        // Verify
        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.jsonrpc, "2.0");
                assert_eq!(req.id, json!(1));
                assert_eq!(req.method, "initialize");
                assert!(req.params.is_some());
            }
            _ => panic!("Expected Request"),
        }
    }

    #[tokio::test]
    async fn test_write_json_rpc_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let response_json = serde_json::to_string(&JsonRpcMessage::Response(response.clone())).unwrap();
        let expected_header = format!("Content-Length: {}\r\n\r\n", response_json.len());

        let mock_stdin = Builder::new().build();
        let mock_stdout = Builder::new()
            .write(expected_header.as_bytes())
            .write(response_json.as_bytes())
            .build();

        let mut transport = StdioTransport {
            stdin: BufReader::new(mock_stdin),
            stdout: mock_stdout,
        };

        // Write message
        transport
            .write_message(&JsonRpcMessage::Response(response))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_read_invalid_content_length() {
        let mock_stdin = Builder::new()
            .read(b"Invalid-Header: test\r\n\r\n")
            .build();

        let mock_stdout = Builder::new().build();

        let mut transport = StdioTransport {
            stdin: BufReader::new(mock_stdin),
            stdout: mock_stdout,
        };

        let result = transport.read_message().await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("Content-Length"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }
}
```

---

## Example 4: Integration Test with Fake Adapter Binary

### Step 1: Create Fake Adapter Binary

```rust
// tests/bin/fake_dap_adapter.rs
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.unwrap();

        // Parse Content-Length
        if let Some(content_len) = line.strip_prefix("Content-Length: ") {
            let len: usize = content_len.trim().parse().unwrap();

            // Skip empty line
            stdin.lock().lines().next();

            // Read JSON body
            let mut buffer = vec![0u8; len];
            io::Read::read_exact(&mut stdin.lock(), &mut buffer).unwrap();

            let request: serde_json::Value = serde_json::from_slice(&buffer).unwrap();

            // Simple response
            let response = serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": request["seq"],
                "command": request["command"],
                "success": true,
                "body": {"capabilities": {}}
            });

            let response_str = serde_json::to_string(&response).unwrap();
            writeln!(stdout, "Content-Length: {}\r", response_str.len()).unwrap();
            writeln!(stdout).unwrap();
            write!(stdout, "{}", response_str).unwrap();
            stdout.flush().unwrap();
        }
    }
}
```

### Step 2: Configure Cargo for Test Binary

```toml
# Cargo.toml
[[bin]]
name = "fake_dap_adapter"
path = "tests/bin/fake_dap_adapter.rs"
test = false
```

### Step 3: Integration Test

```rust
// tests/integration_test.rs
use debugger_mcp::debug::SessionManager;
use std::env;

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_full_session_with_fake_adapter() {
    // Get path to fake adapter binary
    let adapter_path = env::var("CARGO_BIN_EXE_fake_dap_adapter")
        .expect("fake_dap_adapter binary not found");

    let manager = SessionManager::new();

    // This would use the fake adapter instead of real python debugger
    // Note: Requires modifying PythonAdapter to accept custom command
    let session_id = manager
        .create_session_with_adapter(
            "test",
            "test.py".to_string(),
            vec![],
            None,
            &adapter_path,
            &[],
        )
        .await
        .unwrap();

    let session = manager.get_session(&session_id).await.unwrap();

    // Test full workflow
    session.set_breakpoint("test.py".to_string(), 10).await.unwrap();
    session.continue_execution().await.unwrap();

    manager.remove_session(&session_id).await.unwrap();
}
```

---

## Summary

These examples demonstrate:

1. **Mockall for traits**: Type-safe mocking with compile-time guarantees
2. **In-memory channels**: Testing async I/O without real processes
3. **tokio-test builders**: Simulating stdin/stdout for transport tests
4. **Fake binaries**: Minimal DAP adapter for integration tests

Each approach targets different coverage gaps while maintaining test speed and reliability.
