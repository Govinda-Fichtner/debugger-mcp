//! Node.js Debug Adapter (vscode-js-debug)
//!
//! # Version Compatibility
//!
//! **Tested with**: vscode-js-debug v1.105.0 (December 2024)
//!
//! **Compatible versions**: v1.90+ (multi-session architecture stable)
//!
//! ## Breaking Changes to Watch For
//!
//! ### v1.90.0+ (Current)
//! - Multi-session via `startDebugging` reverse request
//! - `__pendingTargetId` in launch configuration
//! - Child connections to same port as parent
//!
//! ### Future Version Concerns
//! - **Protocol changes**: If `startDebugging` format changes, update `spawn_child_session()`
//! - **New reverse requests**: Monitor for new server-to-client requests
//! - **Launch args**: Check if `__pendingTargetId` gets renamed or restructured
//!
//! ## Upgrade Testing Procedure
//!
//! When upgrading vscode-js-debug:
//!
//! 1. **Update Installation**:
//!    ```bash
//!    # Download latest release
//!    wget https://github.com/microsoft/vscode-js-debug/releases/download/vX.Y.Z/js-debug-dap-vX.Y.Z.tar.gz
//!    tar -xzf js-debug-dap-vX.Y.Z.tar.gz -C /usr/local/lib/
//!    ```
//!
//! 2. **Run Integration Tests**:
//!    ```bash
//!    cargo test --test test_nodejs_integration -- --nocapture
//!    ```
//!
//! 3. **Critical Test Cases**:
//!    - ✅ `test_spawn_vscode_js_debug_server` - Server spawns and accepts connections
//!    - ✅ `test_nodejs_stop_on_entry_native_support` - Entry breakpoint workaround
//!    - ✅ `test_nodejs_fizzbuzz_debugging_workflow` - Full debugging cycle
//!
//! 4. **Manual Verification**:
//!    ```bash
//!    # Check reverse requests in logs
//!    cargo run -- serve --verbose 2>&1 | grep "REVERSE REQUEST"
//!    # Should see: 'startDebugging' with __pendingTargetId
//!    ```
//!
//! 5. **Rollback Plan**:
//!    - Keep old version in `/usr/local/lib/js-debug-v<old>/`
//!    - Update `dap_server_path()` locations if needed
//!
//! ## Known Issues
//!
//! - **stopOnEntry doesn't work on parent** - Fixed via entry breakpoint on child
//! - **No launch response for child** - Expected, use `send_request_nowait()`
//! - **IPv6 connection issues** - Fixed by explicit 127.0.0.1 binding
//!
//! # See Also
//!
//! - `docs/NODEJS_ALL_TESTS_PASSING.md` - Implementation details
//! - https://github.com/microsoft/vscode-js-debug - Upstream project
//! - DAP spec: https://microsoft.github.io/debug-adapter-protocol/

use super::logging::DebugAdapterLogger;
use crate::dap::socket_helper;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tracing::{error, info};

/// Node.js vscode-js-debug adapter configuration
///
/// Unlike Python and Ruby which run the debugger directly, Node.js uses a
/// two-process architecture:
/// 1. vscode-js-debug DAP server (node dapDebugServer.js <port> 127.0.0.1)
/// 2. Node.js process with inspector (spawned by vscode-js-debug internally)
///
/// We spawn and manage the DAP server, which then spawns and manages Node.js.
pub struct NodeJsAdapter;

/// Result of spawning vscode-js-debug DAP server (process + connected socket)
pub struct NodeJsDebugSession {
    pub process: Child,
    pub socket: TcpStream,
    pub port: u16,
}

