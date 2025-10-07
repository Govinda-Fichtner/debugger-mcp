use crate::{Error, Result};
use crate::adapters::python::PythonAdapter;
use crate::adapters::ruby::RubyAdapter;
use crate::adapters::nodejs::NodeJsAdapter;
use crate::adapters::rust::RustAdapter;
use crate::adapters::logging::DebugAdapterLogger;
use crate::dap::client::DapClient;
use super::session::DebugSession;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

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
        // Type alias for STDIO adapter tuple: (command, args, adapter_id, launch_args, adapter_for_logging)
        type StdioAdapterTuple<'a> = (String, Vec<String>, &'a str, serde_json::Value, Box<dyn DebugAdapterLogger + 'a>);

        let (command, adapter_args, adapter_id, launch_args, adapter): StdioAdapterTuple = match language {
            "python" => {
                // Create adapter instance for logging
                let adapter = PythonAdapter;

                // Log adapter selection
                adapter.log_selection();

                let cmd = PythonAdapter::command();
                let adapter_args = PythonAdapter::args();
                let adapter_id = PythonAdapter::adapter_id();
                let launch_args = PythonAdapter::launch_args_with_options(
                    &program,
                    &args,
                    cwd.as_deref(),
                    stop_on_entry,
                );

                // Log transport initialization
                adapter.log_transport_init();

                (cmd, adapter_args, adapter_id, launch_args, Box::new(adapter))
            }
            "ruby" => {
                // Create adapter instance for logging
                let adapter = RubyAdapter;

                // Log adapter selection
                adapter.log_selection();

                // Log transport initialization
                adapter.log_transport_init();

                // Ruby uses socket-based communication, not stdio
                // Spawn rdbg and connect to socket
                adapter.log_spawn_attempt();
                let ruby_session = RubyAdapter::spawn(&program, &args, stop_on_entry)
                    .await
                    .map_err(|e| {
                        adapter.log_spawn_error(&e);
                        e
                    })?;

                // Log successful connection with Ruby-specific details
                ruby_session.log_connection_success_with_port();

                let adapter_id = RubyAdapter::adapter_id();
                let launch_args = RubyAdapter::launch_args_with_options(
                    &program,
                    &args,
                    cwd.as_deref(),
                    stop_on_entry,
                );

                // Create DAP client from socket
                let client = DapClient::from_socket(ruby_session.socket)
                    .await
                    .map_err(|e| {
                        adapter.log_connection_error(&e);
                        e
                    })?;

                // Create session
                let session = DebugSession::new(language.to_string(), program.clone(), client).await?;
                let session_id = session.id.clone();

                // Store session immediately
                let session_arc = Arc::new(session);
                {
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(session_id.clone(), session_arc.clone());
                }

                // Log workaround application (Ruby requires entry breakpoint workaround)
                adapter.log_workaround_applied();

                // Initialize and launch in the background
                tokio::spawn(session_arc.initialize_and_launch_async(
                    adapter_id.to_string(),
                    launch_args,
                ));

                return Ok(session_id);
            }
            "nodejs" => {
                // Create adapter instance for logging
                let adapter = NodeJsAdapter;

                // Log adapter selection
                adapter.log_selection();

                // Log transport initialization
                adapter.log_transport_init();

                // Node.js uses socket-based communication with vscode-js-debug DAP server
                // Spawn vscode-js-debug and connect to socket
                adapter.log_spawn_attempt();
                let nodejs_session = NodeJsAdapter::spawn_dap_server()
                    .await
                    .map_err(|e| {
                        adapter.log_spawn_error(&e);
                        e
                    })?;

                // Log successful connection with Node.js-specific details
                nodejs_session.log_connection_success_with_details();

                let adapter_id = NodeJsAdapter::adapter_id();
                let launch_args = NodeJsAdapter::launch_config(
                    &program,
                    &args,
                    cwd.as_deref(),
                    stop_on_entry,
                );

                // Create DAP client from socket (parent session)
                let parent_client = DapClient::from_socket(nodejs_session.socket)
                    .await
                    .map_err(|e| {
                        adapter.log_connection_error(&e);
                        e
                    })?;

                info!("🔄 [NODEJS] Creating multi-session manager for parent session");

                // Create session with multi-session mode
                use super::session::SessionMode;
                use super::multi_session::MultiSessionManager;

                let session_id = uuid::Uuid::new_v4().to_string();
                let multi_session_manager = MultiSessionManager::new(session_id.clone());

                let session_mode = SessionMode::MultiSession {
                    parent_client: Arc::new(RwLock::new(parent_client)),
                    multi_session_manager: multi_session_manager.clone(),
                    vscode_js_debug_port: nodejs_session.port,
                };

                let session = DebugSession::new_with_mode(
                    language.to_string(),
                    program.clone(),
                    session_mode,
                )
                .await?;

                // Store session immediately
                let session_arc = Arc::new(session);
                {
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(session_id.clone(), session_arc.clone());
                }

                // Register child session spawn callback on parent client
                info!("🔄 [NODEJS] Registering child session spawn callback");
                let session_clone = session_arc.clone();
                if let SessionMode::MultiSession { parent_client, .. } = &session_arc.session_mode {
                    let parent = parent_client.read().await;
                    parent
                        .on_child_session_spawn(move |target_id| {
                            let session = session_clone.clone();
                            Box::pin(async move {
                                info!("🎯 [NODEJS] Child session spawn callback invoked for target_id: {}", target_id);
                                if let Err(e) = session.spawn_child_session(target_id).await {
                                    error!("❌ [NODEJS] Failed to spawn child session: {}", e);
                                } else {
                                    info!("✅ [NODEJS] Child session spawned successfully");
                                }
                            })
                        })
                        .await;
                }

                // Log workaround application (Node.js uses multi-session for stopOnEntry)
                adapter.log_workaround_applied();

                // Initialize and launch in the background
                // This will trigger the parent session, which will send startDebugging reverse request
                tokio::spawn(session_arc.initialize_and_launch_async(
                    adapter_id.to_string(),
                    launch_args,
                ));

                return Ok(session_id);
            }
            "rust" => {
                // Create adapter instance for logging
                let adapter = RustAdapter;

                // Log adapter selection
                adapter.log_selection();

                info!("🔨 [RUST] Compiling Rust source before debugging");

                // Step 1: Compile the Rust source (auto-detects single-file vs Cargo project)
                RustAdapter::log_compilation_start(&program, false);  // false = debug build
                let binary_path = RustAdapter::compile(&program, false)
                    .await
                    .map_err(|e| {
                        RustAdapter::log_compilation_error(&e);
                        e
                    })?;

                RustAdapter::log_compilation_success(&binary_path);

                // Step 2: Prepare CodeLLDB adapter
                let cmd = RustAdapter::command();
                let adapter_args = RustAdapter::args();
                let adapter_id = RustAdapter::adapter_id();
                let launch_args = RustAdapter::launch_args(
                    &binary_path,  // Use compiled binary path, not source
                    &args,
                    cwd.as_deref(),
                    stop_on_entry,
                );

                // Log transport initialization
                adapter.log_transport_init();

                (cmd, adapter_args, adapter_id, launch_args, Box::new(adapter))
            }
            _ => return Err(Error::AdapterNotFound(language.to_string())),
        };

        // Spawn DAP client (Python/Rust path - uses STDIO transport)
        // Adapter instance is passed from match arm above for language-specific logging
        adapter.log_spawn_attempt();
        let client = DapClient::spawn(&command, &adapter_args)
            .await
            .map_err(|e| {
                adapter.log_spawn_error(&e);
                e
            })?;

        // Log successful connection
        adapter.log_connection_success();

        // Create session
        let session = DebugSession::new(language.to_string(), program, client).await?;
        let session_id = session.id.clone();

        // Store session immediately
        let session_arc = Arc::new(session);
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session_arc.clone());
        }

        // Log workaround if needed (Python/Rust don't require workarounds)
        adapter.log_workaround_applied();

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
        // Use a truly unsupported language (ruby is now supported!)
        let result = manager
            .create_session("javascript", "test.js".to_string(), vec![], None, false)
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::AdapterNotFound(lang)) => {
                assert_eq!(lang, "javascript");
            }
            _ => panic!("Expected AdapterNotFound error"),
        }
    }
}
