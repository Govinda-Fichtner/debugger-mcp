use serde_json::{json, Value};

/// Python debugpy adapter configuration
pub struct PythonAdapter;

impl PythonAdapter {
    pub fn command() -> String {
        "python".to_string()
    }

    pub fn args() -> Vec<String> {
        vec![
            "-m".to_string(),
            "debugpy.adapter".to_string(),
        ]
    }

    pub fn adapter_id() -> &'static str {
        "debugpy"
    }

    pub fn launch_args(program: &str, args: &[String], cwd: Option<&str>) -> Value {
        let mut launch = json!({
            "request": "launch",
            "type": "python",
            "program": program,
            "args": args,
            "console": "integratedTerminal",
            "stopOnEntry": false,
        });

        if let Some(cwd_path) = cwd {
            launch["cwd"] = json!(cwd_path);
        }

        launch
    }
}
