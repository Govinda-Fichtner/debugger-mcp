use super::logging::DebugAdapterLogger;
use crate::dap::socket_helper;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tracing::{error, info};

/// Go Delve debugger adapter configuration
///
/// Delve provides native DAP (Debug Adapter Protocol) support via `dlv dap` command.
/// Unlike other debuggers that may require an adapter layer, Delve speaks DAP natively,
/// making integration straightforward.
///
/// ## Multi-File Support
///
/// Delve automatically handles:
/// - Single `.go` files
/// - Multi-file packages (all files in a directory)
/// - Go modules (directories with `go.mod`)
///
/// No special detection or compilation step needed - Delve compiles on-the-fly.
pub struct GoAdapter;

/// Result of spawning Go debugger (process + connected socket)
pub struct GoDebugSession {
    pub process: Child,
    pub socket: TcpStream,
    pub port: u16,
}

impl GoAdapter {
    pub fn command() -> String {
        "dlv".to_string()
    }

    /// Spawn Delve with DAP communication over TCP socket
    ///
    /// This spawns `dlv dap --listen=127.0.0.1:<PORT>` and connects to the socket.
    /// Returns the process and connected TCP stream for DAP communication.
    ///
    /// ## Multi-File Support
    ///
    /// The same spawn function works for all Go program types:
    /// - Single file: `program = "main.go"`
    /// - Package: `program = "/path/to/package/"`
    /// - Module: `program = "/path/to/module/"` (with go.mod)
    ///
    /// Delve determines the type automatically.
    pub async fn spawn(
        _program: &str,
        _program_args: &[String],
        _stop_on_entry: bool,
    ) -> Result<GoDebugSession> {
        // 1. Find free port
        let port = socket_helper::find_free_port()?;

        // 2. Build dlv dap command args
        let args = vec![
            "dap".to_string(),
            "--listen".to_string(),
            format!("127.0.0.1:{}", port),
        ];

        info!("Spawning dlv on port {}: dlv {:?}", port, args);

        // 3. Spawn dlv process
        let child = Command::new("dlv")
            .args(&args)
            .spawn()
            .map_err(|e| Error::Process(format!("Failed to spawn dlv: {}", e)))?;

        // 4. Connect to socket (with 3 second timeout - dlv needs a moment to start)
        let socket = socket_helper::connect_with_retry(port, Duration::from_secs(3))
            .await
            .map_err(|e| {
                Error::Process(format!("Failed to connect to dlv on port {}: {}", port, e))
            })?;

        Ok(GoDebugSession {
            process: child,
            socket,
            port,
        })
    }

