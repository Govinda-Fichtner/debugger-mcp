use std::error::Error;
use tracing::{debug, info};

/// Trait defining the logging contract for all debug adapters
///
/// This ensures consistent visibility into adapter lifecycle across all languages.
/// Each adapter MUST implement all methods to provide language-specific context.
///
/// # Architecture
///
/// Two-tier logging:
/// 1. **High-level**: WHAT and WHEN to log (defined by trait methods)
/// 2. **Low-level**: HOW to provide language-specific context (implemented by adapters)
///
/// # Lifecycle Events
///
/// Every adapter logs these events in order:
/// 1. Selection (`log_selection`) - Adapter chosen for language
/// 2. Transport Init (`log_transport_init`) - STDIO vs Socket setup
/// 3. Spawn (`log_spawn_attempt`) - Process/server starting
/// 4. Connection (`log_connection_success`) - Ready for DAP
/// 5. Workaround (`log_workaround_applied`) - If needed
/// 6. Shutdown (`log_shutdown`) - Cleanup
///
/// Errors logged via: `log_spawn_error`, `log_connection_error`, `log_init_error`
pub trait DebugAdapterLogger {
    // ========================================================================
    // Metadata (Language-Specific Constants)
    // ========================================================================

    /// Full language name: "Python", "Ruby", "Node.js", "Go", "Java"
    fn language_name(&self) -> &str;

    /// Emoji for visual identification: "🐍", "💎", "🟢", "🔷", "☕"
    fn language_emoji(&self) -> &str;

    /// Transport mechanism: "STDIO", "TCP Socket", "Named Pipe"
    fn transport_type(&self) -> &str;

    /// Adapter identifier: "debugpy", "rdbg", "vscode-js-debug", "delve"
    fn adapter_id(&self) -> &str;

    /// Full command line that will be executed
    fn command_line(&self) -> String;

    /// Whether this adapter requires workarounds
    fn requires_workaround(&self) -> bool {
        false
    }

    /// Reason for workaround (if applicable)
    fn workaround_reason(&self) -> Option<&str> {
        None
    }

    // ========================================================================
    // Lifecycle Events (Default implementations with consistent format)
    // ========================================================================

    /// Log adapter selection (called when language is matched)
    ///
    /// Default format:
    /// ```text
    /// 🐍 [PYTHON] Adapter selected: debugpy
    ///    Transport: STDIO
    ///    Command: python -m debugpy.adapter
    ///    Workaround: <reason> (if applicable)
    /// ```
    fn log_selection(&self) {
        info!(
            "{} [{}] Adapter selected: {}",
            self.language_emoji(),
            self.language_name().to_uppercase(),
            self.adapter_id()
        );
        info!("   Transport: {}", self.transport_type());
        info!("   Command: {}", self.command_line());

        if self.requires_workaround() {
            info!(
                "   Workaround: {}",
                self.workaround_reason().unwrap_or("Required")
            );
        }
    }

    /// Log transport initialization
    ///
    /// Default format:
    /// ```text
    /// 📡 [PYTHON] Initializing STDIO transport
    /// ```
    fn log_transport_init(&self) {
        info!(
            "📡 [{}] Initializing {} transport",
            self.language_name().to_uppercase(),
            self.transport_type()
        );
    }

    /// Log process spawn attempt
    ///
    /// Default format:
    /// ```text
    /// 🚀 [PYTHON] Spawning adapter process
    ///    Command: python -m debugpy.adapter
    /// ```
    fn log_spawn_attempt(&self) {
        info!(
            "🚀 [{}] Spawning adapter process",
            self.language_name().to_uppercase()
        );
        debug!("   Command: {}", self.command_line());
    }

    /// Log successful connection (can be overridden for adapter-specific details)
    ///
    /// Default format:
    /// ```text
    /// ✅ [PYTHON] Adapter connected and ready
    /// ```
    fn log_connection_success(&self) {
        info!(
            "✅ [{}] Adapter connected and ready",
            self.language_name().to_uppercase()
        );
    }

