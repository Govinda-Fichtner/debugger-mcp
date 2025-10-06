use serde_json::{json, Value};

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
