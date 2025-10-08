use std::sync::{Arc, Mutex};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Captured log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    pub message: String,
    #[allow(dead_code)]
    pub target: String,
}

/// Custom layer that captures logs for validation
pub struct LogCaptureLayer {
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl LogCaptureLayer {
    pub fn new(logs: Arc<Mutex<Vec<LogEntry>>>) -> Self {
        Self { logs }
    }
}

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            level: *metadata.level(),
            message: visitor.message,
            target: metadata.target().to_string(),
        };

        self.logs.lock().unwrap().push(entry);
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove quotes from debug format
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }
}

/// Log validator for integration tests
pub struct LogValidator {
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl LogValidator {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the capture layer for the subscriber
    pub fn layer(&self) -> LogCaptureLayer {
        LogCaptureLayer::new(Arc::clone(&self.logs))
    }

    /// Get all captured logs
    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.lock().unwrap().clone()
    }

    /// Validate that expected log patterns are present
    pub fn validate(&self) -> ValidationResult {
        let logs = self.get_logs();
        let mut result = ValidationResult::new();

        // Define expected log patterns for a complete debugging session
        let expected_patterns = vec![
            // Initialization
            ("Spawning DAP client", "DAP client spawn"),
            ("Sending initialize request", "Initialize request"),
            (
                "send_request: Sending 'initialize'",
                "Initialize send_request",
            ),
            ("message_writer: Task started", "Writer task started"),
            ("message_writer: Lock acquired", "Writer acquires lock"),
            // Events
            ("EVENT RECEIVED: 'initialized'", "Initialized event"),
            ("EVENT RECEIVED: 'stopped'", "Stopped event"),
            (
                "Received 'initialized' event - signaling",
                "Initialized callback",
            ),
            // Configuration
            ("Sending configurationDone", "ConfigurationDone request"),
            (
                "send_request: Received response for 'configurationDone'",
                "ConfigurationDone response",
            ),
            // Breakpoints
            ("set_breakpoints: Starting", "Breakpoint operation start"),
            (
                "set_breakpoints: Sending setBreakpoints request",
                "Breakpoint request send",
            ),
            (
                "send_request: Sending 'setBreakpoints'",
                "setBreakpoints send_request",
            ),
            (
                "send_request: Received response for 'setBreakpoints'",
                "setBreakpoints response",
            ),
            ("set_breakpoints: Success", "Breakpoint success"),
            ("verified=true", "Breakpoint verified"),
            // Execution
            ("send_request: Sending 'continue'", "Continue request"),
            (
                "send_request: Received response for 'continue'",
                "Continue response",
            ),
            // Stack trace
            ("send_request: Sending 'stackTrace'", "StackTrace request"),
            // Cleanup
            ("send_request: Sending 'disconnect'", "Disconnect request"),
            // Note: "message_writer: Task exiting" is optional - may be logged after validation
        ];

        for (pattern, description) in expected_patterns {
            if !logs.iter().any(|log| log.message.contains(pattern)) {
                result
                    .missing_logs
                    .push(format!("{}: '{}'", description, pattern));
            } else {
                result.found_logs.push(description.to_string());
            }
        }

        // Validate log quality
        self.validate_quality(&logs, &mut result);

        result
    }

    /// Validate log quality (proper formatting, emoji usage, etc.)
    fn validate_quality(&self, logs: &[LogEntry], result: &mut ValidationResult) {
        let mut issues = Vec::new();

        for log in logs {
            // Check for proper emoji usage in key operations
            if log.message.contains("message_reader") && !log.message.contains("📖") {
                issues.push(format!(
                    "Message reader log missing 📖 emoji: {}",
                    log.message
                ));
            }
            if log.message.contains("message_writer") && !log.message.contains("📝") {
                issues.push(format!(
                    "Message writer log missing 📝 emoji: {}",
                    log.message
                ));
            }
            if log.message.contains("EVENT RECEIVED") && !log.message.contains("🎯") {
                issues.push(format!("Event log missing 🎯 emoji: {}", log.message));
            }
            if log.message.contains("set_breakpoints: Starting") && !log.message.contains("🔧") {
                issues.push(format!(
                    "Breakpoint start log missing 🔧 emoji: {}",
                    log.message
                ));
            }
            if log.message.contains("set_breakpoints: Success") && !log.message.contains("✅") {
                issues.push(format!("Success log missing ✅ emoji: {}", log.message));
            }
            if log.message.contains("send_request: Sending") && !log.message.contains("✉️") {
                issues.push(format!("Request log missing ✉️ emoji: {}", log.message));
            }

            // Check log level appropriateness
            if (log.message.contains("Failed") || log.message.contains("failed"))
                && log.level != Level::ERROR
                && log.level != Level::WARN
            {
                issues.push(format!(
                    "Failure message should be ERROR or WARN: {}",
                    log.message
                ));
            }
        }

        result.quality_issues = issues;
    }

    /// Print a summary of validation results
    pub fn print_summary(&self, result: &ValidationResult) {
        println!("\n📊 Log Validation Summary");
        println!("═══════════════════════════════════════════════");
        println!("✅ Found {} expected log patterns", result.found_logs.len());
        println!(
            "❌ Missing {} expected log patterns",
            result.missing_logs.len()
        );
        println!("⚠️  Quality issues: {}", result.quality_issues.len());
        println!(
            "📝 Total logs captured: {}",
            self.logs.lock().unwrap().len()
        );

        if !result.missing_logs.is_empty() {
            println!("\n❌ Missing Logs:");
            for missing in &result.missing_logs {
                println!("  - {}", missing);
            }
        }

        if !result.quality_issues.is_empty() {
            println!("\n⚠️  Quality Issues:");
            for issue in &result.quality_issues {
                println!("  - {}", issue);
            }
        }

        if result.is_valid() {
            println!("\n🎉 All validation checks passed!");
        } else {
            println!("\n⚠️  Validation found issues that need attention");
        }
    }

    /// Get statistics about log levels
    pub fn get_stats(&self) -> LogStats {
        let logs = self.get_logs();
        LogStats {
            total: logs.len(),
            error: logs.iter().filter(|l| l.level == Level::ERROR).count(),
            warn: logs.iter().filter(|l| l.level == Level::WARN).count(),
            info: logs.iter().filter(|l| l.level == Level::INFO).count(),
            debug: logs.iter().filter(|l| l.level == Level::DEBUG).count(),
            trace: logs.iter().filter(|l| l.level == Level::TRACE).count(),
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub found_logs: Vec<String>,
    pub missing_logs: Vec<String>,
    pub quality_issues: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            found_logs: Vec::new(),
            missing_logs: Vec::new(),
            quality_issues: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.missing_logs.is_empty() && self.quality_issues.is_empty()
    }
}

#[derive(Debug)]
pub struct LogStats {
    pub total: usize,
    pub error: usize,
    pub warn: usize,
    pub info: usize,
    pub debug: usize,
    pub trace: usize,
}
