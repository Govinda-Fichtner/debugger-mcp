use crate::{Result, Error};
use crate::dap::client::DapClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Child session in a multi-session debugging architecture
///
/// Used by adapters like vscode-js-debug that spawn child sessions for actual debugging.
/// The parent session coordinates, while child sessions handle breakpoints, stepping, etc.
#[derive(Clone)]
pub struct ChildSession {
    /// Unique identifier for this child session
    pub id: String,
    /// DAP client connected to the child debugger
    pub client: Arc<RwLock<DapClient>>,
    /// TCP port the child session is listening on
    pub port: u16,
    /// Type of child session (e.g., "pwa-node", "chrome", "electron")
    pub session_type: String,
}

/// Manager for parent-child session relationships in multi-session debugging
///
/// Tracks child sessions spawned from a parent session and routes operations
/// to the appropriate child. Used primarily for Node.js debugging with vscode-js-debug.
///
/// # Architecture
///
/// ```text
/// Parent Session (dapDebugServer.js)
///     ├─> Child Session 1 (pwa-node) ← Active
///     ├─> Child Session 2 (chrome)
///     └─> Child Session 3 (electron)
/// ```
///
/// # Usage
///
/// ```rust
/// let manager = MultiSessionManager::new("parent-session-id".to_string());
///
/// // Add child session
/// manager.add_child(child_session).await;
///
/// // Get active child for debugging operations
/// if let Some(client) = manager.get_active_child().await {
///     client.read().await.set_breakpoints(...).await?;
/// }
/// ```
#[derive(Clone)]
pub struct MultiSessionManager {
    /// ID of the parent session
    parent_session_id: String,
    /// Map of child session ID to child session
    children: Arc<RwLock<HashMap<String, ChildSession>>>,
    /// Currently active child session ID (operations routed here)
    active_child: Arc<RwLock<Option<String>>>,
}

