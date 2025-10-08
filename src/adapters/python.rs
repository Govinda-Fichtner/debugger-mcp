use serde_json::{json, Value};
use super::logging::DebugAdapterLogger;
use tracing::error;
use std::error::Error;

/// Python debugpy adapter configuration
pub struct PythonAdapter;

impl PythonAdapter {
    pub fn command() -> String {
        "python".to_string()
    }

    pub fn args() -> Vec<String> {
        vec![
            // Add Python flag to disable frozen modules (helps with Python 3.11+)
            "-Xfrozen_modules=off".to_string(),
            "-m".to_string(),
            "debugpy.adapter".to_string(),
        ]
    }

    pub fn adapter_id() -> &'static str {
        "debugpy"
    }

    pub fn launch_args(program: &str, args: &[String], cwd: Option<&str>) -> Value {
        Self::launch_args_with_options(program, args, cwd, false)
    }

    pub fn launch_args_with_options(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "request": "launch",
            "type": "python",
            "program": program,
            "args": args,
            "console": "internalConsole",  // Use internalConsole instead of integratedTerminal
            "stopOnEntry": stop_on_entry,
            // Add Python options to disable frozen modules (Python 3.11+)
            "pythonArgs": ["-Xfrozen_modules=off"],
            // Use the same Python interpreter that's running the adapter
            "python": "python",
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

impl DebugAdapterLogger for PythonAdapter {
    fn language_name(&self) -> &str {
        "Python"
    }

    fn language_emoji(&self) -> &str {
        "🐍"
    }

    fn transport_type(&self) -> &str {
        "STDIO"
    }

    fn adapter_id(&self) -> &str {
        "debugpy"
    }

    fn command_line(&self) -> String {
        let args = Self::args();
        format!("python {}", args.join(" "))
    }

    fn requires_workaround(&self) -> bool {
        false
    }

    fn log_spawn_error(&self, error: &dyn Error) {
        error!("❌ [PYTHON] Failed to spawn debugpy adapter: {}", error);
        error!("   Command: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. debugpy not installed → pip install debugpy");
        error!("   2. python not in PATH → which python");
        error!("   3. Python version < 3.7 → python --version");
        error!("   4. Virtual environment not activated");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ python -c 'import debugpy; print(debugpy.__version__)'");
        error!("   Expected: 1.6.0 or higher");
    }

    fn log_connection_error(&self, error: &dyn Error) {
        error!("❌ [PYTHON] Adapter connection failed: {}", error);
        error!("   Transport: STDIO");
        error!("   This shouldn't happen with STDIO transport");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Adapter process crashed on startup");
        error!("   2. Python exception during debugpy.adapter initialization");
        error!("   3. STDIO pipes broken or closed unexpectedly");
        error!("   ");
        error!("   The adapter process may have written error to stderr.");
        error!("   Check process stderr output for Python exceptions.");
    }

    fn log_init_error(&self, error: &dyn Error) {
        error!("❌ [PYTHON] DAP initialization failed: {}", error);
        error!("   The adapter started but couldn't complete DAP handshake");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Program path doesn't exist or is not accessible");
        error!("   2. Program has Python syntax errors");
        error!("   3. Required modules not installed in Python environment");
        error!("   4. Incompatible debugpy version (need >= 1.6.0)");
        error!("   ");
        error!("   Verify program can run:");
        error!("   $ python <program_path>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command() {
        assert_eq!(PythonAdapter::command(), "python");
    }

    #[test]
    fn test_args() {
        let args = PythonAdapter::args();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "-Xfrozen_modules=off");
        assert_eq!(args[1], "-m");
        assert_eq!(args[2], "debugpy.adapter");
    }

    #[test]
    fn test_adapter_id() {
        assert_eq!(PythonAdapter::adapter_id(), "debugpy");
    }

    #[test]
    fn test_launch_args_without_cwd() {
        let program = "/path/to/script.py";
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let launch = PythonAdapter::launch_args(program, &args, None);

        assert_eq!(launch["request"], "launch");
        assert_eq!(launch["type"], "python");
        assert_eq!(launch["program"], program);
        assert_eq!(launch["args"], json!(args));
        assert_eq!(launch["console"], "internalConsole");
        assert!(!launch["stopOnEntry"].as_bool().unwrap_or(true));
        assert!(launch["cwd"].is_null());
    }

    #[test]
    fn test_launch_args_with_cwd() {
        let program = "/path/to/script.py";
        let args = vec!["arg1".to_string()];
        let cwd = Some("/working/dir");
        let launch = PythonAdapter::launch_args(program, &args, cwd);

        assert_eq!(launch["cwd"], "/working/dir");
        assert_eq!(launch["program"], program);
    }

    #[test]
    fn test_launch_args_empty_args() {
        let program = "test.py";
        let args = Vec::<String>::new();
        let launch = PythonAdapter::launch_args(program, &args, None);

        assert_eq!(launch["args"], json!([]));
    }
}
