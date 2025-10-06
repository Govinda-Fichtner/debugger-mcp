use crate::{Error, Result};
use super::types::Message;
use super::transport_trait::DapTransportTrait;
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tracing::{debug, trace};

/// DAP Transport using STDIO
pub struct DapTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl DapTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    pub async fn read_message(&mut self) -> Result<Message> {
        // Read Content-Length header
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).await?;

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

        trace!("DAP: Reading message with Content-Length: {}", content_length);

        // Read content
        let mut buffer = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.stdout, &mut buffer).await?;

        let content = String::from_utf8(buffer)
            .map_err(|e| Error::Dap(format!("Invalid UTF-8: {}", e)))?;

        debug!("DAP received: {}", content);

        let msg: Message = serde_json::from_str(&content)
            .map_err(|e| Error::Dap(format!("Failed to parse DAP message: {}", e)))?;
        
        Ok(msg)
    }

    pub async fn write_message(&mut self, msg: &Message) -> Result<()> {
        let content = serde_json::to_string(msg)
            .map_err(|e| Error::Dap(format!("Failed to serialize DAP message: {}", e)))?;
        
        debug!("DAP sending: {}", content);

        let headers = format!("Content-Length: {}\r\n\r\n", content.len());

        self.stdin.write_all(headers.as_bytes()).await?;
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.flush().await?;

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
    use super::*;
    use super::super::types::{Request, Response, Event};
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

        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| {
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
