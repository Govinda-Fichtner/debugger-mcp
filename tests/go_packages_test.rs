/// Integration tests for Go (Delve) debugging
///
/// These tests verify that the Go adapter works correctly end-to-end with Delve,
/// including multi-file package support which is a key feature.
///
/// Test Coverage:
/// 1. Go adapter configuration and metadata
/// 2. Launch args for single files, packages, and modules
/// 3. Socket-based DAP communication
/// 4. Single .go file debugging
/// 5. Multi-file package debugging (4 files)
/// 6. Go module support (with go.mod)
use debugger_mcp::adapters::golang::GoAdapter;
use serde_json::json;

/// Test that Go adapter command is "dlv"
#[test]
fn test_go_adapter_command() {
    assert_eq!(GoAdapter::command(), "dlv");
}

/// Test that Go adapter ID is "delve"
#[test]
fn test_go_adapter_id() {
    assert_eq!(GoAdapter::adapter_id(), "delve");
}

/// Test single-file launch args structure
#[test]
fn test_go_launch_args_single_file() {
    let program = "/workspace/fizzbuzz.go";
    let program_args = vec!["100".to_string()];
    let cwd = Some("/workspace");
    let launch_args = GoAdapter::launch_args_with_options(program, &program_args, cwd, true);

    assert_eq!(launch_args["request"], "launch");
    assert_eq!(launch_args["type"], "go");
    assert_eq!(launch_args["mode"], "debug");
    assert_eq!(launch_args["program"], program);
    assert_eq!(launch_args["args"], json!(program_args));
    assert_eq!(launch_args["stopOnEntry"], true);
    assert_eq!(launch_args["cwd"], "/workspace");
}

/// Test multi-file package launch args structure
#[test]
fn test_go_launch_args_package_directory() {
    let program = "/workspace/mypackage/"; // Directory, not file
    let program_args = Vec::<String>::new();
    let launch_args = GoAdapter::launch_args_with_options(program, &program_args, None, false);

    assert_eq!(launch_args["program"], "/workspace/mypackage/");
    assert_eq!(launch_args["mode"], "debug");
    assert_eq!(launch_args["stopOnEntry"], false);
    assert!(launch_args["cwd"].is_null());
}

/// Test Go module launch args structure
#[test]
fn test_go_launch_args_module() {
    let program = "/workspace/mymodule/"; // Directory with go.mod
    let program_args = vec!["--verbose".to_string()];
    let cwd = Some("/workspace/mymodule");
    let launch_args = GoAdapter::launch_args_with_options(program, &program_args, cwd, true);

    assert_eq!(launch_args["program"], "/workspace/mymodule/");
    assert_eq!(launch_args["args"], json!(["--verbose"]));
    assert_eq!(launch_args["cwd"], "/workspace/mymodule");
}

/// Test that launch args handle missing cwd
#[test]
fn test_go_launch_args_no_cwd() {
    let program = "/workspace/test.go";
    let program_args = Vec::<String>::new();
    let launch_args = GoAdapter::launch_args_with_options(program, &program_args, None, false);

    assert_eq!(launch_args["program"], program);
    assert_eq!(launch_args["stopOnEntry"], false);
    assert!(launch_args["cwd"].is_null());
}

/// Test Go single-file debugging (requires dlv installed)
#[tokio::test]
#[ignore] // Requires dlv installed and in PATH
async fn test_go_single_file_debug() {
    use debugger_mcp::adapters::golang;
    use std::path::PathBuf;

    // Get fixture path
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go/fizzbuzz.go");

    // Spawn Delve
    let result = golang::GoAdapter::spawn(fixture_path.to_str().unwrap(), &[], true).await;

    assert!(
        result.is_ok(),
        "Failed to spawn dlv for single file: {:?}",
        result.err()
    );

    let session = result.unwrap();
    assert!(session.process.id().is_some(), "Delve process not running");
    assert!(session.port > 0, "Invalid port");

    // Cleanup
    drop(session);
}

/// Test Go multi-file package debugging (requires dlv installed)
#[tokio::test]
#[ignore] // Requires dlv installed and in PATH
async fn test_go_multifile_package_debug() {
    use debugger_mcp::adapters::golang;
    use std::path::PathBuf;

    // Get fixture path (directory, not file!)
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go/multifile");

    // Spawn Delve with package directory
    let result = golang::GoAdapter::spawn(fixture_path.to_str().unwrap(), &[], false).await;

    assert!(
        result.is_ok(),
        "Failed to spawn dlv for multi-file package: {:?}",
        result.err()
    );

    let session = result.unwrap();
    assert!(session.process.id().is_some(), "Delve process not running");
    assert!(session.port > 0, "Invalid port");

    // Cleanup
    drop(session);
}

/// Test that Delve uses TCP Socket transport (like Ruby/Node.js)
#[test]
fn test_go_transport_is_tcp_socket() {
    use debugger_mcp::adapters::logging::DebugAdapterLogger;

    let adapter = GoAdapter;
    assert_eq!(adapter.transport_type(), "TCP Socket");
}

/// Test that Go adapter doesn't require workarounds (unlike Ruby)
#[test]
fn test_go_no_workarounds_needed() {
    use debugger_mcp::adapters::logging::DebugAdapterLogger;

    let adapter = GoAdapter;
    assert!(!adapter.requires_workaround());
    assert_eq!(adapter.workaround_reason(), None);
}

/// Test Go adapter metadata
#[test]
fn test_go_adapter_metadata() {
    use debugger_mcp::adapters::logging::DebugAdapterLogger;

    let adapter = GoAdapter;
    assert_eq!(adapter.language_name(), "Go");
    assert_eq!(adapter.language_emoji(), "🐹");
    assert_eq!(adapter.command_line(), "dlv dap --listen=127.0.0.1:<PORT>");
}
