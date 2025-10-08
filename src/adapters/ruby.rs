use serde_json::{json, Value};
use crate::{Result, Error};
use crate::dap::socket_helper;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use std::time::Duration;
use tracing::{error, info};
use super::logging::DebugAdapterLogger;

/// Ruby rdbg (debug gem) adapter configuration
///
/// Unlike Python's debugpy which has a separate adapter server,
/// rdbg runs the program directly and communicates via TCP socket.
pub struct RubyAdapter;

/// Result of spawning Ruby debugger (process + connected socket)
pub struct RubyDebugSession {
    pub process: Child,
    pub socket: TcpStream,
    pub port: u16,
}

impl RubyAdapter {
    pub fn command() -> String {
        "rdbg".to_string()
    }

    /// Spawn rdbg with socket-based DAP communication
    ///
    /// This spawns `rdbg --open --port <PORT> program.rb` and connects to the socket.
    /// Returns the process and connected TCP stream for DAP communication.
    pub async fn spawn(
        program: &str,
        program_args: &[String],
        stop_on_entry: bool,
    ) -> Result<RubyDebugSession> {
        // 1. Find free port
        let port = socket_helper::find_free_port()?;

        // 2. Build command args
        let mut args = vec![
            "--open".to_string(),
            "--port".to_string(),
            port.to_string(),
        ];

        // Add stop behavior flag
        if stop_on_entry {
            args.push("--stop-at-load".to_string());
        } else {
            args.push("--nonstop".to_string());
        }

        // Add program path
        args.push(program.to_string());

        // Add program arguments
        args.extend(program_args.iter().cloned());

        info!("Spawning rdbg on port {}: rdbg {:?}", port, args);

        // 3. Spawn rdbg process
        let child = Command::new("rdbg")
            .args(&args)
            .spawn()
            .map_err(|e| Error::Process(format!("Failed to spawn rdbg: {}", e)))?;

        // 4. Connect to socket (with 2 second timeout)
        let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2))
            .await
            .map_err(|e| Error::Process(format!(
                "Failed to connect to rdbg on port {}: {}",
                port, e
            )))?;

        Ok(RubyDebugSession {
            process: child,
            socket,
            port,
        })
    }

    pub fn adapter_id() -> &'static str {
        "rdbg"
    }

    pub fn launch_args_with_options(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "request": "launch",
            "type": "ruby",
            "program": program,
            "args": args,
            "stopOnEntry": stop_on_entry,
            // Ruby debugger uses localfs for path mapping
            "localfs": true,
        });

        if let Some(cwd_path) = cwd {
            launch["cwd"] = json!(cwd_path);
        }

        launch
    }
}

// ============================================================================
// DebugAdapterLogger Trait Implementation
// ============================================================================

impl DebugAdapterLogger for RubyAdapter {
    fn language_name(&self) -> &str {
        "Ruby"
    }

    fn language_emoji(&self) -> &str {
        "💎"
    }

    fn transport_type(&self) -> &str {
        "TCP Socket"
    }

    fn adapter_id(&self) -> &str {
        "rdbg"
    }

    fn command_line(&self) -> String {
        // Port is allocated dynamically, show template
        "rdbg --open --port <PORT> [--stop-at-load|--nonstop] <program> [args...]".to_string()
    }

    fn requires_workaround(&self) -> bool {
        true
    }

    fn workaround_reason(&self) -> Option<&str> {
        Some("rdbg socket mode doesn't honor --stop-at-load flag")
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] Failed to spawn rdbg: {}", error);
        error!("   Command template: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. debug gem not installed → gem install debug");
        error!("   2. rdbg not in PATH → which rdbg");
        error!("   3. Ruby version < 3.1 → ruby --version");
        error!("   4. Port already in use (rare with dynamic allocation)");
        error!("   5. Permission denied on port binding");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ gem list debug");
        error!("   Expected: debug (>= 1.0.0)");
        error!("   ");
        error!("   $ rdbg --version");
        error!("   Expected: rdbg 1.x.x or higher");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] Socket connection failed: {}", error);
        error!("   Transport: TCP Socket");
        error!("   Timeout: 2 seconds");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. rdbg process crashed before opening socket");
        error!("   2. Port blocked by firewall");
        error!("   3. Program exited immediately (syntax error or file not found)");
        error!("   4. Socket binding failed (port already in use)");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   Check if rdbg process is still running:");
        error!("   $ ps aux | grep rdbg");
        error!("   ");
        error!("   Verify program can run:");
        error!("   $ ruby <program_path>");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] DAP initialization failed: {}", error);
        error!("   Socket connected but DAP protocol handshake failed");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Incompatible rdbg version (need >= 1.0.0)");
        error!("   2. Program has Ruby syntax errors");
        error!("   3. Required gems not installed");
        error!("   4. DAP protocol version mismatch");
        error!("   ");
        error!("   Verify rdbg compatibility:");
        error!("   $ rdbg --version");
        error!("   ");
        error!("   Test program syntax:");
        error!("   $ ruby -c <program_path>");
    }
}

/// Helper to log Ruby-specific connection success with port information
impl RubyDebugSession {
    pub fn log_connection_success_with_port(&self) {
        info!("✅ [RUBY] Connected to rdbg on port {}", self.port);
        info!("   Socket: localhost:{}", self.port);
        info!("   Process ID: {:?}", self.process.id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command() {
        assert_eq!(RubyAdapter::command(), "rdbg");
    }


    #[test]
    fn test_adapter_id() {
        assert_eq!(RubyAdapter::adapter_id(), "rdbg");
    }

    #[test]
    fn test_launch_args_without_cwd() {
        let program = "/path/to/script.rb";
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let launch = RubyAdapter::launch_args_with_options(program, &args, None, true);

        assert_eq!(launch["request"], "launch");
        assert_eq!(launch["type"], "ruby");
        assert_eq!(launch["program"], program);
        assert_eq!(launch["args"], json!(args));
        assert_eq!(launch["stopOnEntry"], true);
        assert_eq!(launch["localfs"], true);
        assert!(launch["cwd"].is_null());
    }

    #[test]
    fn test_launch_args_with_cwd() {
        let program = "/path/to/script.rb";
        let args = vec!["arg1".to_string()];
        let cwd = Some("/working/dir");
        let launch = RubyAdapter::launch_args_with_options(program, &args, cwd, false);

        assert_eq!(launch["cwd"], "/working/dir");
        assert_eq!(launch["program"], program);
        assert_eq!(launch["stopOnEntry"], false);
    }

    #[test]
    fn test_launch_args_empty_args() {
        let program = "test.rb";
        let args = Vec::<String>::new();
        let launch = RubyAdapter::launch_args_with_options(program, &args, None, true);

        assert_eq!(launch["args"], json!([]));
    }
}
