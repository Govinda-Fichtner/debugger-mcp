use super::transport_trait::DapTransportTrait;
use super::types::Message;
use crate::{Error, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{ChildStdin, ChildStdout};
use tracing::{debug, trace};

/// DAP Transport - supports both STDIO and TCP socket
pub enum DapTransport {
    /// STDIO transport (used by Python/debugpy)
    Stdio {
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    },
    /// TCP socket transport (used by Ruby/rdbg)
    Socket { stream: BufReader<TcpStream> },
}

impl DapTransport {
    /// Create a new STDIO transport (for Python/debugpy)
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self::Stdio {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    /// Create a new TCP socket transport (for Ruby/rdbg)
    pub fn new_socket(stream: TcpStream) -> Self {
        Self::Socket {
            stream: BufReader::new(stream),
        }
    }

    pub async fn read_message(&mut self) -> Result<Message> {
        // Read from either stdio or socket
        let (_headers, content) = match self {
            Self::Stdio { stdout, .. } => Self::read_from_stream(stdout).await?,
            Self::Socket { stream } => Self::read_from_stream(stream).await?,
        };

        debug!("DAP received: {}", content);

        let msg: Message = serde_json::from_str(&content)
            .map_err(|e| Error::Dap(format!("Failed to parse DAP message: {}", e)))?;

        Ok(msg)
    }

    /// Helper to read DAP message from any async reader
    async fn read_from_stream<R: AsyncBufReadExt + tokio::io::AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<(String, String)> {
        // Read Content-Length header
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            if line == "\r\n" || line == "\n" {
                break;
            }

            headers.push_str(&line);
        }

        // Parse Content-Length
        let content_length = headers
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .ok_or_else(|| Error::Dap("Missing Content-Length header".to_string()))?;

        trace!(
            "DAP: Reading message with Content-Length: {}",
            content_length
        );

        // Read content
        let mut buffer = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut buffer).await?;

        let content =
            String::from_utf8(buffer).map_err(|e| Error::Dap(format!("Invalid UTF-8: {}", e)))?;

        Ok((headers, content))
    }

    pub async fn write_message(&mut self, msg: &Message) -> Result<()> {
        let content = serde_json::to_string(msg)
            .map_err(|e| Error::Dap(format!("Failed to serialize DAP message: {}", e)))?;

        debug!("DAP sending: {}", content);

        let headers = format!("Content-Length: {}\r\n\r\n", content.len());

        // Write to either stdio or socket
        match self {
            Self::Stdio { stdin, .. } => {
                stdin.write_all(headers.as_bytes()).await?;
                stdin.write_all(content.as_bytes()).await?;
                stdin.flush().await?;
            }
            Self::Socket { stream } => {
                let inner_stream = stream.get_mut();
                inner_stream.write_all(headers.as_bytes()).await?;
                inner_stream.write_all(content.as_bytes()).await?;
                inner_stream.flush().await?;
            }
        }

        Ok(())
    }
}

// Implement the trait for the concrete transport
#[async_trait]
impl DapTransportTrait for DapTransport {
    async fn read_message(&mut self) -> Result<Message> {
        // Delegate to existing implementation
        self.read_message().await
    }

    async fn write_message(&mut self, msg: &Message) -> Result<()> {
        // Delegate to existing implementation
        self.write_message(msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{Event, Request, Response};
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
    async fn test_mock_read_initialize_response() {
        let mut mock_transport = MockDapTransport::new();

        mock_transport.expect_read_message().times(1).returning(|| {
            Ok(Message::Response(Response {
                seq: 1,
                request_seq: 1,
                command: "initialize".to_string(),
                success: true,
                message: None,
                body: Some(json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsFunctionBreakpoints": false,
                })),
            }))
        });

        let msg = mock_transport.read_message().await.unwrap();

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
    async fn test_mock_write_launch_request() {
        let mut mock_transport = MockDapTransport::new();

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

        let request = Message::Request(Request {
            seq: 1,
            command: "launch".to_string(),
            arguments: Some(json!({"program": "test.py"})),
        });

        mock_transport.write_message(&request).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_read_error() {
        let mut mock_transport = MockDapTransport::new();

        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| Err(Error::Dap("Connection closed".to_string())));

        let result = mock_transport.read_message().await;

        assert!(result.is_err());
        match result {
            Err(Error::Dap(msg)) => assert_eq!(msg, "Connection closed"),
            _ => panic!("Expected Dap error"),
        }
    }

    #[tokio::test]
    async fn test_mock_read_event() {
        let mut mock_transport = MockDapTransport::new();

        mock_transport.expect_read_message().times(1).returning(|| {
            Ok(Message::Event(Event {
                seq: 1,
                event: "stopped".to_string(),
                body: Some(json!({
                    "reason": "breakpoint",
                    "threadId": 1,
                })),
            }))
        });

        let msg = mock_transport.read_message().await.unwrap();

        match msg {
            Message::Event(evt) => {
                assert_eq!(evt.event, "stopped");
                assert!(evt.body.is_some());
            }
            _ => panic!("Expected Event"),
        }
    }

    #[tokio::test]
    async fn test_mock_write_multiple_requests() {
        let mut mock_transport = MockDapTransport::new();

        mock_transport
            .expect_write_message()
            .times(3)
            .returning(|_| Ok(()));

        let commands = ["initialize", "launch", "configurationDone"];

        for (i, cmd) in commands.iter().enumerate() {
            let request = Message::Request(Request {
                seq: i as i32 + 1,
                command: cmd.to_string(),
                arguments: None,
            });
            mock_transport.write_message(&request).await.unwrap();
        }
    }
}
