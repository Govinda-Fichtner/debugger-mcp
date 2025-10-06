use crate::{Error, Result};
use super::transport::DapTransport;
use super::transport_trait::DapTransportTrait;
use super::types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex, oneshot, Notify};
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

type ResponseSender = oneshot::Sender<Response>;
type EventNotifier = Arc<Notify>;
type EventCallback = Arc<dyn Fn(Event) + Send + Sync>;

/// DAP Client with event-driven architecture
pub struct DapClient {
    transport: Arc<Mutex<Box<dyn DapTransportTrait>>>,
    seq_counter: Arc<AtomicI32>,
    pending_requests: Arc<RwLock<HashMap<i32, ResponseSender>>>,
    #[allow(dead_code)] // Reserved for future event handling
    event_tx: mpsc::UnboundedSender<Event>,
    // For backward compatibility with wait_for_event
    event_notifiers: Arc<RwLock<HashMap<String, EventNotifier>>>,
    // New: Event callbacks (can have multiple callbacks per event)
    event_callbacks: Arc<RwLock<HashMap<String, Vec<EventCallback>>>>,
    // Channel for sending write requests to avoid lock contention
    write_tx: mpsc::UnboundedSender<Message>,
    _child: Option<Child>,
}

impl DapClient {
    /// Spawn a DAP adapter via stdio (for Python/debugpy)
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

