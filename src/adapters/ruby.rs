use serde_json::{json, Value};

/// Ruby rdbg (debug gem) adapter configuration
pub struct RubyAdapter;

impl RubyAdapter {
    pub fn command() -> String {
        "rdbg".to_string()
    }

    pub fn args_with_options(stop_on_entry: bool) -> Vec<String> {
        // Use --command mode for stdio communication (not --open which uses sockets)
        // -O flag runs the program (similar to debugpy's behavior)
        let mut args = vec![
            "--command".to_string(),
        ];

        // Add --nonstop flag if we DON'T want to stop on entry
        // Default rdbg behavior is to stop at program start
        if !stop_on_entry {
            args.push("--nonstop".to_string());
        }

        args
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
        let args = RubyAdapter::args_with_options(true);
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], "--command");
        // Should NOT have --nonstop when stopOnEntry is true
        assert!(!args.contains(&"--nonstop".to_string()));
    }

    #[test]
    fn test_args_without_stop_on_entry() {
        let args = RubyAdapter::args_with_options(false);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "--command");
        assert_eq!(args[1], "--nonstop");
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
