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
    ) -> Result<String> {
        let (command, adapter_args, adapter_id, launch_args) = match language {
            "python" => {
                let cmd = PythonAdapter::command();
                let adapter_args = PythonAdapter::args();
                let adapter_id = PythonAdapter::adapter_id();
                let launch_args = PythonAdapter::launch_args(
                    &program,
                    &args,
                    cwd.as_deref(),
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

        // Initialize DAP
        session.initialize(adapter_id).await?;

        // Launch program
        session.launch(launch_args).await?;

        // Store session
        let session_arc = Arc::new(session);
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_arc);

        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Arc<DebugSession>> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
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
