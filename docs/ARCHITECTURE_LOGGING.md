# Debugger MCP Two-Tier Logging Architecture

## Date: 2025-10-07

## Overview

A **two-tier logging architecture** that separates:
1. **High-level structure**: WHAT and WHEN to log (language-agnostic)
2. **Low-level implementation**: HOW to log language-specific details (pluggable)

This enables consistent logging across all language adapters while allowing each to provide specific context.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│           High-Level: DebugAdapterLogger Trait              │
│  (Defines WHAT lifecycle events MUST be logged)             │
├─────────────────────────────────────────────────────────────┤
│  • log_selection()         - Adapter chosen                 │
│  • log_initialization()    - Transport setup                │
│  • log_spawn_command()     - Process/server start           │
│  • log_connection()        - Connection established         │
│  • log_workaround()        - Workaround application         │
│  • log_error()             - Error with full context        │
│  • log_shutdown()          - Cleanup                        │
├─────────────────────────────────────────────────────────────┤
│           Metadata Methods (Language-Specific)              │
│  • language_name() → "Python" / "Ruby" / "Node.js"         │
│  • language_emoji() → "🐍" / "💎" / "🟢"                    │
│  • transport_type() → "STDIO" / "TCP Socket"               │
│  • adapter_id() → "debugpy" / "rdbg" / "vscode-js-debug"   │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼────────┐  ┌──────▼───────┐  ┌────────▼────────┐
│ PythonAdapter  │  │ RubyAdapter  │  │ NodeJsAdapter   │
│  (debugpy)     │  │  (rdbg)      │  │ (vscode-js-deb) │
├────────────────┤  ├──────────────┤  ├─────────────────┤
│ • STDIO        │  │ • TCP Socket │  │ • TCP Socket    │
│ • No workaround│  │ • Entry BP   │  │ • Entry BP      │
│ • Simple spawn │  │ • Port alloc │  │ • Multi-session │
└────────────────┘  └──────────────┘  └─────────────────┘
```

## Design Principles

### 1. **Separation of Concerns**
- **High-level**: Lifecycle events (WHEN to log)
- **Low-level**: Language details (WHAT to include)

### 2. **Consistency Guarantees**
- Trait methods ensure ALL adapters log the same events
- Compiler enforces implementation
- No silent adapters (Python's current problem)

### 3. **Extensibility**
- Adding Go/Java = implement the trait
- No changes to logging infrastructure
- Automatic consistency with existing languages

### 4. **Testability**
- Can verify all trait methods are called
- Can test log output format
- Can ensure error context is complete

## High-Level Trait Definition

```rust
/// Trait defining the logging contract for all debug adapters
///
/// This ensures consistent visibility into adapter lifecycle across all languages.
/// Each adapter MUST implement all methods to provide language-specific context.
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
    fn requires_workaround(&self) -> bool { false }

    /// Reason for workaround (if applicable)
    fn workaround_reason(&self) -> Option<&str> { None }

    // ========================================================================
    // Lifecycle Events (Implemented with default behavior)
    // ========================================================================

    /// Log adapter selection (called when language is matched)
    fn log_selection(&self) {
        info!("{} [{}] Adapter selected: {}",
              self.language_emoji(),
              self.language_name().to_uppercase(),
              self.adapter_id());
        info!("   Transport: {}", self.transport_type());
        info!("   Command: {}", self.command_line());

        if self.requires_workaround() {
            info!("   Workaround: {}",
                  self.workaround_reason().unwrap_or("Required"));
        }
    }

    /// Log transport initialization
    fn log_transport_init(&self) {
        info!("📡 [{}] Initializing {} transport",
              self.language_name().to_uppercase(),
              self.transport_type());
    }

    /// Log process spawn (for all adapters)
    fn log_spawn_attempt(&self) {
        info!("🚀 [{}] Spawning adapter process",
              self.language_name().to_uppercase());
        debug!("   Command: {}", self.command_line());
    }

    /// Log successful connection (adapter-specific details via override)
    fn log_connection_success(&self) {
        info!("✅ [{}] Adapter connected and ready",
              self.language_name().to_uppercase());
    }

    /// Log workaround application (only if required)
    fn log_workaround_applied(&self) {
        if self.requires_workaround() {
            info!("🔧 [{}] Applying workaround: {}",
                  self.language_name().to_uppercase(),
                  self.workaround_reason().unwrap_or("Unknown"));
        }
    }

    /// Log adapter shutdown
    fn log_shutdown(&self) {
        info!("🛑 [{}] Shutting down adapter",
              self.language_name().to_uppercase());
    }

    // ========================================================================
    // Error Logging (MUST be overridden for language-specific context)
    // ========================================================================

    /// Log spawn error with full context and troubleshooting
    fn log_spawn_error(&self, error: &dyn std::error::Error);

    /// Log connection error with troubleshooting steps
    fn log_connection_error(&self, error: &dyn std::error::Error);

    /// Log initialization error
    fn log_init_error(&self, error: &dyn std::error::Error);
}
```

## Low-Level Implementations

### Python Adapter

```rust
impl DebugAdapterLogger for PythonAdapter {
    fn language_name(&self) -> &str { "Python" }
    fn language_emoji(&self) -> &str { "🐍" }
    fn transport_type(&self) -> &str { "STDIO" }
    fn adapter_id(&self) -> &str { "debugpy" }

