use crate::{Error, Result};
use super::transport::DapTransport;
use super::types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, oneshot};
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

type ResponseSender = oneshot::Sender<Response>;

/// DAP Client
pub struct DapClient {
    transport: Arc<RwLock<DapTransport>>,
    seq_counter: Arc<AtomicI32>,
    pending_requests: Arc<RwLock<HashMap<i32, ResponseSender>>>,
    event_tx: mpsc::UnboundedSender<Event>,
    _child: Child,
}

impl DapClient {
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        info!("Spawning DAP client: {} {:?}", command, args);

        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Process(format!("Failed to spawn debug adapter: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| Error::Process("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| Error::Process("Failed to get stdout".to_string()))?;

        let transport = Arc::new(RwLock::new(DapTransport::new(stdin, stdout)));
        let seq_counter = Arc::new(AtomicI32::new(1));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let client = Self {
            transport: transport.clone(),
            seq_counter: seq_counter.clone(),
            pending_requests: pending_requests.clone(),
            event_tx,
            _child: child,
        };

        // Spawn message handler
        tokio::spawn(Self::message_handler(
            transport.clone(),
            pending_requests.clone(),
            event_rx,
        ));

        Ok(client)
    }

    async fn message_handler(
        transport: Arc<RwLock<DapTransport>>,
        pending_requests: Arc<RwLock<HashMap<i32, ResponseSender>>>,
        mut _event_rx: mpsc::UnboundedReceiver<Event>,
    ) {
        loop {
            let msg = {
                let mut transport = transport.write().await;
                match transport.read_message().await {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to read DAP message: {}", e);
                        break;
                    }
                }
            };

            match msg {
                Message::Response(resp) => {
                    debug!("Received response for seq {}", resp.request_seq);
                    let mut pending = pending_requests.write().await;
                    if let Some(sender) = pending.remove(&resp.request_seq) {
                        if sender.send(resp).is_err() {
                            warn!("Failed to send response to waiting request");
                        }
                    } else {
                        warn!("Received response for unknown request: {}", resp.request_seq);
                    }
                }
                Message::Event(event) => {
                    debug!("Received event: {}", event.event);
                    // Events are currently logged but not forwarded
                    // In a full implementation, we'd forward to event_tx
                }
                Message::Request(_) => {
                    warn!("Received request from debug adapter (reverse requests not implemented)");
                }
            }
        }
    }

    pub async fn send_request(&self, command: &str, arguments: Option<Value>) -> Result<Response> {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        
        let request = Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        let (tx, rx) = oneshot::channel();
        
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(seq, tx);
        }

        {
            let mut transport = self.transport.write().await;
            transport.write_message(&Message::Request(request)).await?;
        }

        rx.await
            .map_err(|_| Error::Dap("Request cancelled or connection closed".to_string()))
    }

    pub async fn initialize(&self, adapter_id: &str) -> Result<Capabilities> {
        let args = InitializeRequestArguments {
            client_id: Some("debugger_mcp".to_string()),
            client_name: Some("debugger_mcp".to_string()),
            adapter_id: adapter_id.to_string(),
            locale: Some("en-US".to_string()),
            lines_start_at_1: Some(true),
            columns_start_at_1: Some(true),
            path_format: Some("path".to_string()),
        };

        let response = self.send_request("initialize", Some(serde_json::to_value(args)?)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("Initialize failed: {:?}", response.message)));
        }

        let caps: Capabilities = response.body
            .ok_or_else(|| Error::Dap("No capabilities in initialize response".to_string()))
            .and_then(|v| serde_json::from_value(v).map_err(|e| Error::Dap(format!("Failed to parse capabilities: {}", e))))?;

        Ok(caps)
    }

    pub async fn launch(&self, args: Value) -> Result<()> {
        let response = self.send_request("launch", Some(args)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("Launch failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn configuration_done(&self) -> Result<()> {
        let response = self.send_request("configurationDone", None).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("ConfigurationDone failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn set_breakpoints(&self, source: Source, breakpoints: Vec<SourceBreakpoint>) -> Result<Vec<Breakpoint>> {
        let args = SetBreakpointsArguments {
            source,
            breakpoints: Some(breakpoints),
            source_modified: Some(false),
        };

        let response = self.send_request("setBreakpoints", Some(serde_json::to_value(args)?)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("SetBreakpoints failed: {:?}", response.message)));
        }

        #[derive(serde::Deserialize)]
        struct SetBreakpointsResponse {
            breakpoints: Vec<Breakpoint>,
        }

        let body: SetBreakpointsResponse = response.body
            .ok_or_else(|| Error::Dap("No breakpoints in response".to_string()))
            .and_then(|v| serde_json::from_value(v).map_err(|e| Error::Dap(format!("Failed to parse breakpoints: {}", e))))?;

        Ok(body.breakpoints)
    }

    pub async fn continue_execution(&self, thread_id: i32) -> Result<()> {
        let args = ContinueArguments { thread_id };
        
        let response = self.send_request("continue", Some(serde_json::to_value(args)?)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("Continue failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn stack_trace(&self, thread_id: i32) -> Result<Vec<StackFrame>> {
        let args = StackTraceArguments {
            thread_id,
            start_frame: None,
            levels: None,
        };

        let response = self.send_request("stackTrace", Some(serde_json::to_value(args)?)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("StackTrace failed: {:?}", response.message)));
        }

        #[derive(serde::Deserialize)]
        struct StackTraceResponse {
            #[serde(rename = "stackFrames")]
            stack_frames: Vec<StackFrame>,
        }

        let body: StackTraceResponse = response.body
            .ok_or_else(|| Error::Dap("No stack frames in response".to_string()))
            .and_then(|v| serde_json::from_value(v).map_err(|e| Error::Dap(format!("Failed to parse stack frames: {}", e))))?;

        Ok(body.stack_frames)
    }

    pub async fn evaluate(&self, expression: &str, frame_id: Option<i32>) -> Result<String> {
        let args = EvaluateArguments {
            expression: expression.to_string(),
            frame_id,
            context: Some("repl".to_string()),
        };

        let response = self.send_request("evaluate", Some(serde_json::to_value(args)?)).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("Evaluate failed: {:?}", response.message)));
        }

        #[derive(serde::Deserialize)]
        struct EvaluateResponse {
            result: String,
        }

        let body: EvaluateResponse = response.body
            .ok_or_else(|| Error::Dap("No result in evaluate response".to_string()))
            .and_then(|v| serde_json::from_value(v).map_err(|e| Error::Dap(format!("Failed to parse evaluate result: {}", e))))?;

        Ok(body.result)
    }

    pub async fn disconnect(&self) -> Result<()> {
        let response = self.send_request("disconnect", None).await?;
        
        if !response.success {
            warn!("Disconnect failed: {:?}", response.message);
        }

        Ok(())
    }
}