    /// Log workaround application (only if required)
    ///
    /// Default format:
    /// ```text
    /// 🔧 [RUBY] Applying workaround: rdbg socket mode doesn't honor --stop-at-load
    /// ```
    fn log_workaround_applied(&self) {
        if self.requires_workaround() {
            info!(
                "🔧 [{}] Applying workaround: {}",
                self.language_name().to_uppercase(),
                self.workaround_reason().unwrap_or("Unknown")
            );
        }
    }

    /// Log adapter shutdown
    ///
    /// Default format:
    /// ```text
    /// 🛑 [PYTHON] Shutting down adapter
    /// ```
    fn log_shutdown(&self) {
        info!(
            "🛑 [{}] Shutting down adapter",
            self.language_name().to_uppercase()
        );
    }

    // ========================================================================
    // Error Logging (MUST be implemented for language-specific context)
    // ========================================================================

    /// Log spawn error with full context and troubleshooting steps
    ///
    /// Implementation MUST include:
    /// - Error message
    /// - Full command that failed
    /// - Possible causes (numbered list)
    /// - Troubleshooting steps
    ///
    /// Example:
    /// ```text
    /// ❌ [PYTHON] Failed to spawn debugpy adapter: No such file or directory
    ///    Command: python -m debugpy.adapter
    ///
    ///    Possible causes:
    ///    1. debugpy not installed → pip install debugpy
    ///    2. python not in PATH → which python
    ///
    ///    Troubleshooting:
    ///    $ python -c 'import debugpy; print(debugpy.__version__)'
    /// ```
    fn log_spawn_error(&self, error: &dyn Error);

    /// Log connection error with troubleshooting steps
    ///
    /// Implementation MUST include:
    /// - Error message
    /// - Connection details (port for sockets)
    /// - Possible causes
    /// - Troubleshooting steps
    fn log_connection_error(&self, error: &dyn Error);

    /// Log initialization error (DAP protocol handshake failure)
    ///
    /// Implementation MUST include:
    /// - Error message
    /// - What succeeded (spawn, connection)
    /// - What failed (initialize, launch)
    /// - Possible causes
    fn log_init_error(&self, error: &dyn Error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::error;

    /// Mock adapter for testing trait default implementations
    struct MockAdapter;

    impl DebugAdapterLogger for MockAdapter {
        fn language_name(&self) -> &str {
            "Test"
        }
        fn language_emoji(&self) -> &str {
            "🧪"
        }
        fn transport_type(&self) -> &str {
            "Mock Transport"
        }
        fn adapter_id(&self) -> &str {
            "mock-adapter"
        }
        fn command_line(&self) -> String {
            "mock-command arg1 arg2".to_string()
        }

        fn log_spawn_error(&self, _error: &dyn Error) {
            error!("Mock spawn error");
        }
        fn log_connection_error(&self, _error: &dyn Error) {
            error!("Mock connection error");
        }
        fn log_init_error(&self, _error: &dyn Error) {
            error!("Mock init error");
        }
    }

    #[test]
    fn test_metadata_methods() {
        let adapter = MockAdapter;
        assert_eq!(adapter.language_name(), "Test");
        assert_eq!(adapter.language_emoji(), "🧪");
        assert_eq!(adapter.transport_type(), "Mock Transport");
        assert_eq!(adapter.adapter_id(), "mock-adapter");
        assert_eq!(adapter.command_line(), "mock-command arg1 arg2");
    }

    #[test]
    fn test_default_no_workaround() {
        let adapter = MockAdapter;
        assert!(!adapter.requires_workaround());
        assert!(adapter.workaround_reason().is_none());
    }

    #[test]
    fn test_lifecycle_methods_dont_panic() {
        let adapter = MockAdapter;

        // These should not panic
        adapter.log_selection();
        adapter.log_transport_init();
        adapter.log_spawn_attempt();
        adapter.log_connection_success();
        adapter.log_workaround_applied();
        adapter.log_shutdown();
    }
}