    fn command_line(&self) -> String {
        format!("python {} -m debugpy.adapter",
                PythonAdapter::args().join(" "))
    }

    fn requires_workaround(&self) -> bool { false }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
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

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [PYTHON] Adapter connection failed: {}", error);
        error!("   This shouldn't happen with STDIO transport");
        error!("   The adapter process may have crashed on startup");
        error!("   ");
        error!("   Check adapter stderr for Python exceptions");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [PYTHON] Initialization failed: {}", error);
        error!("   The adapter started but couldn't initialize");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Program path doesn't exist");
        error!("   2. Program has syntax errors");
        error!("   3. Required modules not installed");
    }
}
```

### Ruby Adapter

```rust
impl DebugAdapterLogger for RubyAdapter {
    fn language_name(&self) -> &str { "Ruby" }
    fn language_emoji(&self) -> &str { "💎" }
    fn transport_type(&self) -> &str { "TCP Socket" }
    fn adapter_id(&self) -> &str { "rdbg" }

    fn command_line(&self) -> String {
        // Note: Port is dynamic, show template
        "rdbg --open --port <PORT> <program>".to_string()
    }

    fn requires_workaround(&self) -> bool { true }

    fn workaround_reason(&self) -> Option<&str> {
        Some("rdbg socket mode doesn't honor --stop-at-load flag")
    }

    fn log_connection_success(&self) {
        info!("✅ [RUBY] Connected to rdbg on port {}", self.port);
        debug!("   Socket: localhost:{}", self.port);
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] Failed to spawn rdbg: {}", error);
        error!("   Command: rdbg --open --port {}", self.port);
        error!("   ");
        error!("   Possible causes:");
        error!("   1. debug gem not installed → gem install debug");
        error!("   2. rdbg not in PATH → which rdbg");
        error!("   3. Ruby version < 3.1 → ruby --version");
        error!("   4. Port {} already in use", self.port);
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ gem list debug");
        error!("   Expected: debug (>= 1.0.0)");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] Socket connection failed: {}", error);
        error!("   Port: {}", self.port);
        error!("   Timeout: 2 seconds");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. rdbg process crashed before opening socket");
        error!("   2. Port {} blocked by firewall", self.port);
        error!("   3. Program exited immediately (syntax error)");
        error!("   ");
        error!("   Check if rdbg process is still running:");
        error!("   $ ps aux | grep rdbg");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUBY] DAP initialization failed: {}", error);
        error!("   Socket connected but DAP protocol failed");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Incompatible rdbg version");
        error!("   2. Program has syntax errors");
        error!("   3. Required gems not installed");
    }
}
```

### Node.js Adapter

```rust
impl DebugAdapterLogger for NodeJsAdapter {
    fn language_name(&self) -> &str { "Node.js" }
    fn language_emoji(&self) -> &str { "🟢" }
    fn transport_type(&self) -> &str { "TCP Socket (Multi-Session)" }
    fn adapter_id(&self) -> &str { "vscode-js-debug" }