    pub fn adapter_id() -> &'static str {
        "delve"
    }

    /// Generate DAP launch configuration for Go debugging
    ///
    /// ## Multi-File Support
    ///
    /// The `program` parameter can be:
    /// - A `.go` file path: `"/path/to/main.go"`
    /// - A package directory: `"/path/to/package/"`
    /// - A module directory: `"/path/to/module/"` (containing go.mod)
    ///
    /// Delve automatically detects the type and compiles appropriately.
    ///
    /// ## Mode Field
    ///
    /// Go-specific `mode` field specifies how to launch:
    /// - `"debug"`: Debug a Go program (default)
    /// - `"test"`: Debug Go tests
    /// - `"exec"`: Debug a pre-compiled binary
    pub fn launch_args_with_options(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "request": "launch",
            "type": "go",
            "mode": "debug",
            "program": program,
            "args": args,
            "stopOnEntry": stop_on_entry,
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

impl DebugAdapterLogger for GoAdapter {
    fn language_name(&self) -> &str {
        "Go"
    }

    fn language_emoji(&self) -> &str {
        "🐹"
    }

    fn transport_type(&self) -> &str {
        "TCP Socket"
    }

    fn adapter_id(&self) -> &str {
        "delve"
    }

    fn command_line(&self) -> String {
        "dlv dap --listen=127.0.0.1:<PORT>".to_string()
    }

    fn requires_workaround(&self) -> bool {
        false
    }

    fn workaround_reason(&self) -> Option<&str> {
        None
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [GO] Failed to spawn dlv: {}", error);
        error!("   Command: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Delve not installed → go install github.com/go-delve/delve/cmd/dlv@latest");
        error!("   2. dlv not in PATH → which dlv");
        error!("   3. Go toolchain not installed → go version");
        error!("   4. Port already in use (rare with dynamic allocation)");
        error!("   5. Permission denied on port binding");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ go version");
        error!("   Expected: go version go1.21+ or higher");
        error!("   ");
        error!("   $ dlv version");
        error!("   Expected: Delve Debugger, Version: 1.20.0 or higher");
        error!("   ");
        error!("   Installation:");
        error!("   $ go install github.com/go-delve/delve/cmd/dlv@latest");
        error!("   $ export PATH=$PATH:$(go env GOPATH)/bin");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [GO] Socket connection failed: {}", error);
        error!("   Transport: TCP Socket");
        error!("   Timeout: 3 seconds");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. dlv process crashed before opening socket");
        error!("   2. Port blocked by firewall");
        error!("   3. Program has Go syntax errors (dlv tries to compile on launch)");
        error!("   4. Socket binding failed (port already in use)");
        error!("   5. Go module dependencies not downloaded (run: go mod download)");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   Check if dlv process is still running:");
        error!("   $ ps aux | grep dlv");
        error!("   ");
        error!("   Verify program can compile:");
        error!("   $ go build <program_path>");
        error!("   ");
        error!("   For Go modules:");
        error!("   $ cd <program_directory>");
        error!("   $ go mod download");
        error!("   $ go mod tidy");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [GO] DAP initialization failed: {}", error);
        error!("   Socket connected but DAP protocol handshake failed");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Incompatible Delve version (need >= 1.20.0 for stable DAP)");
        error!("   2. Program has Go syntax errors");
        error!("   3. Go module dependencies missing");
        error!("   4. DAP protocol version mismatch");
        error!("   ");
        error!("   Verify Delve compatibility:");
        error!("   $ dlv version");
        error!("   Expected: Version 1.20.0 or higher");
        error!("   ");
        error!("   Test program compilation:");
        error!("   $ go build <program_path>");
        error!("   ");
        error!("   Update Delve:");
        error!("   $ go install github.com/go-delve/delve/cmd/dlv@latest");
    }
}

/// Helper to log Go-specific connection success with port information
impl GoDebugSession {
    pub fn log_connection_success_with_port(&self) {
        info!("✅ [GO] Connected to dlv on port {}", self.port);
        info!("   Socket: localhost:{}", self.port);
        info!("   Process ID: {:?}", self.process.id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command() {
        assert_eq!(GoAdapter::command(), "dlv");
    }

    #[test]
    fn test_adapter_id() {
        assert_eq!(GoAdapter::adapter_id(), "delve");
    }

    #[test]
    fn test_launch_args_without_cwd() {
        let program = "/path/to/main.go";
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let launch = GoAdapter::launch_args_with_options(program, &args, None, true);

        assert_eq!(launch["request"], "launch");
        assert_eq!(launch["type"], "go");
        assert_eq!(launch["mode"], "debug");
        assert_eq!(launch["program"], program);
        assert_eq!(launch["args"], json!(args));
        assert_eq!(launch["stopOnEntry"], true);
        assert!(launch["cwd"].is_null());
    }

    #[test]
    fn test_launch_args_with_cwd() {
        let program = "/path/to/package/";
        let args = vec!["arg1".to_string()];
        let cwd = Some("/working/dir");
        let launch = GoAdapter::launch_args_with_options(program, &args, cwd, false);

        assert_eq!(launch["cwd"], "/working/dir");
        assert_eq!(launch["program"], program);
        assert_eq!(launch["stopOnEntry"], false);
    }

    #[test]
    fn test_launch_args_empty_args() {
        let program = "test.go";
        let args = Vec::<String>::new();
        let launch = GoAdapter::launch_args_with_options(program, &args, None, true);

        assert_eq!(launch["args"], json!([]));
    }

    #[test]
    fn test_launch_args_multifile_package() {
        // Test that package directory works (multi-file support)
        let program = "/path/to/mypackage/";
        let args = Vec::<String>::new();
        let launch = GoAdapter::launch_args_with_options(program, &args, None, false);

        assert_eq!(launch["program"], "/path/to/mypackage/");
        assert_eq!(launch["mode"], "debug");
    }

    #[test]
    fn test_debug_adapter_logger_trait() {
        let adapter = GoAdapter;

        assert_eq!(adapter.language_name(), "Go");
        assert_eq!(adapter.language_emoji(), "🐹");
        assert_eq!(adapter.transport_type(), "TCP Socket");
        assert_eq!(adapter.adapter_id(), "delve");
        assert_eq!(adapter.command_line(), "dlv dap --listen=127.0.0.1:<PORT>");
        assert!(!adapter.requires_workaround());
        assert_eq!(adapter.workaround_reason(), None);
    }
}