impl MultiSessionManager {
    /// Create a new multi-session manager for a parent session
    pub fn new(parent_session_id: String) -> Self {
        info!("🔄 Creating MultiSessionManager for parent: {}", parent_session_id);
        Self {
            parent_session_id,
            children: Arc::new(RwLock::new(HashMap::new())),
            active_child: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a child session to the manager
    ///
    /// If this is the first child, it becomes the active child automatically.
    /// Future enhancement: Allow explicit active child selection.
    pub async fn add_child(&self, child: ChildSession) {
        let child_id = child.id.clone();
        let port = child.port;
        let session_type = child.session_type.clone();

        info!("➕ Adding child session '{}' (type: {}, port: {})", child_id, session_type, port);

        self.children.write().await.insert(child_id.clone(), child);

        // Set as active if first child
        let mut active = self.active_child.write().await;
        if active.is_none() {
            info!("   ✅ Set as active child (first child)");
            *active = Some(child_id);
        } else {
            info!("   Child added but not active (active: {:?})", active);
        }
    }

    /// Remove a child session (e.g., when it terminates)
    pub async fn remove_child(&self, child_id: &str) -> Result<()> {
        info!("➖ Removing child session '{}'", child_id);

        let mut children = self.children.write().await;
        if children.remove(child_id).is_none() {
            warn!("   Child session '{}' not found", child_id);
            return Err(Error::SessionNotFound(child_id.to_string()));
        }

        // If this was the active child, clear active or pick another
        let mut active = self.active_child.write().await;
        if active.as_ref() == Some(&child_id.to_string()) {
            info!("   Active child removed");
            *active = children.keys().next().cloned();
            if let Some(new_active) = &*active {
                info!("   New active child: {}", new_active);
            } else {
                info!("   No active child (no children remaining)");
            }
        }

        Ok(())
    }

    /// Get the active child session's DAP client
    ///
    /// Returns None if no child sessions exist yet.
    /// Operations should fall back to parent client if None.
    pub async fn get_active_child(&self) -> Option<Arc<RwLock<DapClient>>> {
        let active_id = self.active_child.read().await;
        if let Some(id) = active_id.as_ref() {
            let children = self.children.read().await;
            children.get(id).map(|child| child.client.clone())
        } else {
            None
        }
    }

    /// Get the active child session ID
    pub async fn get_active_child_id(&self) -> Option<String> {
        self.active_child.read().await.clone()
    }

    /// Set the active child session
    ///
    /// Future enhancement: Allow switching between multiple child sessions
    pub async fn set_active_child(&self, child_id: String) -> Result<()> {
        let children = self.children.read().await;
        if !children.contains_key(&child_id) {
            return Err(Error::SessionNotFound(format!(
                "Child session '{}' not found",
                child_id
            )));
        }
        drop(children);

        info!("🎯 Setting active child to: {}", child_id);
        *self.active_child.write().await = Some(child_id);
        Ok(())
    }

    /// Get all child session IDs
    pub async fn get_children(&self) -> Vec<String> {
        self.children.read().await.keys().cloned().collect()
    }

    /// Get a specific child session by ID
    pub async fn get_child(&self, child_id: &str) -> Option<ChildSession> {
        self.children.read().await.get(child_id).cloned()
    }

    /// Get the number of child sessions
    pub async fn child_count(&self) -> usize {
        self.children.read().await.len()
    }

    /// Get the parent session ID
    pub fn parent_id(&self) -> &str {
        &self.parent_session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dap::transport_trait::DapTransportTrait;
    use crate::dap::types::*;
    use mockall::mock;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<Message>;
            async fn write_message(&mut self, msg: &Message) -> Result<()>;
        }
    }

    fn create_empty_mock() -> MockTestTransport {
        let mut mock = MockTestTransport::new();
        mock.expect_read_message().returning(|| Err(Error::Dap("Connection closed".to_string())));
        mock
    }

    async fn create_mock_child_session(id: &str, port: u16) -> ChildSession {
        let mock_transport = create_empty_mock();
        let client = DapClient::new_with_transport(Box::new(mock_transport), None).await.unwrap();

        ChildSession {
            id: id.to_string(),
            client: Arc::new(RwLock::new(client)),
            port,
            session_type: "pwa-node".to_string(),
        }
    }

    #[tokio::test]
    async fn test_multi_session_manager_new() {
        let manager = MultiSessionManager::new("parent-123".to_string());
        assert_eq!(manager.parent_id(), "parent-123");
        assert_eq!(manager.child_count().await, 0);
        assert!(manager.get_active_child().await.is_none());
    }

    #[tokio::test]
    async fn test_add_first_child_becomes_active() {
        let manager = MultiSessionManager::new("parent".to_string());
        let child = create_mock_child_session("child-1", 9000).await;

        manager.add_child(child).await;

        assert_eq!(manager.child_count().await, 1);
        assert_eq!(manager.get_active_child_id().await, Some("child-1".to_string()));
        assert!(manager.get_active_child().await.is_some());
    }

    #[tokio::test]
    async fn test_add_multiple_children() {
        let manager = MultiSessionManager::new("parent".to_string());

        let child1 = create_mock_child_session("child-1", 9000).await;
        let child2 = create_mock_child_session("child-2", 9001).await;

        manager.add_child(child1).await;
        manager.add_child(child2).await;

        assert_eq!(manager.child_count().await, 2);
        // First child should still be active
        assert_eq!(manager.get_active_child_id().await, Some("child-1".to_string()));

        let children = manager.get_children().await;
        assert!(children.contains(&"child-1".to_string()));
        assert!(children.contains(&"child-2".to_string()));
    }

    #[tokio::test]
    async fn test_set_active_child() {
        let manager = MultiSessionManager::new("parent".to_string());

        let child1 = create_mock_child_session("child-1", 9000).await;
        let child2 = create_mock_child_session("child-2", 9001).await;

        manager.add_child(child1).await;
        manager.add_child(child2).await;

        // Switch to child-2
        manager.set_active_child("child-2".to_string()).await.unwrap();
        assert_eq!(manager.get_active_child_id().await, Some("child-2".to_string()));
    }

    #[tokio::test]
    async fn test_set_active_child_not_found() {
        let manager = MultiSessionManager::new("parent".to_string());
        let result = manager.set_active_child("nonexistent".to_string()).await;

        assert!(result.is_err());
        match result {
            Err(Error::SessionNotFound(msg)) => {
                assert!(msg.contains("nonexistent"));
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_remove_child() {
        let manager = MultiSessionManager::new("parent".to_string());
        let child = create_mock_child_session("child-1", 9000).await;

        manager.add_child(child).await;
        assert_eq!(manager.child_count().await, 1);

        manager.remove_child("child-1").await.unwrap();
        assert_eq!(manager.child_count().await, 0);
        assert!(manager.get_active_child().await.is_none());
    }

    #[tokio::test]
    async fn test_remove_active_child_switches_to_next() {
        let manager = MultiSessionManager::new("parent".to_string());

        let child1 = create_mock_child_session("child-1", 9000).await;
        let child2 = create_mock_child_session("child-2", 9001).await;

        manager.add_child(child1).await;
        manager.add_child(child2).await;

        // Remove active child (child-1)
        manager.remove_child("child-1").await.unwrap();

        // child-2 should become active
        assert_eq!(manager.child_count().await, 1);
        assert_eq!(manager.get_active_child_id().await, Some("child-2".to_string()));
    }

    #[tokio::test]
    async fn test_get_child() {
        let manager = MultiSessionManager::new("parent".to_string());
        let child = create_mock_child_session("child-1", 9000).await;

        manager.add_child(child).await;

        let retrieved = manager.get_child("child-1").await;
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "child-1");
        assert_eq!(retrieved.port, 9000);
        assert_eq!(retrieved.session_type, "pwa-node");
    }

    #[tokio::test]
    async fn test_get_child_not_found() {
        let manager = MultiSessionManager::new("parent".to_string());
        assert!(manager.get_child("nonexistent").await.is_none());
    }
}
