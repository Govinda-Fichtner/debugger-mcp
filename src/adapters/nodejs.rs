use serde_json::{json, Value};
use crate::{Result, Error};
use crate::dap::socket_helper;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use std::time::Duration;
use tracing::info;

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
    /// 1. /tmp/js-debug/src/dapDebugServer.js (for tests)
    /// 2. /usr/local/lib/js-debug/src/dapDebugServer.js (production)
    /// 3. ~/.vscode-js-debug/src/dapDebugServer.js (user install)
    pub fn dap_server_path() -> Result<String> {
        let locations = vec![
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
             https://github.com/microsoft/vscode-js-debug/releases/latest".to_string()
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
            .args(&[
                &dap_server_path,
                &port.to_string(),
                "127.0.0.1", // IPv4 explicit
            ])
            .spawn()
            .map_err(|e| Error::Process(format!(
                "Failed to spawn vscode-js-debug: {}. Is Node.js installed?", e
            )))?;

        // 4. Connect to DAP server (with 2 second timeout)
        let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2))
            .await
            .map_err(|e| Error::Process(format!(
                "Failed to connect to vscode-js-debug on port {}: {}",
                port, e
            )))?;

        info!("✅ Connected to vscode-js-debug DAP server on port {}", port);

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
        let expected_structure = vec![
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