    fn command_line(&self) -> String {
        format!("node {} --server={}",
                self.dap_server_path,
                self.port)
    }

    fn requires_workaround(&self) -> bool { true }

    fn workaround_reason(&self) -> Option<&str> {
        Some("vscode-js-debug uses parent-child session architecture")
    }

    fn log_connection_success(&self) {
        info!("✅ [NODEJS] Connected to vscode-js-debug on port {}", self.port);
        debug!("   DAP server: {}", self.dap_server_path);
        debug!("   Architecture: Parent session (child sessions spawned dynamically)");
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] Failed to spawn vscode-js-debug: {}", error);
        error!("   Command: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. vscode-js-debug not installed → npm install -g vscode-js-debug");
        error!("   2. DAP server path incorrect: {}", self.dap_server_path);
        error!("   3. Node.js not in PATH → which node");
        error!("   4. Port {} already in use", self.port);
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ node {} --version", self.dap_server_path);
        error!("   Expected: vscode-js-debug DAP server");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] Socket connection failed: {}", error);
        error!("   Port: {}", self.port);
        error!("   Timeout: 2 seconds");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. vscode-js-debug crashed on startup");
        error!("   2. Port {} blocked by firewall", self.port);
        error!("   3. DAP server not listening on --server flag");
        error!("   ");
        error!("   Verify DAP server is installed:");
        error!("   $ ls {}", self.dap_server_path);
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [NODEJS] DAP initialization failed: {}", error);
        error!("   Socket connected but DAP protocol failed");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Incompatible vscode-js-debug version");
        error!("   2. Multi-session handshake failed");
        error!("   3. Program path doesn't exist or has errors");
    }
}
```

## Integration with Manager

The manager uses the trait methods during adapter lifecycle:

```rust
impl SessionManager {
    pub async fn start_session(&mut self, language: &str, ...) -> Result<String> {
        // Select adapter and log selection
        let adapter: Box<dyn DebugAdapterLogger> = match language {
            "python" => {
                let adapter = PythonAdapter;
                adapter.log_selection();
                Box::new(adapter)
            }
            "ruby" => {
                let adapter = RubyAdapter { port: 0 }; // Set during spawn
                adapter.log_selection();
                Box::new(adapter)
            }
            "nodejs" => {
                let adapter = NodeJsAdapter { port: 0, ... };
                adapter.log_selection();
                Box::new(adapter)
            }
            _ => return Err(Error::UnsupportedLanguage(language.to_string())),
        };

        // Log transport initialization
        adapter.log_transport_init();

        // Spawn adapter
        adapter.log_spawn_attempt();
        match spawn_adapter(&adapter).await {
            Ok(process) => {
                adapter.log_connection_success();
            }
            Err(e) => {
                adapter.log_spawn_error(&e);
                return Err(e);
            }
        }

        // Apply workarounds if needed
        if adapter.requires_workaround() {
            adapter.log_workaround_applied();
            apply_workaround(&adapter).await?;
        }

        // Continue with DAP initialization...
    }
}
```

## Benefits of This Architecture

### 1. **Consistency Guarantees**
- **Before**: Python has 0 logs, Ruby has 1, Node.js has 3
- **After**: All adapters log ALL lifecycle events (enforced by trait)

### 2. **Extensibility**
Adding Go is just:
```rust
impl DebugAdapterLogger for GoAdapter {
    fn language_name(&self) -> &str { "Go" }
    fn language_emoji(&self) -> &str { "🔷" }
    // ... implement other methods
}
```

### 3. **Testability**
```rust
#[test]
fn test_all_adapters_implement_logging() {
    let adapters: Vec<Box<dyn DebugAdapterLogger>> = vec![
        Box::new(PythonAdapter),
        Box::new(RubyAdapter { port: 0 }),
        Box::new(NodeJsAdapter { ... }),
    ];

    for adapter in adapters {
        // Verify all methods work
        assert!(!adapter.language_name().is_empty());
        assert!(!adapter.language_emoji().is_empty());
        // ... etc
    }
}
```

### 4. **Documentation**
The trait itself documents what MUST be logged:
```rust
/// Every adapter MUST log:
/// 1. Selection (when chosen)
/// 2. Transport initialization
/// 3. Spawn attempt
/// 4. Connection success/failure
/// 5. Workaround application (if needed)
/// 6. Errors with full context
```

## Implementation Plan

### Phase 1: Define Trait (15 minutes)
- Create `src/adapters/logging.rs`
- Define `DebugAdapterLogger` trait
- Export from `src/adapters/mod.rs`

### Phase 2: Implement for Python (10 minutes)
- Add trait impl to `src/adapters/python.rs`
- Provide error context (spawn, connection, init)
- Test with Python debugging session

### Phase 3: Implement for Ruby (10 minutes)
- Add trait impl to `src/adapters/ruby.rs`
- Add socket-specific error context
- Include port information in logs

### Phase 4: Implement for Node.js (10 minutes)
- Add trait impl to `src/adapters/nodejs.rs`
- Add multi-session context
- Include DAP server path in errors

### Phase 5: Update Manager (10 minutes)
- Use trait methods in `start_session()`
- Remove ad-hoc logging
- Ensure all lifecycle events logged

### Phase 6: Testing (10 minutes)
- Run all three languages with `RUST_LOG=debug`
- Verify consistent log format
- Verify all errors include context

**Total**: ~65 minutes

## Success Criteria

After implementation, running:
```bash
RUST_LOG=debugger_mcp=debug cargo test --test test_all_languages -- --nocapture
```

Should show:
```
🐍 [PYTHON] Adapter selected: debugpy
   Transport: STDIO
   Command: python -Xfrozen_modules=off -m debugpy.adapter
