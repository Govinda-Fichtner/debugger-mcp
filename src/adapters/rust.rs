//! Rust Debug Adapter (CodeLLDB)
//!
//! # Overview
//!
//! Rust debugging uses CodeLLDB (vadimcn.vscode-lldb), an LLDB-based debug adapter.
//! Unlike Python/Ruby/Node.js which debug source files directly, Rust requires
//! compilation before debugging.
//!
//! # Architecture
//!
//! ```
//! User provides: /workspace/fizzbuzz.rs
//!      ↓ Compile with rustc
//! Binary created: /workspace/target/debug/fizzbuzz
//!      ↓ Spawn CodeLLDB via STDIO
//! Debug session: CodeLLDB ← STDIO → MCP Server
//! ```
//!
//! # Transport
//!
//! **STDIO** (like Python, not socket like Ruby/Node.js)
//! - CodeLLDB supports STDIO since v1.11.0
//! - Command: `codelldb --port 0` (port 0 = STDIO mode)
//! - Simple, reliable, no port allocation needed
//!
//! # Compilation Strategy
//!
//! **Phase 1: Single-file support**
//! - Input: `/workspace/fizzbuzz.rs`
//! - Compile: `rustc -g fizzbuzz.rs -o target/debug/fizzbuzz`
//! - Output: `/workspace/target/debug/fizzbuzz`
//!
//! **Phase 2: Cargo project support** (future)
//! - Detect Cargo.toml
//! - Run: `cargo build`
//! - Parse metadata for binary path
//!
//! # Key Differences from Other Languages
//!
//! | Aspect | Python/Ruby/Node.js | Rust |
//! |--------|---------------------|------|
//! | Input | Source file | Source file |
//! | Compilation | No | **Yes** |
//! | Debug target | Source file | **Compiled binary** |
//! | Program path | `/workspace/app.py` | `/workspace/target/debug/app` |
//!
//! # See Also
//!
//! - `docs/RUST_DEBUGGING_RESEARCH_AND_PROPOSAL.md` - Architecture and research
//! - https://github.com/vadimcn/codelldb - CodeLLDB debugger

use super::logging::DebugAdapterLogger;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, error, info};

/// Rust CodeLLDB adapter configuration
pub struct RustAdapter;

/// Rust project type detection result
#[derive(Debug, Clone, PartialEq)]
pub enum RustProjectType {
    /// Single Rust source file (e.g., fizzbuzz.rs)
    SingleFile(PathBuf),
    /// Cargo project with manifest
    CargoProject {
        /// Cargo.toml root directory
        root: PathBuf,
        /// Path to Cargo.toml
        manifest: PathBuf,
    },
}

/// Cargo target type (binary, test, example)
#[derive(Debug, Clone, PartialEq)]
pub enum CargoTargetType {
    /// Binary executable (from [[bin]] or src/main.rs)
    Binary,
    /// Test binary (from `cargo test --no-run`)
    Test,
    /// Example binary (from examples/)
    Example(String),
}

impl RustAdapter {
    /// Get CodeLLDB command path
    ///
    /// Checks multiple locations in order:
    /// 1. /usr/local/lib/codelldb/adapter/codelldb (Docker container - new location)
    /// 2. /usr/local/bin/codelldb (Docker container - old location)
    /// 3. /usr/bin/codelldb (system install)
    /// 4. codelldb (in PATH)
    pub fn command() -> String {
        let locations = vec![
            "/usr/local/lib/codelldb/adapter/codelldb",
            "/usr/local/bin/codelldb",
            "/usr/bin/codelldb",
        ];

        for location in locations {
            if Path::new(location).exists() {
                return location.to_string();
            }
        }

        // Fall back to PATH
        "codelldb".to_string()
    }