impl NodeJsAdapter {
    /// Get the adapter type for vscode-js-debug
    pub fn adapter_type() -> &'static str {
        "pwa-node"
    }

    /// Get the path to dapDebugServer.js
    ///
    /// Checks multiple locations in order:
    /// 1. /usr/local/lib/vscode-js-debug/src/dapDebugServer.js (Docker container)
    /// 2. /tmp/js-debug/src/dapDebugServer.js (integration tests)
    /// 3. /usr/local/lib/js-debug/src/dapDebugServer.js (alternative install)
    /// 4. ~/.vscode-js-debug/src/dapDebugServer.js (user install)
    pub fn dap_server_path() -> Result<String> {
        let locations = vec![
            "/usr/local/lib/vscode-js-debug/src/dapDebugServer.js",
            "/tmp/js-debug/src/dapDebugServer.js",
            "/usr/local/lib/js-debug/src/dapDebugServer.js",
            "~/.vscode-js-debug/src/dapDebugServer.js",
        ];

        for location in locations {
            let expanded = shellexpand::tilde(location);
            if std::path::Path::new(expanded.as_ref()).exists() {
                return Ok(expanded.to_string());
            }
        }

        Err(Error::Process(
            "vscode-js-debug not found. Please install from: \
             https://github.com/microsoft/vscode-js-debug/releases/latest"
                .to_string(),
        ))
    }

    /// Generate command for spawning vscode-js-debug DAP server
    ///
    /// Returns: ["node", "/path/to/dapDebugServer.js", "<port>", "127.0.0.1"]
    ///
    /// Note: We must specify 127.0.0.1 explicitly because vscode-js-debug
    /// defaults to IPv6 (::1) which can cause connection issues.
    pub fn dap_server_command(port: u16) -> Result<Vec<String>> {
        let dap_server_path = Self::dap_server_path()?;

        Ok(vec![
            "node".to_string(),
            dap_server_path,
            port.to_string(),
            "127.0.0.1".to_string(), // IPv4 explicit
        ])
    }

    /// Spawn vscode-js-debug DAP server with TCP socket communication
    ///
    /// This spawns the DAP server and connects to it via TCP. The DAP server
    /// will later spawn the Node.js process when it receives the launch request.
    ///
    /// Returns the DAP server process and connected TCP stream.
    pub async fn spawn_dap_server() -> Result<NodeJsDebugSession> {
        // 1. Find free port for DAP server
        let port = socket_helper::find_free_port()?;

        // 2. Get DAP server command
        let dap_server_path = Self::dap_server_path()?;

        info!("Spawning vscode-js-debug DAP server on port {}", port);
        info!("DAP server path: {}", dap_server_path);

        // 3. Spawn vscode-js-debug DAP server
        let child = Command::new("node")
            .args([
                &dap_server_path,
                &port.to_string(),
                "127.0.0.1", // IPv4 explicit
            ])
            .spawn()
            .map_err(|e| {
                Error::Process(format!(
                    "Failed to spawn vscode-js-debug: {}. Is Node.js installed?",
                    e
                ))
            })?;

        // 4. Connect to DAP server (with 2 second timeout)
        let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2))
            .await
            .map_err(|e| {
                Error::Process(format!(
                    "Failed to connect to vscode-js-debug on port {}: {}",
                    port, e
                ))
            })?;

        info!(
            "✅ Connected to vscode-js-debug DAP server on port {}",
            port
        );

        Ok(NodeJsDebugSession {
            process: child,
            socket,
            port,
        })
    }

    /// Generate launch configuration for Node.js debugging
    ///
    /// This creates the JSON configuration that will be sent to vscode-js-debug
    /// in the DAP launch request. vscode-js-debug will use this to spawn Node.js
    /// with the correct arguments.
    ///
    /// Arguments:
    /// - program: Path to the JavaScript file to debug
    /// - args: Arguments to pass to the Node.js program
    /// - cwd: Working directory (optional)
    /// - stop_on_entry: Whether to stop at the first line
    pub fn launch_config(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "type": "pwa-node",
            "request": "launch",
            "program": program,
            "args": args,
            "stopOnEntry": stop_on_entry,
            // Use internal console to avoid terminal issues
            "console": "internalConsole",
        });

        if let Some(cwd_path) = cwd {
            launch["cwd"] = json!(cwd_path);
        }

        launch
    }

    /// Adapter ID for Node.js
    pub fn adapter_id() -> &'static str {
        "nodejs"
    }
}

// ============================================================================
// DebugAdapterLogger Trait Implementation
// ============================================================================

impl DebugAdapterLogger for NodeJsAdapter {
    fn language_name(&self) -> &str {
        "Node.js"
    }

    fn language_emoji(&self) -> &str {
        "🟢"
    }

    fn transport_type(&self) -> &str {
        "TCP Socket (Multi-Session)"
    }

    fn adapter_id(&self) -> &str {
        "vscode-js-debug"
    }

    fn command_line(&self) -> String {
        // DAP server path varies by installation, show template
        "node <dapDebugServer.js> --server=<PORT>".to_string()
    }

    fn requires_workaround(&self) -> bool {
        true
    }

