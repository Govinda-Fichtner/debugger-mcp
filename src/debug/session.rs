use crate::{Error, Result};
use crate::dap::client::DapClient;
use crate::dap::types::{Source, SourceBreakpoint};
use super::state::{SessionState, DebugState};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct DebugSession {
    pub id: String,
    pub language: String,
    pub program: String,
    client: Arc<RwLock<DapClient>>,
    state: Arc<RwLock<SessionState>>,
}

impl DebugSession {
    pub async fn new(
        language: String,
        program: String,
        client: DapClient,
    ) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        
        Ok(Self {
            id,
            language,
            program,
            client: Arc::new(RwLock::new(client)),
            state: Arc::new(RwLock::new(SessionState::new())),
        })
    }

    pub async fn initialize(&self, adapter_id: &str) -> Result<()> {
        {
            let mut state = self.state.write().await;
            state.set_state(DebugState::Initializing);
        }

        let client = self.client.read().await;
        client.initialize(adapter_id).await?;

        {
            let mut state = self.state.write().await;
            state.set_state(DebugState::Initialized);
        }

        Ok(())
    }

    pub async fn launch(&self, launch_args: serde_json::Value) -> Result<()> {
        {
            let mut state = self.state.write().await;
            state.set_state(DebugState::Launching);
        }

        let client = self.client.read().await;
        client.launch(launch_args).await?;

        {
            let mut state = self.state.write().await;
            state.set_state(DebugState::Running);
        }

        Ok(())
    }

    pub async fn set_breakpoint(&self, source_path: String, line: i32) -> Result<bool> {
        // Add to state
        {
            let mut state = self.state.write().await;
            state.add_breakpoint(source_path.clone(), line);
        }

        // Set via DAP
        let source = Source {
            name: None,
            path: Some(source_path.clone()),
            source_reference: None,
        };

        let breakpoints = vec![SourceBreakpoint {
            line,
            column: None,
            condition: None,
            hit_condition: None,
        }];

        let client = self.client.read().await;
        let result = client.set_breakpoints(source, breakpoints).await?;

        // Update state with results
        if let Some(bp) = result.first() {
            let mut state = self.state.write().await;
            if let Some(id) = bp.id {
                state.update_breakpoint(&source_path, line, id, bp.verified);
            }
            Ok(bp.verified)
        } else {
            Ok(false)
        }
    }

    pub async fn continue_execution(&self) -> Result<()> {
        let state = self.state.read().await;
        let thread_id = state.threads.first().copied().unwrap_or(1);
        drop(state);

        let client = self.client.read().await;
        client.continue_execution(thread_id).await?;

        let mut state = self.state.write().await;
        state.set_state(DebugState::Running);

        Ok(())
    }

    pub async fn stack_trace(&self) -> Result<Vec<crate::dap::types::StackFrame>> {
        let state = self.state.read().await;
        let thread_id = state.threads.first().copied().unwrap_or(1);
        drop(state);

        let client = self.client.read().await;
        client.stack_trace(thread_id).await
    }

    pub async fn evaluate(&self, expression: &str, frame_id: Option<i32>) -> Result<String> {
        let client = self.client.read().await;
        client.evaluate(expression, frame_id).await
    }

    pub async fn disconnect(&self) -> Result<()> {
        let client = self.client.read().await;
        client.disconnect().await?;

        let mut state = self.state.write().await;
        state.set_state(DebugState::Terminated);

        Ok(())
    }

    pub async fn get_state(&self) -> DebugState {
        let state = self.state.read().await;
        state.state.clone()
    }
}
