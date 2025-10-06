use crate::{Error, Result};
use crate::adapters::python::PythonAdapter;
use crate::dap::client::DapClient;
use super::session::DebugSession;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session Manager - manages multiple debug sessions
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<DebugSession>>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        language: &str,
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
        stop_on_entry: bool,
    ) -> Result<String> {
        let (command, adapter_args, adapter_id, launch_args) = match language {
            "python" => {
                let cmd = PythonAdapter::command();
                let adapter_args = PythonAdapter::args();
                let adapter_id = PythonAdapter::adapter_id();
                let launch_args = PythonAdapter::launch_args_with_options(
                    &program,
                    &args,
                    cwd.as_deref(),
                    stop_on_entry,
                );
                (cmd, adapter_args, adapter_id, launch_args)
            }
            _ => return Err(Error::AdapterNotFound(language.to_string())),
        };

        // Spawn DAP client
        let client = DapClient::spawn(&command, &adapter_args).await?;

        // Create session
        let session = DebugSession::new(language.to_string(), program, client).await?;
        let session_id = session.id.clone();

        // Store session immediately
        let session_arc = Arc::new(session);
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session_arc.clone());
        }

        // Initialize and launch in the background
        tokio::spawn(session_arc.initialize_and_launch_async(
            adapter_id.to_string(),
            launch_args,
        ));

        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Arc<DebugSession>> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
    }

    pub async fn get_session_state(&self, session_id: &str) -> Result<crate::debug::state::DebugState> {
        let session = self.get_session(session_id).await?;
        Ok(session.get_state().await)
    }

    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        // Disconnect the session first
        if let Ok(session) = self.get_session(session_id).await {
            let _ = session.disconnect().await;
        }

        let mut sessions = self.sessions.write().await;
        sessions
            .remove(session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_new() {
        let manager = SessionManager::new();
        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let manager = SessionManager::new();
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let manager = SessionManager::new();
        let result = manager.get_session("nonexistent").await;
        assert!(result.is_err());

        match result {
            Err(Error::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_remove_session_not_found() {
        let manager = SessionManager::new();
        let result = manager.remove_session("nonexistent").await;
        assert!(result.is_err());

        match result {
            Err(Error::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_create_session_unknown_language() {
        let manager = SessionManager::new();
        let result = manager
            .create_session("ruby", "test.rb".to_string(), vec![], None, false)
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::AdapterNotFound(lang)) => {
                assert_eq!(lang, "ruby");
            }
            _ => panic!("Expected AdapterNotFound error"),
        }
    }
}