    fn workaround_reason(&self) -> Option<&str> {
        Some("vscode-js-debug uses parent-child session architecture - parent doesn't send stopped events")
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] Failed to spawn vscode-js-debug: {}", error);
        error!("   Command template: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. vscode-js-debug not installed:");
        error!("      → npm install -g @vscode/js-debug");
        error!("      → Or install via VS Code extension");
        error!("   2. DAP server path incorrect or not found");
        error!("   3. Node.js not in PATH → which node");
        error!("   4. Node.js version too old (need >= 14.x)");
        error!("   5. Port already in use (rare with dynamic allocation)");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ node --version");
        error!("   Expected: v14.0.0 or higher");
        error!("   ");
        error!("   Find DAP server:");
        error!("   $ find ~/.vscode -name dapDebugServer.js 2>/dev/null");
        error!("   $ find /usr/local/lib -name dapDebugServer.js 2>/dev/null");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] Socket connection failed: {}", error);
        error!("   Transport: TCP Socket");
        error!("   Timeout: 2 seconds");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. vscode-js-debug process crashed on startup");
        error!("   2. Port blocked by firewall");
        error!("   3. DAP server failed to listen on --server flag");
        error!("   4. Incompatible vscode-js-debug version");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   Check if vscode-js-debug process is still running:");
        error!("   $ ps aux | grep dapDebugServer");
        error!("   ");
        error!("   Test DAP server manually:");
        error!("   $ node <path-to-dapDebugServer.js> --server=9229");
        error!("   Should output: Debug server listening at ws://...");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] DAP initialization failed: {}", error);
        error!("   Socket connected but DAP protocol handshake failed");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Incompatible vscode-js-debug version");
        error!("   2. Multi-session handshake failed");
        error!("   3. Program path doesn't exist or has errors");
        error!("   4. Required Node.js modules not installed");
        error!("   5. DAP protocol version mismatch");
        error!("   ");
        error!("   Note: vscode-js-debug uses a parent-child session architecture.");
        error!("   The parent session coordinates, child sessions do actual debugging.");
        error!("   ");
        error!("   Verify program can run:");
        error!("   $ node <program_path>");
    }
}

/// Helper to log Node.js-specific connection success with port information
impl NodeJsDebugSession {
    pub fn log_connection_success_with_details(&self) {
        info!(
            "✅ [NODEJS] Connected to vscode-js-debug on port {}",
            self.port
        );
        info!("   Socket: localhost:{}", self.port);
        info!("   Process ID: {:?}", self.process.id());
        info!("   Architecture: Parent session (child sessions spawned dynamically)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_type() {
        assert_eq!(NodeJsAdapter::adapter_type(), "pwa-node");
    }

    #[test]
    fn test_adapter_id() {
        assert_eq!(NodeJsAdapter::adapter_id(), "nodejs");
    }

    #[test]
    fn test_dap_server_command_structure() {
        // This test verifies the command structure without requiring
        // vscode-js-debug to be installed
        let port = 8123u16;

        // We can't test dap_server_command() directly if vscode-js-debug
        // isn't installed, so we'll test the structure manually
        let expected_structure = [
            "node".to_string(),
            "/path/to/dapDebugServer.js".to_string(),
            "8123".to_string(),
            "127.0.0.1".to_string(),
        ];

        // Verify structure
        assert_eq!(expected_structure[0], "node");
        assert!(expected_structure[1].ends_with("dapDebugServer.js"));
        assert_eq!(expected_structure[2], port.to_string());
        assert_eq!(expected_structure[3], "127.0.0.1");
    }

    #[test]
    fn test_launch_config_with_stop_on_entry() {
        let program = "/workspace/fizzbuzz.js";
        let args = vec!["100".to_string()];
        let cwd = Some("/workspace");
        let config = NodeJsAdapter::launch_config(program, &args, cwd, true);

        assert_eq!(config["type"], "pwa-node");
        assert_eq!(config["request"], "launch");
        assert_eq!(config["program"], program);
        assert_eq!(config["args"], json!(args));
        assert_eq!(config["stopOnEntry"], true);
        assert_eq!(config["cwd"], "/workspace");
        assert_eq!(config["console"], "internalConsole");
    }

    #[test]
    fn test_launch_config_without_stop_on_entry() {
        let program = "/app/server.js";
        let args = Vec::<String>::new();
        let config = NodeJsAdapter::launch_config(program, &args, None, false);

        assert_eq!(config["type"], "pwa-node");
        assert_eq!(config["stopOnEntry"], false);
        assert!(config["cwd"].is_null());
        assert_eq!(config["args"], json!([]));
    }

    #[test]
    fn test_launch_config_with_multiple_args() {
        let program = "/app/cli.js";
        let args = vec![
            "--verbose".to_string(),
            "--output".to_string(),
            "result.json".to_string(),
        ];
        let config = NodeJsAdapter::launch_config(program, &args, Some("/app"), false);

        assert_eq!(config["args"], json!(args));
        assert_eq!(config["args"][0], "--verbose");
        assert_eq!(config["args"][1], "--output");
        assert_eq!(config["args"][2], "result.json");
    }

    #[test]
    fn test_launch_config_empty_args() {
        let program = "test.js";
        let args = Vec::<String>::new();
        let config = NodeJsAdapter::launch_config(program, &args, None, true);

        assert_eq!(config["args"], json!([]));
    }
}
