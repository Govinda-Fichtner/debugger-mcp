/// Multi-connection TCP listener for vscode-js-debug
///
/// vscode-js-debug uses a multi-session architecture where:
/// 1. Parent session connects first to the DAP server port
/// 2. When parent sends `launch`, vscode-js-debug spawns child Node.js process
/// 3. Child connects BACK to the SAME DAP server port with __pendingTargetId
/// 4. We match the child connection to the pending target ID
///
/// This module manages accepting multiple connections on the same port.

use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

/// Represents a pending child session waiting for a connection
#[derive(Debug, Clone)]
pub struct PendingChild {
    pub target_id: String,
    pub created_at: std::time::Instant,
}

/// Multi-connection listener state
pub struct MultiConnectionListener {
    /// The TCP listener accepting connections
    listener: Arc<Mutex<TcpListener>>,
    /// Port the listener is bound to
    pub port: u16,
    /// Map of pending child sessions (target_id -> PendingChild)
    pending_children: Arc<RwLock<HashMap<String, PendingChild>>>,
    /// Channel to send accepted child connections
    child_tx: mpsc::UnboundedSender<(String, TcpStream)>,
    /// Channel to receive accepted child connections
    child_rx: Arc<Mutex<mpsc::UnboundedReceiver<(String, TcpStream)>>>,
}

impl MultiConnectionListener {
    /// Create a new listener and accept the first (parent) connection
    ///
    /// This binds to a free port, accepts the parent connection, and keeps
    /// the listener open for child connections.
    ///
    /// Returns: (parent_socket, listener)
    pub async fn create_and_accept_parent() -> Result<(TcpStream, Self)> {
        // Bind to a free port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| Error::Process(format!("Failed to bind TCP listener: {}", e)))?;

        let port = listener
            .local_addr()
            .map_err(|e| Error::Process(format!("Failed to get listener address: {}", e)))?
            .port();

        info!("📡 Multi-connection listener bound to port {}", port);

        // Wait for parent connection with timeout
        let parent_socket = timeout(Duration::from_secs(5), listener.accept())
            .await
            .map_err(|_| Error::Process(format!("Timeout waiting for parent connection on port {}", port)))?
            .map_err(|e| Error::Process(format!("Failed to accept parent connection: {}", e)))?
            .0;

        info!("✅ Parent connection accepted on port {}", port);

        // Create channel for child connections
        let (child_tx, child_rx) = mpsc::unbounded_channel();

        let multi_listener = Self {
            listener: Arc::new(Mutex::new(listener)),
            port,
            pending_children: Arc::new(RwLock::new(HashMap::new())),
            child_tx,
            child_rx: Arc::new(Mutex::new(child_rx)),
        };

        // Start accepting child connections in background
        multi_listener.start_accept_loop();

        Ok((parent_socket, multi_listener))
    }

    /// Register a pending child session that we expect to connect
    ///
    /// When parent sends `startDebugging` with `__pendingTargetId`, we register
    /// it here so we can match incoming connections.
    pub async fn register_pending_child(&self, target_id: String) {
        info!("📝 Registering pending child: {}", target_id);
        let mut pending = self.pending_children.write().await;
        pending.insert(
            target_id.clone(),
            PendingChild {
                target_id,
                created_at: std::time::Instant::now(),
            },
        );
    }

    /// Wait for a child connection with the given target ID
    ///
    /// This blocks until a connection arrives with matching __pendingTargetId
    /// or timeout occurs.
    pub async fn wait_for_child_connection(
        &self,
        target_id: &str,
        timeout_duration: Duration,
    ) -> Result<TcpStream> {
        info!("⏳ Waiting for child connection with target_id: {}", target_id);

        let result = timeout(timeout_duration, async {
            let mut rx = self.child_rx.lock().await;
            loop {
                if let Some((id, socket)) = rx.recv().await {
                    if id == target_id {
                        info!("✅ Child connection matched: {}", target_id);
                        return Ok(socket);
                    } else {
                        warn!("⚠️  Received child connection with wrong target_id: {} (expected: {})", id, target_id);
                    }
                } else {
                    return Err(Error::Process(
                        "Child connection channel closed".to_string(),
                    ));
                }
            }
        })
        .await;

        match result {
            Ok(socket_result) => socket_result,
            Err(_) => {
                error!("❌ Timeout waiting for child connection: {}", target_id);
                Err(Error::Process(format!(
                    "Timeout waiting for child connection with target_id: {}",
                    target_id
                )))
            }
        }
    }

    /// Start the background task that accepts child connections
    fn start_accept_loop(&self) {
        let listener = self.listener.clone();
        let pending_children = self.pending_children.clone();
        let child_tx = self.child_tx.clone();
        let port = self.port;

        tokio::spawn(async move {
            info!("🔄 Starting child connection accept loop on port {}", port);

            loop {
                let listener_guard = listener.lock().await;
                match listener_guard.accept().await {
                    Ok((socket, addr)) => {
                        drop(listener_guard); // Release lock before processing

                        info!("📥 New connection received from {} on port {}", addr, port);

                        // We don't have target_id yet - it will come in the DAP initialize message
                        // For now, we need to peek at the first DAP message to extract __pendingTargetId
                        // TODO: Implement DAP message peeking

                        // Temporary workaround: Try to match with any pending child
                        let pending = pending_children.read().await;
                        if let Some((target_id, _)) = pending.iter().next() {
                            let target_id = target_id.clone();
                            drop(pending);

                            info!("🎯 Matching connection to pending child: {}", target_id);
                            if child_tx.send((target_id.clone(), socket)).is_err() {
                                error!("❌ Failed to send child connection to channel");
                            }

                            // Remove from pending list
                            pending_children.write().await.remove(&target_id);
                        } else {
                            warn!("⚠️  Received connection but no pending children registered");
                        }
                    }
                    Err(e) => {
                        error!("❌ Error accepting connection on port {}: {}", port, e);
                        drop(listener_guard);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
    }

    /// Get the port this listener is bound to
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Clean up expired pending children (older than 30 seconds)
    pub async fn cleanup_expired_pending(&self) {
        let mut pending = self.pending_children.write().await;
        let now = std::time::Instant::now();
        pending.retain(|id, child| {
            let age = now.duration_since(child.created_at);
            if age.as_secs() > 30 {
                warn!("🧹 Cleaning up expired pending child: {} (age: {}s)", id, age.as_secs());
                false
            } else {
                true
            }
        });
    }
}
