use serde_json::{json, Value};
use crate::{Result, Error};
use crate::dap::socket_helper;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use std::time::Duration;
use tracing::info;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command() {
        assert_eq!(RubyAdapter::command(), "rdbg");
    }

    #[test]
    fn test_args_with_stop_on_entry() {
        let program = "/path/to/script.rb";
        let program_args = vec!["arg1".to_string(), "arg2".to_string()];
        let args = RubyAdapter::args_with_options(program, &program_args, true);

        assert_eq!(args.len(), 4); // --stop-at-load + program + 2 args
        assert_eq!(args[0], "--stop-at-load");
        assert_eq!(args[1], program);
        assert_eq!(args[2], "arg1");
        assert_eq!(args[3], "arg2");
        // Should NOT have --nonstop when stopOnEntry is true
        assert!(!args.contains(&"--nonstop".to_string()));
    }

    #[test]
    fn test_args_without_stop_on_entry() {
        let program = "/path/to/script.rb";
        let program_args = vec!["arg1".to_string()];
        let args = RubyAdapter::args_with_options(program, &program_args, false);

        assert_eq!(args.len(), 3); // --nonstop + program + 1 arg
        assert_eq!(args[0], "--nonstop");
        assert_eq!(args[1], program);
        assert_eq!(args[2], "arg1");
        // Should NOT have --stop-at-load when stopOnEntry is false
        assert!(!args.contains(&"--stop-at-load".to_string()));
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
