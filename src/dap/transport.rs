use crate::{Error, Result};
use super::types::Message;
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
