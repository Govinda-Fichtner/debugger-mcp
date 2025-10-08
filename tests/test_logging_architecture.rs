/// Test to verify the two-tier logging architecture works correctly
/// across all language adapters (Python, Ruby, Node.js)

use debugger_mcp::adapters::python::PythonAdapter;
use debugger_mcp::adapters::ruby::RubyAdapter;
use debugger_mcp::adapters::nodejs::NodeJsAdapter;
use debugger_mcp::adapters::logging::DebugAdapterLogger;

#[test]
fn test_python_adapter_logging_metadata() {
    let adapter = PythonAdapter;

    // Verify metadata
    assert_eq!(adapter.language_name(), "Python");
    assert_eq!(adapter.language_emoji(), "🐍");
    assert_eq!(adapter.transport_type(), "STDIO");
    assert_eq!(adapter.adapter_id(), "debugpy");
    assert!(!adapter.requires_workaround());
    assert!(adapter.workaround_reason().is_none());

    // Verify command line
    let cmd = adapter.command_line();
    assert!(cmd.contains("python"));
    assert!(cmd.contains("debugpy.adapter"));
}

#[test]
fn test_ruby_adapter_logging_metadata() {
    let adapter = RubyAdapter;

    // Verify metadata
    assert_eq!(adapter.language_name(), "Ruby");
    assert_eq!(adapter.language_emoji(), "💎");
    assert_eq!(adapter.transport_type(), "TCP Socket");
    assert_eq!(adapter.adapter_id(), "rdbg");
    assert!(adapter.requires_workaround());
    assert_eq!(
        adapter.workaround_reason(),
        Some("rdbg socket mode doesn't honor --stop-at-load flag")
    );

    // Verify command line template
    let cmd = adapter.command_line();
    assert!(cmd.contains("rdbg"));
    assert!(cmd.contains("--open"));
    assert!(cmd.contains("--port"));
}

#[test]
fn test_nodejs_adapter_logging_metadata() {
    let adapter = NodeJsAdapter;

    // Verify metadata
    assert_eq!(adapter.language_name(), "Node.js");
    assert_eq!(adapter.language_emoji(), "🟢");
    assert_eq!(adapter.transport_type(), "TCP Socket (Multi-Session)");
    assert_eq!(adapter.adapter_id(), "vscode-js-debug");
    assert!(adapter.requires_workaround());
    assert_eq!(
        adapter.workaround_reason(),
        Some("vscode-js-debug uses parent-child session architecture - parent doesn't send stopped events")
    );

    // Verify command line template
    let cmd = adapter.command_line();
    assert!(cmd.contains("node"));
    assert!(cmd.contains("dapDebugServer"));
}

#[test]
fn test_all_adapters_implement_trait() {
    // This test ensures all adapters implement DebugAdapterLogger
    // and can be used polymorphically

    let adapters: Vec<Box<dyn DebugAdapterLogger>> = vec![
        Box::new(PythonAdapter),
        Box::new(RubyAdapter),
        Box::new(NodeJsAdapter),
    ];

    for adapter in adapters {
        // All adapters must provide these
        assert!(!adapter.language_name().is_empty());
        assert!(!adapter.language_emoji().is_empty());
        assert!(!adapter.transport_type().is_empty());
        assert!(!adapter.adapter_id().is_empty());
        assert!(!adapter.command_line().is_empty());

        // Workarounds are optional
        if adapter.requires_workaround() {
            assert!(adapter.workaround_reason().is_some());
        }
    }
}

#[test]
fn test_logging_methods_dont_panic() {
    // Initialize tracing subscriber for this test
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();

    // Test Python
    let python = PythonAdapter;
    python.log_selection();
    python.log_transport_init();
    python.log_spawn_attempt();
    python.log_connection_success();
    python.log_workaround_applied(); // Should do nothing (no workaround)
    python.log_shutdown();

    // Test Ruby
    let ruby = RubyAdapter;
    ruby.log_selection();
    ruby.log_transport_init();
    ruby.log_spawn_attempt();
    ruby.log_connection_success();
    ruby.log_workaround_applied(); // Should log workaround message
    ruby.log_shutdown();

    // Test Node.js
    let nodejs = NodeJsAdapter;
    nodejs.log_selection();
    nodejs.log_transport_init();
    nodejs.log_spawn_attempt();
    nodejs.log_connection_success();
    nodejs.log_workaround_applied(); // Should log workaround message
    nodejs.log_shutdown();
}

#[test]
fn test_error_logging_methods_dont_panic() {
    // Initialize tracing subscriber for this test
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();

    // Create a mock error
    let error = std::io::Error::new(std::io::ErrorKind::NotFound, "test error");

    // Test Python error logging
    let python = PythonAdapter;
    python.log_spawn_error(&error);
    python.log_connection_error(&error);
    python.log_init_error(&error);

    // Test Ruby error logging
    let ruby = RubyAdapter;
    ruby.log_spawn_error(&error);
    ruby.log_connection_error(&error);
    ruby.log_init_error(&error);

    // Test Node.js error logging
    let nodejs = NodeJsAdapter;
    nodejs.log_spawn_error(&error);
    nodejs.log_connection_error(&error);
    nodejs.log_init_error(&error);
}

/// Documentation test: Show expected log output format
///
/// When running with RUST_LOG=debugger_mcp=info, you should see:
///
/// ```text
/// 🐍 [PYTHON] Adapter selected: debugpy
///    Transport: STDIO
///    Command: python -Xfrozen_modules=off -m debugpy.adapter
/// 📡 [PYTHON] Initializing STDIO transport
/// 🚀 [PYTHON] Spawning adapter process
/// ✅ [PYTHON] Adapter connected and ready
///
/// 💎 [RUBY] Adapter selected: rdbg
///    Transport: TCP Socket
///    Command: rdbg --open --port <PORT> [--stop-at-load|--nonstop] <program> [args...]
///    Workaround: rdbg socket mode doesn't honor --stop-at-load flag
/// 📡 [RUBY] Initializing TCP Socket transport
/// 🚀 [RUBY] Spawning adapter process
/// ✅ [RUBY] Connected to rdbg on port 12345
/// 🔧 [RUBY] Applying workaround: rdbg socket mode doesn't honor --stop-at-load flag
///
/// 🟢 [NODEJS] Adapter selected: vscode-js-debug
///    Transport: TCP Socket (Multi-Session)
///    Command: node <dapDebugServer.js> --server=<PORT>
///    Workaround: vscode-js-debug uses parent-child session architecture - parent doesn't send stopped events
/// 📡 [NODEJS] Initializing TCP Socket (Multi-Session) transport
/// 🚀 [NODEJS] Spawning adapter process
/// ✅ [NODEJS] Connected to vscode-js-debug on port 54321
/// 🔧 [NODEJS] Applying workaround: vscode-js-debug uses parent-child session architecture - parent doesn't send stopped events
/// ```
#[test]
fn example_expected_log_format() {
    // This test documents the expected log format
    // Run with: RUST_LOG=debugger_mcp=info cargo test example_expected_log_format -- --nocapture
}