    /// Get CodeLLDB args for STDIO mode
    ///
    /// Returns: [] (empty)
    /// CodeLLDB 1.11.0+ uses STDIO by default when run without --port argument.
    /// --port is only for TCP mode. When stdio pipes are provided (via DapClient::spawn),
    /// CodeLLDB automatically detects and uses STDIO for DAP communication.
    pub fn args() -> Vec<String> {
        vec![] // Empty = STDIO mode (default for v1.11.0+)
    }

    /// Adapter ID for CodeLLDB
    pub fn adapter_id() -> &'static str {
        "codelldb"
    }

    /// Detect project type from source file path
    ///
    /// Walks up directory tree from source file to find Cargo.toml.
    /// If found, returns CargoProject. Otherwise, returns SingleFile.
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to .rs source file
    ///
    /// # Returns
    ///
    /// RustProjectType indicating single file or Cargo project
    ///
    /// # Example
    ///
    /// ```rust
    /// // Source file in Cargo project
    /// let project = RustAdapter::detect_project_type("/workspace/cargo-simple/src/main.rs")?;
    /// // Returns: CargoProject { root: "/workspace/cargo-simple", manifest: "/workspace/cargo-simple/Cargo.toml" }
    ///
    /// // Single file not in Cargo project
    /// let project = RustAdapter::detect_project_type("/workspace/fizzbuzz.rs")?;
    /// // Returns: SingleFile("/workspace/fizzbuzz.rs")
    /// ```
    pub fn detect_project_type(source_path: &str) -> Result<RustProjectType> {
        let source = PathBuf::from(source_path);

        // Validate source file exists
        if !source.exists() {
            return Err(Error::Compilation(format!(
                "Source file not found: {}",
                source_path
            )));
        }

        debug!("🔍 [RUST] Detecting project type for: {}", source_path);

        // Walk up directory tree to find Cargo.toml
        let mut current = source.parent();
        while let Some(dir) = current {
            let manifest = dir.join("Cargo.toml");
            if manifest.exists() {
                info!("📦 [RUST] Found Cargo project: {}", dir.display());
                info!("📦 [RUST] Manifest: {}", manifest.display());
                return Ok(RustProjectType::CargoProject {
                    root: dir.to_path_buf(),
                    manifest,
                });
            }
            current = dir.parent();
        }

        // No Cargo.toml found - single file
        info!("📄 [RUST] Single file project: {}", source_path);
        Ok(RustProjectType::SingleFile(source))
    }

    /// Parse Cargo JSON output to find executable path
    ///
    /// Cargo with `--message-format=json` emits JSON lines for each build artifact.
    /// This function parses those lines to find the executable binary.
    ///
    /// # Arguments
    ///
    /// * `json_output` - Cargo JSON output (one JSON object per line)
    /// * `target_type` - Type of target to find (Binary, Test, Example)
    ///
    /// # Returns
    ///
    /// Path to executable binary
    ///
    /// # Example JSON Output
    ///
    /// ```json
    /// {"reason":"compiler-artifact","target":{"kind":["bin"],"name":"cargo-simple"},"executable":"/workspace/cargo-simple/target/debug/cargo-simple","fresh":false}
    /// {"reason":"compiler-artifact","target":{"kind":["test"],"name":"integration_test"},"executable":"/workspace/cargo-simple/target/debug/deps/integration_test-abc123","fresh":false}
    /// ```
    pub fn parse_cargo_executable(
        json_output: &str,
        target_type: &CargoTargetType,
    ) -> Result<String> {
        debug!("🔍 [RUST] Parsing Cargo JSON for {:?} target", target_type);

        for line in json_output.lines() {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON line
            let artifact: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue, // Skip non-JSON lines (warnings, etc.)
            };

            // Only process compiler-artifact messages
            if artifact["reason"] != "compiler-artifact" {
                continue;
            }

            // Check if executable field exists
            let Some(executable) = artifact["executable"].as_str() else {
                continue;
            };

            // Get target kind
            let Some(kinds) = artifact["target"]["kind"].as_array() else {
                continue;
            };

            // Match target type
            let matches = match target_type {
                CargoTargetType::Binary => {
                    // Regular binary (not test mode)
                    let is_bin = kinds.iter().any(|k| k == "bin");
                    let is_test_mode = artifact["profile"]["test"].as_bool().unwrap_or(false);
                    is_bin && !is_test_mode
                }
                CargoTargetType::Test => {
                    // Test binary - check profile.test field
                    // cargo test --no-run builds with kind=["bin"] but profile.test=true
                    artifact["profile"]["test"].as_bool().unwrap_or(false)
                }
                CargoTargetType::Example(name) => {
                    if !kinds.iter().any(|k| k == "example") {
                        false
                    } else {
                        // Check example name matches
                        artifact["target"]["name"].as_str() == Some(name)
                    }
                }
            };

            if matches {
                info!("✅ [RUST] Found executable: {}", executable);
                return Ok(executable.to_string());
            }
        }

        Err(Error::Compilation(format!(
            "No executable found for target type: {:?}",
            target_type
        )))
    }

    /// Compile Cargo project
    ///
    /// Runs `cargo build` with JSON output and parses the executable path.
    /// Supports binaries, tests, and examples.
    ///
    /// # Arguments
    ///
    /// * `cargo_root` - Path to Cargo project root (directory containing Cargo.toml)
    /// * `target_type` - Type of target to build
    /// * `release` - If true, compile with optimizations
    ///
    /// # Returns
    ///
    /// Path to compiled executable binary
    ///
    /// # Example
    ///
    /// ```rust
    /// // Build binary
    /// let binary = RustAdapter::compile_cargo_project(
    ///     "/workspace/cargo-simple",
    ///     &CargoTargetType::Binary,
    ///     false
    /// ).await?;
    ///
    /// // Build test
    /// let test_binary = RustAdapter::compile_cargo_project(
    ///     "/workspace/cargo-simple",
    ///     &CargoTargetType::Test,
    ///     false
    /// ).await?;
    ///
    /// // Build example
    /// let example = RustAdapter::compile_cargo_project(
    ///     "/workspace/cargo-example",
    ///     &CargoTargetType::Example("demo".to_string()),
    ///     false
    /// ).await?;
    /// ```
    pub async fn compile_cargo_project(
        cargo_root: &str,
        target_type: &CargoTargetType,
        release: bool,
    ) -> Result<String> {
        let cargo_root_path = Path::new(cargo_root);

        // Validate Cargo root exists
        if !cargo_root_path.exists() {
            return Err(Error::Compilation(format!(
                "Cargo root not found: {}",
                cargo_root
            )));
        }

        // Validate Cargo.toml exists
        let manifest = cargo_root_path.join("Cargo.toml");
        if !manifest.exists() {
            return Err(Error::Compilation(format!(
                "Cargo.toml not found in: {}",
                cargo_root
            )));
        }

        let build_type = if release { "release" } else { "debug" };
        info!("🔨 [RUST] Building Cargo project: {}", cargo_root);
        info!("🔨 [RUST] Target type: {:?}", target_type);
        info!("🔨 [RUST] Build type: {}", build_type);

        // Build cargo command
        let mut cmd = Command::new("cargo");
        cmd.current_dir(cargo_root_path);

        // Add target-specific command and flags
        match target_type {
            CargoTargetType::Binary => {
                // Build binaries
                cmd.arg("build");
                cmd.arg("--message-format=json");
            }
            CargoTargetType::Test => {
                // Build tests without running them
                cmd.arg("test");
                cmd.arg("--no-run");
                cmd.arg("--message-format=json");
            }
            CargoTargetType::Example(name) => {
                // Build specific example
                cmd.arg("build");
                cmd.arg("--message-format=json");
                cmd.arg("--example");
                cmd.arg(name);
            }
        }

        if release {
            cmd.arg("--release");
        }

        debug!("🔨 [RUST] Running: cargo {:?}", cmd.as_std().get_args());

        // Execute compilation
        let output = cmd.output().await.map_err(|e| {
            Error::Compilation(format!(
                "Failed to execute cargo: {}. Is cargo installed?",
                e
            ))
        })?;

        // Check compilation result
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("❌ [RUST] Cargo build failed");
            error!("❌ [RUST] stderr:\n{}", stderr);
            return Err(Error::Compilation(format!(
                "Cargo build failed:\n{}",
                stderr
            )));
        }

        // Parse JSON output to find executable
        let stdout = String::from_utf8_lossy(&output.stdout);
        let executable = Self::parse_cargo_executable(&stdout, target_type)?;

        info!("✅ [RUST] Cargo build successful: {}", executable);

        Ok(executable)
    }

    /// Compile Rust source (auto-detects single-file vs Cargo project)
    ///
    /// This is the main entry point for Rust compilation. It automatically detects
    /// whether the source is part of a Cargo project and routes to the appropriate
    /// compilation method.
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to .rs source file
    /// * `release` - If true, compile with optimizations
    ///
    /// # Returns
    ///
    /// Path to compiled executable binary
    ///
    /// # Example
    ///
    /// ```rust
    /// // Single file - uses rustc
    /// let binary = RustAdapter::compile("/workspace/fizzbuzz.rs", false).await?;
    ///
    /// // Cargo project - uses cargo build
    /// let binary = RustAdapter::compile("/workspace/cargo-simple/src/main.rs", false).await?;
    /// ```
    pub async fn compile(source_path: &str, release: bool) -> Result<String> {
        // Detect project type
        let project_type = Self::detect_project_type(source_path)?;

        match project_type {
            RustProjectType::SingleFile(_) => {
                info!("📄 [RUST] Compiling single file with rustc");
                Self::compile_single_file(source_path, release).await
            }
            RustProjectType::CargoProject { root, .. } => {
                info!("📦 [RUST] Compiling Cargo project");
                let root_str = root
                    .to_str()
                    .ok_or_else(|| Error::Compilation("Non-UTF8 Cargo root path".to_string()))?;
                // Default to building binary target
                Self::compile_cargo_project(root_str, &CargoTargetType::Binary, release).await
            }
        }
    }

    /// Compile Rust source file to binary
    ///
    /// This compiles a single Rust source file using rustc.
    /// For Cargo projects, use `compile_cargo_project()` instead.
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to .rs source file (e.g., "/workspace/fizzbuzz.rs")
    /// * `release` - If true, compile with optimizations
    ///
    /// # Returns
    ///
    /// Path to compiled binary (e.g., "/workspace/target/debug/fizzbuzz")
    ///
    /// # Example
    ///
    /// ```rust
    /// let binary = RustAdapter::compile_single_file("/workspace/fizzbuzz.rs", false).await?;
    /// // binary = "/workspace/target/debug/fizzbuzz"
    /// ```
    pub async fn compile_single_file(source_path: &str, release: bool) -> Result<String> {
        let source = Path::new(source_path);

        // Validate source file exists
        if !source.exists() {
            return Err(Error::Compilation(format!(
                "Source file not found: {}",
                source_path
            )));
        }

        // Extract binary name from source filename
        let binary_name = source
            .file_stem()
            .ok_or_else(|| Error::Compilation("Invalid source filename".to_string()))?
            .to_str()
            .ok_or_else(|| Error::Compilation("Non-UTF8 filename".to_string()))?;

        // Determine output directory: <source_dir>/target/<debug|release>
        let source_dir = source
            .parent()
            .ok_or_else(|| Error::Compilation("Cannot determine source directory".to_string()))?;

        let build_type = if release { "release" } else { "debug" };
        let output_dir = source_dir.join("target").join(build_type);

        // Create output directory if it doesn't exist
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| Error::Compilation(format!("Failed to create output directory: {}", e)))?;

        let binary_path = output_dir.join(binary_name);

        info!("🔨 [RUST] Compiling: {}", source_path);
        info!("🔨 [RUST] Output: {}", binary_path.display());
        info!("🔨 [RUST] Build type: {}", build_type);

        // Build rustc command
        let mut cmd = Command::new("rustc");
        cmd.arg(source_path);
        cmd.arg("-o").arg(&binary_path);

        if release {
            // Release build: optimizations + debug symbols
            cmd.arg("-C").arg("opt-level=3");
            cmd.arg("-g"); // Keep debug symbols even in release
        } else {
            // Debug build: no optimizations, full debug symbols
            cmd.arg("-g");
        }

        // Execute compilation
        let output = cmd.output().await.map_err(|e| {
            Error::Compilation(format!(
                "Failed to execute rustc: {}. Is rustc installed?",
                e
            ))
        })?;

        // Check compilation result
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Compilation(format!(
                "Compilation failed:\n{}",
                stderr
            )));
        }

        let binary_path_str = binary_path
            .to_str()
            .ok_or_else(|| Error::Compilation("Non-UTF8 binary path".to_string()))?
            .to_string();

        info!("✅ [RUST] Compilation successful: {}", binary_path_str);

        Ok(binary_path_str)
    }

    /// Generate launch configuration for Rust debugging
    ///
    /// This creates the JSON configuration sent to CodeLLDB in the DAP launch request.
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to compiled binary (NOT source file)
    /// * `args` - Arguments to pass to the binary
    /// * `cwd` - Working directory (optional)
    /// * `stop_on_entry` - Whether to stop at program entry point
    ///
    /// # Note
    ///
    /// `binary_path` must be the compiled binary, not the source file!
    /// - ❌ Wrong: `/workspace/fizzbuzz.rs`
    /// - ✅ Correct: `/workspace/target/debug/fizzbuzz`
    pub fn launch_args(
        binary_path: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "type": "lldb",
            "request": "launch",
            "program": binary_path,  // Compiled binary, not source
            "args": args,
            "stopOnEntry": stop_on_entry,
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

impl DebugAdapterLogger for RustAdapter {
    fn language_name(&self) -> &str {
        "Rust"
    }

    fn language_emoji(&self) -> &str {
        "🦀"
    }

    fn transport_type(&self) -> &str {
        "STDIO"
    }

    fn adapter_id(&self) -> &str {
        "codelldb"
    }

    fn command_line(&self) -> String {
        format!("{} --port 0", Self::command())
    }

    fn requires_workaround(&self) -> bool {
        false // CodeLLDB supports stopOnEntry natively
    }

    fn workaround_reason(&self) -> Option<&str> {
        None
    }

    fn log_spawn_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUST] Failed to spawn CodeLLDB: {}", error);
        error!("   Command: {}", self.command_line());
        error!("   ");
        error!("   Possible causes:");
        error!("   1. CodeLLDB not installed or not in PATH");
        error!("      → Download from: https://github.com/vadimcn/codelldb/releases");
        error!("      → Or install via VS Code extension: vadimcn.vscode-lldb");
        error!("   2. Incorrect CodeLLDB path in container");
        error!("   3. CodeLLDB binary not executable");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ which codelldb");
        error!("   $ codelldb --version");
    }

    fn log_connection_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUST] Adapter connection failed: {}", error);
        error!("   Transport: STDIO");
        error!("   This shouldn't happen with STDIO transport");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. CodeLLDB process crashed on startup");
        error!("   2. STDIO pipes broken or closed unexpectedly");
        error!("   3. CodeLLDB version incompatible (need >= 1.11.0 for STDIO)");
        error!("   ");
        error!("   Check CodeLLDB stderr output for error messages.");
    }

    fn log_init_error(&self, error: &dyn std::error::Error) {
        error!("❌ [RUST] DAP initialization failed: {}", error);
        error!("   CodeLLDB started but couldn't complete DAP handshake");
        error!("   ");
        error!("   Possible causes:");
        error!("   1. Binary path doesn't exist or is not executable");
        error!("   2. Binary was not compiled with debug symbols (-g)");
        error!("   3. Binary architecture mismatch (e.g., x86_64 vs ARM64)");
        error!("   4. Incompatible CodeLLDB version");
        error!("   ");
        error!("   Verify binary can run:");
        error!("   $ file <binary_path>");
        error!("   $ <binary_path> --help");
    }
}