📡 [PYTHON] Initializing STDIO transport
🚀 [PYTHON] Spawning adapter process
✅ [PYTHON] Adapter connected and ready

💎 [RUBY] Adapter selected: rdbg
   Transport: TCP Socket
   Command: rdbg --open --port <PORT> <program>
   Workaround: rdbg socket mode doesn't honor --stop-at-load flag
📡 [RUBY] Initializing TCP Socket transport
🚀 [RUBY] Spawning adapter process
✅ [RUBY] Connected to rdbg on port 54321
🔧 [RUBY] Applying workaround: rdbg socket mode doesn't honor --stop-at-load flag

🟢 [NODEJS] Adapter selected: vscode-js-debug
   Transport: TCP Socket (Multi-Session)
   Command: node /path/to/dapDebugServer.js --server=12345
   Workaround: vscode-js-debug uses parent-child session architecture
📡 [NODEJS] Initializing TCP Socket (Multi-Session) transport
🚀 [NODEJS] Spawning adapter process
✅ [NODEJS] Connected to vscode-js-debug on port 12345
🔧 [NODEJS] Applying workaround: vscode-js-debug uses parent-child session architecture
```

## Next Steps

After logging architecture is in place:
1. ✅ **Consistent visibility** across all languages
2. ✅ **Easy to debug** adapter-specific issues
3. ✅ **Ready for multi-session** implementation with full logging
4. ✅ **Future-proof** for Go, Java, etc.
