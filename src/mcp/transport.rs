use crate::{Error, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, trace};

use super::protocol::JsonRpcMessage;

pub struct StdioTransport {
    stdin: BufReader<tokio::io::Stdin>,
    stdout: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(tokio::io::stdin()),
            stdout: tokio::io::stdout(),
        }
    }

    pub async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        // Read Content-Length header
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            self.stdin.read_line(&mut line).await?;

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
            .ok_or_else(|| Error::InvalidRequest("Missing Content-Length header".to_string()))?;

        trace!("Reading message with Content-Length: {}", content_length);

        // Read content
        let mut buffer = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.stdin, &mut buffer).await?;

        let content = String::from_utf8(buffer)
            .map_err(|e| Error::InvalidRequest(format!("Invalid UTF-8: {}", e)))?;

        debug!("Received message: {}", content);

        let msg: JsonRpcMessage = serde_json::from_str(&content)?;
        Ok(msg)
    }

    pub async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()> {
        let content = serde_json::to_string(msg)?;
        debug!("Sending message: {}", content);

        let headers = format!("Content-Length: {}\r\n\r\n", content.len());

        self.stdout.write_all(headers.as_bytes()).await?;
        self.stdout.write_all(content.as_bytes()).await?;
        self.stdout.flush().await?;

        Ok(())
    }
}