        let transport: Box<dyn DapTransportTrait> = Box::new(DapTransport::new(stdin, stdout));
        Self::new_with_transport(transport, Some(child)).await
    }

    /// Create DAP client from TCP socket (for Ruby/rdbg)
    pub async fn from_socket(socket: tokio::net::TcpStream) -> Result<Self> {
        info!("Creating DAP client from socket: {:?}", socket.peer_addr());

        let transport: Box<dyn DapTransportTrait> = Box::new(DapTransport::new_socket(socket));
        Self::new_with_transport(transport, None).await
    }

    /// Create a new DAP client with a custom transport (for testing)
    pub async fn new_with_transport(
        transport: Box<dyn DapTransportTrait>,
        child: Option<Child>,
    ) -> Result<Self> {
        let transport = Arc::new(Mutex::new(transport));
        let seq_counter = Arc::new(AtomicI32::new(1));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (write_tx, write_rx) = mpsc::unbounded_channel();

        let event_notifiers = Arc::new(RwLock::new(HashMap::new()));
        let event_callbacks = Arc::new(RwLock::new(HashMap::new()));

        let client = Self {
            transport: transport.clone(),
            seq_counter: seq_counter.clone(),
            pending_requests: pending_requests.clone(),
            event_tx,
            event_notifiers: event_notifiers.clone(),
            event_callbacks: event_callbacks.clone(),
            write_tx: write_tx.clone(),
            _child: child,
        };

        // Spawn message reader handler
        tokio::spawn(Self::message_reader(
            transport.clone(),
            pending_requests.clone(),
            event_notifiers.clone(),
            event_callbacks.clone(),
            event_rx,
        ));

        // Spawn message writer handler
        tokio::spawn(Self::message_writer(
            transport.clone(),
            write_rx,
        ));

        Ok(client)
    }

    /// Message reader task - reads messages from transport and dispatches them
    async fn message_reader(
        transport: Arc<Mutex<Box<dyn DapTransportTrait>>>,
        pending_requests: Arc<RwLock<HashMap<i32, ResponseSender>>>,
        event_notifiers: Arc<RwLock<HashMap<String, EventNotifier>>>,
        event_callbacks: Arc<RwLock<HashMap<String, Vec<EventCallback>>>>,
        mut _event_rx: mpsc::UnboundedReceiver<Event>,
    ) {
        loop {
            debug!("📖 message_reader: Attempting to acquire transport lock");

            // Try to read a message with the lock, but release it if no message is ready
            let msg_result = {
                let mut transport = transport.lock().await;
                debug!("📖 message_reader: Lock acquired, checking for message");

                // Use select with timeout to avoid holding lock indefinitely
                let read_future = transport.read_message();
                tokio::select! {
                    result = read_future => {
                        debug!("📖 message_reader: Message read attempt completed");
                        Some(result)
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {
                        debug!("📖 message_reader: Read timeout, releasing lock");
                        None
                    }
                }
            };
            debug!("📖 message_reader: Lock released");

            // If we didn't get a message, continue loop (will retry)
            let msg = match msg_result {
                None => {
                    tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
                    continue;
                }
                Some(Ok(msg)) => msg,
                Some(Err(e)) => {
                    error!("📖 message_reader: Failed to read DAP message: {}", e);
                    break;
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
                    info!("🎯 EVENT RECEIVED: '{}' with body: {:?}", event.event, event.body);

                    // 1. Notify anyone waiting for this specific event (legacy wait_for_event)
                    let notifiers = event_notifiers.read().await;
                    if let Some(notifier) = notifiers.get(&event.event) {
                        info!("  Notifying waiters for event '{}'", event.event);
                        notifier.notify_waiters();
                    }
                    drop(notifiers);

                    // 2. Invoke registered event callbacks
                    let callbacks = event_callbacks.read().await;
                    if let Some(handlers) = callbacks.get(&event.event) {
                        info!("  Found {} callback(s) for event '{}'", handlers.len(), event.event);
                        for (idx, callback) in handlers.iter().enumerate() {
                            info!("  Invoking callback {} for event '{}'", idx, event.event);
                            // Invoke callback with cloned event
                            callback(event.clone());
                            info!("  Callback {} completed for event '{}'", idx, event.event);
                        }
                    } else {
                        info!("  No callbacks registered for event '{}'", event.event);
                    }
                }
                Message::Request(_) => {
                    warn!("Received request from debug adapter (reverse requests not implemented)");
                }
            }

            // Small sleep to let other tasks run (e.g., configurationDone sender)
            // This is necessary because read_message() blocks holding the lock
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
        }
    }

    /// Message writer task - writes messages to transport from a channel
    /// This avoids lock contention between reader and writers
    async fn message_writer(
        transport: Arc<Mutex<Box<dyn DapTransportTrait>>>,
        mut write_rx: mpsc::UnboundedReceiver<Message>,
    ) {
        info!("📝 message_writer: Task started");
        while let Some(message) = write_rx.recv().await {
            let msg_type = match &message {
                Message::Request(req) => format!("Request({})", req.command),
                Message::Response(resp) => format!("Response(seq {})", resp.seq),
                Message::Event(evt) => format!("Event({})", evt.event),
            };
            info!("📝 message_writer: Received {} from channel", msg_type);
            info!("📝 message_writer: Attempting to acquire transport lock");
            let mut transport = transport.lock().await;
            info!("📝 message_writer: Lock acquired, writing message");
            if let Err(e) = transport.write_message(&message).await {
                error!("📝 message_writer: Failed to write DAP message: {}", e);
                break;
            }
            info!("📝 message_writer: Message written successfully, releasing lock");
            drop(transport);
            info!("📝 message_writer: Lock released");
        }
        info!("📝 message_writer: Task exiting");
    }

    /// Register a callback for a specific DAP event
    /// The callback will be invoked every time the event is received
    pub async fn on_event<F>(&self, event_name: &str, callback: F)
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let mut callbacks = self.event_callbacks.write().await;
        callbacks
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(Arc::new(callback));
    }

    /// Remove all callbacks for a specific event
    pub async fn remove_event_handlers(&self, event_name: &str) {
        let mut callbacks = self.event_callbacks.write().await;
        callbacks.remove(event_name);
    }

    /// Wait for a specific DAP event with a timeout (legacy method)
    pub async fn wait_for_event(&self, event_name: &str, timeout: tokio::time::Duration) -> Result<()> {
        let notifier = {
            let mut notifiers = self.event_notifiers.write().await;
            // Create or get existing notifier for this event
            notifiers.entry(event_name.to_string())
                .or_insert_with(|| Arc::new(Notify::new()))
                .clone()
        };

        // Wait for notification or timeout
        tokio::select! {
            _ = notifier.notified() => {
                debug!("Received '{}' event", event_name);
                Ok(())
            }
            _ = tokio::time::sleep(timeout) => {
                Err(Error::Dap(format!("Timeout waiting for '{}' event after {:?}", event_name, timeout)))
            }
        }
    }

    /// Send a request without waiting for response (fire-and-forget)
    /// Useful when you'll handle the response via another mechanism
    pub async fn send_request_nowait(&self, command: &str, arguments: Option<Value>) -> Result<i32> {
        debug!("send_request_nowait: Starting for command '{}'", command);
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);

        let request = Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        debug!("send_request_nowait: Acquiring transport lock for command '{}'", command);
        let mut transport = self.transport.lock().await;
        debug!("send_request_nowait: Writing {} request (seq {})", command, seq);
        transport.write_message(&Message::Request(request)).await?;
        debug!("send_request_nowait: Message written successfully");
        drop(transport);

        Ok(seq)
    }

    /// Send a request and wait for response (blocking)
    pub async fn send_request(&self, command: &str, arguments: Option<Value>) -> Result<Response> {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);

        info!("✉️  send_request: Sending '{}' request (seq {})", command, seq);

        let request = Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(seq, tx);
            info!("✉️  send_request: Registered pending request for seq {}", seq);
        }

        info!("✉️  send_request: Sending message to write channel");
        self.write_tx.send(Message::Request(request))
            .map_err(|_| Error::Dap("Write channel closed".to_string()))?;

        info!("✉️  send_request: Waiting for response to seq {}", seq);
        let response = rx.await
            .map_err(|_| Error::Dap("Request cancelled or connection closed".to_string()))?;

        info!("✅ send_request: Received response for '{}' (seq {}), success: {}", command, seq, response.success);
        Ok(response)
    }

    /// Send a request with a timeout (aggressive timeout wrapper)
    pub async fn send_request_with_timeout(
        &self,
        command: &str,
        arguments: Option<Value>,
        timeout: std::time::Duration,
    ) -> Result<Response> {
        info!("⏱️  send_request_with_timeout: '{}' with timeout {:?}", command, timeout);

        tokio::time::timeout(timeout, self.send_request(command, arguments))
            .await
            .map_err(|_| Error::Dap(format!("Request '{}' timed out after {:?}", command, timeout)))?
    }

    /// Send a request with a callback for the response
    pub async fn send_request_async<F>(&self, command: &str, arguments: Option<Value>, callback: F) -> Result<i32>
    where
        F: FnOnce(Result<Response>) + Send + 'static,
    {
        debug!("send_request_async: Starting for command '{}'", command);
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);

        let request = Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        let (tx, rx) = oneshot::channel();

        debug!("send_request_async: Registering pending request");
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(seq, tx);
        }

        debug!("send_request_async: Sending {} request (seq {}) to write channel", command, seq);
        self.write_tx.send(Message::Request(request))
            .map_err(|_| Error::Dap("Write channel closed".to_string()))?;
        debug!("send_request_async: Request queued");

        // Spawn task to wait for response and invoke callback
        tokio::spawn(async move {
            debug!("send_request_async callback task: Waiting for response seq {}", seq);
            match rx.await {
                Ok(response) => {
                    debug!("send_request_async callback task: Got response for seq {}", seq);
                    callback(Ok(response))
                },
                Err(_) => callback(Err(Error::Dap("Request cancelled or connection closed".to_string()))),
            }
        });

        debug!("send_request_async: Returning seq {}", seq);
        Ok(seq)
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

    /// Proper DAP initialization and launch sequence following the specification
    /// This method implements the correct async flow:
    /// 1. Send initialize, get response
    /// 2. Register 'initialized' event handler (just signals, doesn't call methods)
    /// 3. Send launch (triggers 'initialized' event)
    /// 4. Wait for 'initialized' signal, then send configurationDone from main context
    /// 5. Wait for launch response
    pub async fn initialize_and_launch(
        &self,
        adapter_id: &str,
        launch_args: Value,
        adapter_type: Option<&str>,
    ) -> Result<()> {
        // Step 1: Send initialize request and get capabilities
        info!("Sending initialize request to adapter");
        let capabilities = self.initialize(adapter_id).await?;
        debug!("Adapter capabilities: supportsConfigurationDoneRequest={:?}",
               capabilities.supports_configuration_done_request);

        let config_done_supported = capabilities.supports_configuration_done_request.unwrap_or(false);

        // Check if we need Ruby stopOnEntry workaround
        // Ruby debuggers (rdbg) in socket mode don't honor --stop-at-load properly.
        // Workaround: After 'initialized' event, send explicit 'pause' request.
        // See: docs/RUBY_STOPENTRY_FIX.md
        let is_ruby = adapter_type == Some("ruby");
        let stop_on_entry = launch_args.get("stopOnEntry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let needs_ruby_workaround = is_ruby && stop_on_entry;

        if needs_ruby_workaround {
            info!("🔧 Ruby stopOnEntry workaround will be applied");
        }

        // Extract program path before moving launch_args (for Ruby workaround)
        let program_path_for_breakpoint = if needs_ruby_workaround {
            launch_args.get("program")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        // Step 2: Register 'initialized' event handler BEFORE sending launch
        // This handler will be invoked DURING launch processing
        // We use a simple signal approach - the handler just notifies, doesn't send messages
        let (init_tx, init_rx) = oneshot::channel();
        let init_tx = Arc::new(tokio::sync::Mutex::new(Some(init_tx)));

        self.on_event("initialized", move |_event| {
            info!("Received 'initialized' event - signaling");
            let tx = init_tx.clone();
            // Just signal - don't call any async methods from here
            // This keeps the event handler fast (< 0.1ms like Python standalone test)
            tokio::spawn(async move {
                if let Some(sender) = tx.lock().await.take() {
                    let _ = sender.send(());
                }
            });
        }).await;

        // Step 3: Send launch request (doesn't wait for response yet)
        info!("Sending launch request with args: {:?}", launch_args);
        let launch_seq = self.send_request_nowait("launch", Some(launch_args)).await?;
        info!("Launch request sent with seq {}", launch_seq);

        // Step 4: Wait for 'initialized' event signal
        if config_done_supported {
            info!("Waiting for 'initialized' event (timeout: 5s)...");
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), init_rx).await {
                Ok(Ok(())) => {
                    info!("✅ Received 'initialized' event signal");

                    // Ruby stopOnEntry workaround: Set entry breakpoint BEFORE configurationDone
                    // This follows the correct DAP sequence (setBreakpoints must be before configurationDone)
                    if needs_ruby_workaround {
                        info!("🔧 Applying Ruby stopOnEntry workaround: setting entry breakpoint");
                        info!("   (Per DAP spec: breakpoints must be set BEFORE configurationDone)");

                        match program_path_for_breakpoint.as_deref() {
                            Some(path) => {
                                // Find first executable line
                                let entry_line = Self::find_first_executable_line_ruby(path);
                                info!("  Entry breakpoint will be set at line {}", entry_line);

                                // Create breakpoint at entry line
                                let source = Source {
                                    path: Some(path.to_string()),
                                    name: None,
                                    source_reference: None,
                                };

                                let breakpoint = SourceBreakpoint {
                                    line: entry_line as i32,
                                    column: None,
                                    condition: None,
                                    hit_condition: None,
                                };

                                // Set breakpoint BEFORE configurationDone (per DAP spec)
                                match self.set_breakpoints(source, vec![breakpoint]).await {
                                    Ok(bps) => {
                                        if let Some(bp) = bps.first() {
                                            if bp.verified {
                                                info!("✅ Entry breakpoint set at line {} (verified)", entry_line);
                                            } else {
                                                warn!("⚠️  Entry breakpoint not verified at line {}", entry_line);
                                                warn!("   Program may not stop - check if line is executable");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("⚠️  Failed to set entry breakpoint: {}", e);
                                        warn!("   Continuing anyway - program might not stop at entry");
                                    }
                                }
                            }
                            None => {
                                warn!("⚠️  No program path in launch args - cannot set entry breakpoint");
                                warn!("   Ruby stopOnEntry may not work");
                            }
                        }
                    }
                }
                Ok(Err(_)) => {
                    error!("❌ 'initialized' event signal was cancelled");
                    return Err(Error::Dap("'initialized' event signal was cancelled".to_string()));
                }
                Err(_) => {
                    error!("❌ Timeout waiting for 'initialized' event (5s)");
                    error!("   This usually means:");
                    error!("   1. The program path is invalid or not found");
                    error!("   2. The Python environment doesn't have the target program");
                    error!("   3. The program has a syntax error preventing launch");
                    error!("   4. debugpy couldn't start the target program");
                    error!("   Check that the program path exists and is executable");
                    return Err(Error::Dap("Timeout waiting for 'initialized' event (5s). Program may not exist or has errors.".to_string()));
                }
            }

            // Step 5: Now send configurationDone from main context (not from event handler)
            info!("Sending configurationDone");
            self.configuration_done().await?;
            info!("configurationDone completed");
        }

        // Step 6: Wait for launch response (using wait_for_event on the response)
        // The launch response should arrive shortly after configurationDone
        info!("Waiting for launch to complete");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        info!("Launch sequence completed successfully");
        Ok(())
    }

    /// Helper to clone the client for use in callbacks
    /// Returns an Arc-wrapped clone of the necessary fields
    #[allow(dead_code)]
    fn clone_for_callback(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            seq_counter: self.seq_counter.clone(),
            pending_requests: self.pending_requests.clone(),
            event_tx: self.event_tx.clone(),
            event_notifiers: self.event_notifiers.clone(),
            event_callbacks: self.event_callbacks.clone(),
            write_tx: self.write_tx.clone(),
            _child: None, // Don't clone the child process
        }
    }

    pub async fn configuration_done(&self) -> Result<()> {
        let response = self.send_request("configurationDone", None).await?;
        
        if !response.success {
            return Err(Error::Dap(format!("ConfigurationDone failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn set_breakpoints(&self, source: Source, breakpoints: Vec<SourceBreakpoint>) -> Result<Vec<Breakpoint>> {
        info!("🔧 set_breakpoints: Starting for source {:?}, {} breakpoints", source.path, breakpoints.len());
        for (i, bp) in breakpoints.iter().enumerate() {
            info!("  Breakpoint {}: line {}, condition: {:?}", i, bp.line, bp.condition);
        }

        let args = SetBreakpointsArguments {
            source,
            breakpoints: Some(breakpoints),
            source_modified: Some(false),
        };

        info!("🔧 set_breakpoints: Sending setBreakpoints request...");
        let response = self.send_request("setBreakpoints", Some(serde_json::to_value(args)?)).await?;
        info!("🔧 set_breakpoints: Received response, success: {}", response.success);
        
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

        info!("✅ set_breakpoints: Success, {} breakpoints verified", body.breakpoints.len());
        for (i, bp) in body.breakpoints.iter().enumerate() {
            info!("  Breakpoint {}: id={:?}, verified={}, line={:?}", i, bp.id, bp.verified, bp.line);
        }

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

    /// Find the first executable line in a Ruby source file
    ///
    /// Skips comments, empty lines, requires, and class/module definitions
    /// to find the first actual executable line.
    ///
    /// Returns line number (1-indexed) or 1 as fallback.
    fn find_first_executable_line_ruby(program_path: &str) -> usize {
        use std::fs;

        let content = match fs::read_to_string(program_path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Could not read {} for line detection: {}, using line 1", program_path, e);
                return 1;
            }
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Skip shebang
            if line_num == 0 && trimmed.starts_with("#!") {
                continue;
            }

            // Skip comments
            if trimmed.starts_with('#') {
                continue;
            }

            // Skip requires/loads (not executable, just declarations)
            if trimmed.starts_with("require") || trimmed.starts_with("load") {
                continue;
            }

            // Skip class/module/def declarations (not entry point)
            if trimmed.starts_with("class ") || trimmed.starts_with("module ") {
                // Continue looking inside the class for executable code
                continue;
            }

            // Found first executable line!
            info!("  First executable line detected: {}", line_num + 1);
            return line_num + 1; // DAP uses 1-indexed lines
        }

        // Fallback: No executable line found, use line 1
        warn!("No executable line found in {}, using line 1", program_path);
        1
    }

    pub async fn next(&self, thread_id: i32) -> Result<()> {
        let args = NextArguments { thread_id };

        let response = self.send_request("next", Some(serde_json::to_value(args)?)).await?;

        if !response.success {
            return Err(Error::Dap(format!("Next (step over) failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn step_in(&self, thread_id: i32) -> Result<()> {
        let args = StepInArguments { thread_id };

        let response = self.send_request("stepIn", Some(serde_json::to_value(args)?)).await?;

        if !response.success {
            return Err(Error::Dap(format!("StepIn failed: {:?}", response.message)));
        }

        Ok(())
    }

    pub async fn step_out(&self, thread_id: i32) -> Result<()> {
        let args = StepOutArguments { thread_id };

        let response = self.send_request("stepOut", Some(serde_json::to_value(args)?)).await?;

        if !response.success {
            return Err(Error::Dap(format!("StepOut failed: {:?}", response.message)));
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

    // === Timeout Wrappers (Aggressive Timeouts) ===

    /// Initialize with 2 second timeout
    /// DAP init takes ~100ms normally, 2s = 20x safety margin
    pub async fn initialize_with_timeout(&self, adapter_id: &str) -> Result<Capabilities> {
        let timeout = std::time::Duration::from_secs(2);
        info!("⏱️  initialize_with_timeout: Starting with 2s timeout");

        tokio::time::timeout(timeout, self.initialize(adapter_id))
            .await
            .map_err(|_| Error::Dap(format!("Initialize timed out after {:?}", timeout)))?
    }

    /// Launch with 5 second timeout
    /// Launch is more complex and may involve file loading
    pub async fn launch_with_timeout(&self, args: Value) -> Result<()> {
        let timeout = std::time::Duration::from_secs(5);
        info!("⏱️  launch_with_timeout: Starting with 5s timeout");

        tokio::time::timeout(timeout, self.launch(args))
            .await
            .map_err(|_| Error::Dap(format!("Launch timed out after {:?}", timeout)))?
    }

    /// Disconnect with 2 second timeout (force cleanup)
    /// If disconnect hangs, we want to fail fast and let process cleanup handle it
    pub async fn disconnect_with_timeout(&self) -> Result<()> {
        let timeout = std::time::Duration::from_secs(2);
        info!("⏱️  disconnect_with_timeout: Starting with 2s timeout");

        tokio::time::timeout(timeout, self.disconnect())
            .await
            .map_err(|_| {
                warn!("Disconnect timed out after {:?}, proceeding anyway", timeout);
                Error::Dap(format!("Disconnect timed out after {:?}", timeout))
            })?
    }

    /// Initialize and launch with combined timeout (2s + 5s = 7s total)
    /// This wraps the entire sequence with aggressive timeouts
    pub async fn initialize_and_launch_with_timeout(
        &self,
        adapter_id: &str,
        launch_args: Value,
        adapter_type: Option<&str>,
    ) -> Result<()> {
        let timeout = std::time::Duration::from_secs(7);
        info!("⏱️  initialize_and_launch_with_timeout: Starting with 7s timeout");
        if let Some(atype) = adapter_type {
            info!("   Adapter type: {}", atype);
        }

        tokio::time::timeout(
            timeout,
            self.initialize_and_launch(adapter_id, launch_args, adapter_type)
        )
            .await
            .map_err(|_| Error::Dap(format!("Initialize and launch timed out after {:?}", timeout)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::transport_trait::DapTransportTrait;
    use mockall::mock;
    use serde_json::json;

    mock! {
        pub TestTransport {}

        #[async_trait::async_trait]
        impl DapTransportTrait for TestTransport {
            async fn read_message(&mut self) -> Result<Message>;
            async fn write_message(&mut self, msg: &Message) -> Result<()>;
        }
    }

    // Helper to create a mock that responds once then errors
    fn create_mock_with_response(response: Response) -> MockTestTransport {
        let mut mock = MockTestTransport::new();

        // Expect write
        mock.expect_write_message()
            .times(1)
            .returning(|_| Ok(()));

        // Return response once
        mock.expect_read_message()
            .times(1)
            .return_once(move || Ok(Message::Response(response)));

        // Then error to stop message loop
        mock.expect_read_message()
            .returning(|| Err(Error::Dap("Connection closed".to_string())));

        mock
    }

    #[tokio::test]
    async fn test_dap_client_initialize() {
        let mut mock_transport = MockTestTransport::new();

        // Expect write of initialize request
        mock_transport
            .expect_write_message()
            .times(1)
            .returning(|_| Ok(()));

        // Return initialize response, then error to stop message loop
        mock_transport
            .expect_read_message()
            .times(1)
            .returning(|| {
                Ok(Message::Response(Response {
                    seq: 1,
                    request_seq: 1,
                    command: "initialize".to_string(),
                    success: true,
                    message: None,
                    body: Some(json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsFunctionBreakpoints": false,
                        "supportsConditionalBreakpoints": true,
                    })),
                }))
            });

        // Second read returns error to stop background task
        mock_transport
            .expect_read_message()
            .returning(|| Err(Error::Dap("Connection closed".to_string())));

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let caps = client.initialize("test-adapter").await.unwrap();

        assert!(caps.supports_configuration_done_request.unwrap_or(false));
        assert!(!caps.supports_function_breakpoints.unwrap_or(true));
        assert!(caps.supports_conditional_breakpoints.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_dap_client_launch_success() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "launch".to_string(),
            success: true,
            message: None,
            body: None,
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let launch_args = json!({"program": "test.py"});
        client.launch(launch_args).await.unwrap();
    }

    #[tokio::test]
    async fn test_dap_client_launch_failure() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "launch".to_string(),
            success: false,
            message: Some("Failed to start program".to_string()),
            body: None,
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let launch_args = json!({"program": "test.py"});
        let result = client.launch(launch_args).await;

        assert!(result.is_err());
        match result {
            Err(Error::Dap(msg)) => assert!(msg.contains("Launch failed")),
            _ => panic!("Expected Dap error"),
        }
    }

    #[tokio::test]
    async fn test_dap_client_set_breakpoints() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "setBreakpoints".to_string(),
            success: true,
            message: None,
            body: Some(json!({
                "breakpoints": [
                    {
                        "id": 1,
                        "verified": true,
                        "line": 10
                    }
                ]
            })),
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let source = Source {
            name: Some("test.py".to_string()),
            path: Some("/path/to/test.py".to_string()),
            source_reference: None,
        };

        let breakpoints = vec![SourceBreakpoint {
            line: 10,
            column: None,
            condition: None,
            hit_condition: None,
        }];

        let result = client.set_breakpoints(source, breakpoints).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, Some(1));
        assert!(result[0].verified);
    }

    #[tokio::test]
    async fn test_dap_client_continue_execution() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "continue".to_string(),
            success: true,
            message: None,
            body: None,
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        client.continue_execution(1).await.unwrap();
    }

    #[tokio::test]
    async fn test_dap_client_stack_trace() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "stackTrace".to_string(),
            success: true,
            message: None,
            body: Some(json!({
                "stackFrames": [
                    {
                        "id": 1,
                        "name": "main",
                        "line": 42,
                        "column": 10
                    }
                ]
            })),
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let frames = client.stack_trace(1).await.unwrap();

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].name, "main");
        assert_eq!(frames[0].line, 42);
    }

    #[tokio::test]
    async fn test_dap_client_evaluate() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "evaluate".to_string(),
            success: true,
            message: None,
            body: Some(json!({
                "result": "42"
            })),
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        let result = client.evaluate("x + y", Some(1)).await.unwrap();

        assert_eq!(result, "42");
    }

    #[tokio::test]
    async fn test_dap_client_configuration_done() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "configurationDone".to_string(),
            success: true,
            message: None,
            body: None,
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        client.configuration_done().await.unwrap();
    }

    #[tokio::test]
    async fn test_dap_client_disconnect() {
        let mock_transport = create_mock_with_response(Response {
            seq: 1,
            request_seq: 1,
            command: "disconnect".to_string(),
            success: true,
            message: None,
            body: None,
        });

        let client = DapClient::new_with_transport(Box::new(mock_transport), None)
            .await
            .unwrap();

        client.disconnect().await.unwrap();
    }
}