/// Helper to log Rust-specific compilation step
impl RustAdapter {
    pub fn log_compilation_start(source: &str, release: bool) {
        let build_type = if release { "release" } else { "debug" };
        info!("🔨 [RUST] Compiling {} ({} build)", source, build_type);
    }

    pub fn log_compilation_success(binary: &str) {
        info!("✅ [RUST] Compilation successful: {}", binary);
    }

    pub fn log_compilation_error(error: &dyn std::error::Error) {
        error!("❌ [RUST] Compilation failed: {}", error);
        error!("   ");
        error!("   Common compilation errors:");
        error!("   1. Syntax errors in source code");
        error!("   2. Missing dependencies (for Cargo projects)");
        error!("   3. rustc not installed or not in PATH");
        error!("   4. Incorrect source file path");
        error!("   ");
        error!("   Troubleshooting:");
        error!("   $ rustc --version");
        error!("   Expected: rustc 1.83.0 or higher");
        error!("   ");
        error!("   $ rustc <source_file>");
        error!("   This should show detailed compilation errors");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command() {
        // Should return a valid command path
        let cmd = RustAdapter::command();
        assert!(!cmd.is_empty());
        assert!(cmd.contains("codelldb"));
    }

    #[test]
    fn test_args() {
        let args = RustAdapter::args();
        assert_eq!(args.len(), 0); // Empty for STDIO mode (v1.11.0+)
    }

    #[test]
    fn test_adapter_id() {
        assert_eq!(RustAdapter::adapter_id(), "codelldb");
    }

    #[test]
    fn test_launch_args_basic() {
        let binary = "/workspace/target/debug/fizzbuzz";
        let args = vec![];
        let config = RustAdapter::launch_args(binary, &args, None, false);

        assert_eq!(config["type"], "lldb");
        assert_eq!(config["request"], "launch");
        assert_eq!(config["program"], binary);
        assert_eq!(config["args"], json!([]));
        assert_eq!(config["stopOnEntry"], false);
        assert!(config["cwd"].is_null());
    }

    #[test]
    fn test_launch_args_with_stop_on_entry() {
        let binary = "/workspace/target/debug/app";
        let args = vec!["--verbose".to_string()];
        let config = RustAdapter::launch_args(binary, &args, Some("/workspace"), true);

        assert_eq!(config["program"], binary);
        assert_eq!(config["args"], json!(["--verbose"]));
        assert_eq!(config["cwd"], "/workspace");
        assert_eq!(config["stopOnEntry"], true);
    }

    #[test]
    fn test_launch_args_with_multiple_args() {
        let binary = "/workspace/target/release/cli";
        let args = vec![
            "--config".to_string(),
            "config.toml".to_string(),
            "--verbose".to_string(),
        ];
        let config = RustAdapter::launch_args(binary, &args, None, false);

        assert_eq!(config["args"], json!(args));
    }

    // Compilation tests require rustc installed
    #[tokio::test]
    #[ignore] // Only run when rustc is available
    async fn test_compile_single_file_creates_binary() {
        // This test requires a real Rust source file
        // In CI/CD, this would be run inside Dockerfile.rust container
        let test_source = "/tmp/test_fizzbuzz.rs";

        // Create a simple test program
        tokio::fs::write(
            test_source,
            r#"
fn main() {
    println!("Hello from test");
}
"#,
        )
        .await
        .unwrap();

        let binary = RustAdapter::compile_single_file(test_source, false)
            .await
            .unwrap();

        // Verify binary was created
        assert!(Path::new(&binary).exists());

        // Clean up
        let _ = tokio::fs::remove_file(test_source).await;
        let _ = tokio::fs::remove_file(&binary).await;
    }
}
