/// Helper functions for socket-based DAP adapters (e.g., Ruby/rdbg)
use crate::{Error, Result};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Find an available TCP port on localhost
pub fn find_free_port() -> Result<u16> {
    // Use port 0 to let OS assign a free port
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| Error::Process(format!("Failed to bind to port: {}", e)))?;

    let port = listener.local_addr()
        .map_err(|e| Error::Process(format!("Failed to get local address: {}", e)))?
        .port();

    debug!("Found free port: {}", port);
    Ok(port)
}

/// Connect to TCP socket with retry and timeout
///
/// Retries connecting to the specified port for up to `timeout` duration,
/// with 100ms between attempts.
pub async fn connect_with_retry(port: u16, timeout: Duration) -> Result<TcpStream> {
    let start = std::time::Instant::now();
    let retry_interval = Duration::from_millis(100);

    info!("Connecting to 127.0.0.1:{} (timeout: {:?})", port, timeout);

    loop {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(stream) => {
                info!("Connected to 127.0.0.1:{} after {:?}", port, start.elapsed());
                return Ok(stream);
            }
            Err(e) => {
                if start.elapsed() >= timeout {
                    return Err(Error::Process(format!(
                        "Failed to connect to port {} after {:?}: {}",
                        port, timeout, e
                    )));
                }
                // Wait before retrying
                tokio::time::sleep(retry_interval).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn test_find_free_port() {
        let port = find_free_port().unwrap();
        assert!(port > 0); // Port should be non-zero (u16 max is 65535 anyway)
    }

    #[test]
    fn test_find_multiple_free_ports() {
        let port1 = find_free_port().unwrap();
        let port2 = find_free_port().unwrap();
        // Ports should be different (very high probability)
        assert_ne!(port1, port2);
    }

    #[tokio::test]
    async fn test_connect_with_retry_success() {
        // Start a test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Accept connection in background
        tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        // Connect should succeed
        let result = connect_with_retry(port, Duration::from_secs(2)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connect_with_retry_timeout() {
        // Try to connect to a port that's not listening
        let port = find_free_port().unwrap();

        // Should timeout after 500ms
        let result = connect_with_retry(port, Duration::from_millis(500)).await;
        assert!(result.is_err());

        // Check error message
        match result {
            Err(Error::Process(msg)) => {
                assert!(msg.contains("Failed to connect"));
                assert!(msg.contains(&port.to_string()));
            }
            _ => panic!("Expected Process error"),
        }
    }

    #[tokio::test]
    async fn test_connect_with_retry_eventual_success() {
        let port = find_free_port().unwrap();

        // Start server after a delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
            let _ = listener.accept().await;
        });

        // Should eventually connect
        let result = connect_with_retry(port, Duration::from_secs(2)).await;
        assert!(result.is_ok());
    }
}
