# Rust Debugging Support - Research and Implementation Proposal

**Date**: 2024-10-07
**Phase**: Research Complete - Proposing Architecture
**Target**: Production-ready Rust debugging support

---

## Executive Summary

After researching CodeLLDB, dap-rs, and analyzing our existing Python/Ruby/Node.js implementations, I propose a **STDIO-based approach** using **CodeLLDB** (vadimcn.vscode-lldb extension) for Rust debugging support.

**Key Decision**: Use CodeLLDB with STDIO transport (similar to Python), not socket-based (like Ruby/Node.js).

**Rationale**: CodeLLDB supports STDIO natively (since v1.11.0), making it the simplest and most reliable approach.

---

## Research Findings

### 1. CodeLLDB Overview

**What is CodeLLDB?**
- Debug adapter for C, C++, and **Rust** based on LLDB
- Part of the `vadimcn.vscode-lldb` VS Code extension
- Implements Debug Adapter Protocol (DAP)
- Actively maintained (widely used in VS Code Rust ecosystem)

**Key Features**:
- ✅ STDIO transport support (v1.11.0+)
- ✅ Native Rust debugging with LLDB backend
- ✅ Supports both debug and release builds (with debug symbols)
- ✅ Works with Cargo projects and standalone binaries
- ✅ `stopOnEntry` supported natively
- ✅ Expression evaluation in Rust context
- ✅ Stack traces, variables, breakpoints

**Installation**:
- Download from VS Code marketplace or GitHub releases
- Extract `.vsix` file (it's a ZIP archive)
- Binary location: `extension/adapter/codelldb`
- Platform-specific: Linux x86_64/aarch64, macOS, Windows

### 2. Transport Mechanism

**CodeLLDB supports STDIO** (since v1.11.0):
```bash
# Launch CodeLLDB in STDIO mode
/path/to/codelldb --port 0
# Port 0 = STDIO mode (not TCP socket)
```

**Comparison**:
- **Python (debugpy)**: STDIO ✅ - Simple, reliable
- **Ruby (rdbg)**: Socket ⚠️ - Required (no STDIO DAP support)
- **Node.js (vscode-js-debug)**: Socket ⚠️ - Required (multi-session architecture)
- **Rust (CodeLLDB)**: STDIO ✅ - Supported, preferred

**Decision**: Use STDIO for Rust (like Python), not socket.

### 3. Launch Configuration

**Basic Rust launch config**:
```json
{
  "type": "lldb",
  "request": "launch",
  "program": "/workspace/target/debug/fizzbuzz",
  "args": [],
  "cwd": "/workspace",
  "stopOnEntry": true
}
```

**Key differences from other languages**:
- **Program path**: Points to compiled **binary**, not source file
  - Python: `/workspace/fizzbuzz.py` (source)
  - Ruby: `/workspace/fizzbuzz.rb` (source)
  - Node.js: `/workspace/fizzbuzz.js` (source)
  - **Rust**: `/workspace/target/debug/fizzbuzz` (**binary**)
- **Build requirement**: Must compile before debugging
- **Type**: `"lldb"` (not `"rust"` or `"codelldb"`)

### 4. Cargo Integration Challenge

**Problem**: User provides source file, we need binary path.

**Examples**:
```
User input: /workspace/src/main.rs
We need:    /workspace/target/debug/my_project_name

User input: /workspace/fizzbuzz.rs
We need:    /workspace/target/debug/fizzbuzz
```

**Solution Options**:

**Option A: Simple file-based** (start here)
```rust
// For simple Rust files: fizzbuzz.rs → target/debug/fizzbuzz
let source_path = "/workspace/fizzbuzz.rs";
let binary_name = Path::new(source_path)
    .file_stem()
    .unwrap()
    .to_str()
    .unwrap();
let binary_path = format!("/workspace/target/debug/{}", binary_name);
```

**Option B: Cargo metadata** (future enhancement)
```rust
// For Cargo projects: parse Cargo.toml or run `cargo metadata`
let metadata = run_cargo_metadata()?;
let binary_path = metadata.target_directory + "/debug/" + metadata.package_name;
```

**Option C: User provides binary** (alternative)
```rust
// User specifies binary path directly
debugger_start({
  language: "rust",
  program: "/workspace/target/debug/fizzbuzz"  // Binary, not source
})
```

**Recommendation**: Start with **Option A** for single-file Rust programs, add **Option B** for Cargo projects in future iteration.

### 5. Debug vs Release Builds

**Debug builds** (`cargo build`):
- Full debug symbols
- No optimizations
- Larger binaries
- Best debugging experience
- **Default target**: `target/debug/<name>`

**Release builds** (`cargo build --release`):
- Optimized code
- Requires `debug = true` in Cargo.toml
- Harder to debug (inlined functions, optimized away variables)
- **Target**: `target/release/<name>`

**Decision**: Support both, but **recommend debug builds** in documentation.

**Implementation**:
```rust
pub fn launch_args(program: &str, release: bool) -> Value {
    // program is the source file path
    let binary_path = if release {
        derive_release_binary_path(program)
    } else {
        derive_debug_binary_path(program)
    };

    json!({
        "type": "lldb",
        "request": "launch",
        "program": binary_path,
        "stopOnEntry": stop_on_entry,
    })
}
```

### 6. Lessons Learned from Our Implementations

#### Python (STDIO - Simple)
**✅ What worked well**:
- STDIO is simple and reliable
- No port allocation, no connection retries
- Native `stopOnEntry` support
- Clear error messages

**Takeaway**: STDIO is preferred when supported.

#### Ruby (Socket - Forced)
**⚠️ What caused issues**:
- rdbg doesn't support STDIO DAP
- Socket mode required workarounds:
  - Port allocation
  - Connection retry logic
  - 2-second timeout
  - Entry breakpoint workaround (stopOnEntry broken)

**Lesson**: Avoid sockets if STDIO works.

**Critical finding**: "rdbg does NOT support DAP protocol via stdio. It only supports DAP via socket (`--open` flag)."

#### Node.js (Socket - Multi-session)
**🔧 What needed workarounds**:
- Multi-session architecture (parent + child)
- `stopOnEntry` doesn't work on parent → entry breakpoint on child
- No response to child launch request
- Event forwarding from child to parent

**Lesson**: Complex architectures add significant complexity. Avoid if simpler alternative exists.

#### Container Paths (All languages)
**📘 Critical lesson**:
- #1 user pain point: Container path confusion
- Host: `/home/vagrant/projects/fizzbuzz.rs`
- Container: `/workspace/fizzbuzz.rs`
- **All paths must use container perspective**

**For Rust**:
- Source: `/workspace/fizzbuzz.rs` (or `/workspace/src/main.rs`)
- Binary: `/workspace/target/debug/fizzbuzz`
- Both must use container paths!

---

## Proposed Architecture

### High-Level Design

```
AI Agent (Claude Desktop)
    ↕ MCP Protocol
┌─────────────────────────────────────────┐
│     DAP MCP Server (Rust/Tokio)         │
│                                         │
│  ┌────────────────────────────────────┐ │
│  │  Rust Adapter                      │ │
│  │  - Compile Rust source             │ │
│  │  - Derive binary path              │ │
│  │  - Spawn CodeLLDB via STDIO        │ │
│  └──────────────┬─────────────────────┘ │
└─────────────────┼─────────────────────────┘
                  ↕ STDIO (like Python)
            ┌─────┴──────┐
            │  CodeLLDB  │
            │  (LLDB)    │
            └─────┬──────┘
                  ↕
         ┌────────┴─────────┐
         │  Rust Binary     │
         │  (target/debug/) │
         └──────────────────┘
```

**Key characteristics**:
- ✅ STDIO transport (simple like Python)
- ✅ Single session (no multi-session complexity)
- ✅ Native stopOnEntry support
- ✅ Compilation step before debugging
- ✅ Container-aware paths

### Adapter Structure

```rust
// src/adapters/rust.rs

use serde_json::{json, Value};
use super::logging::DebugAdapterLogger;
use std::path::Path;

/// Rust CodeLLDB adapter configuration
pub struct RustAdapter;

impl RustAdapter {
    /// Get CodeLLDB command
    pub fn command() -> String {
        // First check Docker container path
        if Path::new("/usr/local/bin/codelldb").exists() {
            "/usr/local/bin/codelldb".to_string()
        } else if Path::new("/usr/bin/codelldb").exists() {
            "/usr/bin/codelldb".to_string()
        } else {
            "codelldb".to_string() // Hope it's in PATH
        }
    }

    /// Get CodeLLDB args for STDIO mode
    pub fn args() -> Vec<String> {
        vec!["--port".to_string(), "0".to_string()] // Port 0 = STDIO
    }

    pub fn adapter_id() -> &'static str {
        "codelldb"
    }

    /// Compile Rust source and return binary path
    pub async fn compile(source_path: &str, release: bool) -> Result<String> {
        // Determine if this is a Cargo project or single file
        let is_cargo_project = Path::new(source_path)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("Cargo.toml").exists())
            .unwrap_or(false);

        if is_cargo_project {
            // Cargo project: Run cargo build
            compile_cargo_project(source_path, release).await
        } else {
            // Single file: Use rustc
            compile_single_file(source_path, release).await
        }
    }

    /// Generate launch configuration for Rust debugging
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
```

### Compilation Strategy

**Single-file Rust programs** (for testing):
```rust
async fn compile_single_file(source: &str, release: bool) -> Result<String> {
    // Example: /workspace/fizzbuzz.rs
    let source_path = Path::new(source);
    let binary_name = source_path.file_stem().unwrap();

    // Output: /workspace/target/debug/fizzbuzz
    let output_dir = source_path.parent().unwrap().join("target");
    let build_type = if release { "release" } else { "debug" };
    let binary_path = output_dir.join(build_type).join(binary_name);

    // Create output directory
    std::fs::create_dir_all(&output_dir.join(build_type))?;

    // Compile with rustc
    let mut cmd = Command::new("rustc");
    cmd.arg(source);
    cmd.arg("-o").arg(&binary_path);

    if !release {
        cmd.arg("-g"); // Debug symbols
    } else {
        cmd.arg("-C").arg("opt-level=3");
        cmd.arg("-g"); // Debug symbols even in release
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Compilation(format!("Compilation failed:\n{}", stderr)));
    }

    Ok(binary_path.to_string_lossy().to_string())
}
```

**Cargo projects** (future):
```rust
async fn compile_cargo_project(source: &str, release: bool) -> Result<String> {
    // Find Cargo.toml
    let cargo_toml = find_cargo_toml(source)?;
    let project_dir = cargo_toml.parent().unwrap();

    // Run cargo build
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    cmd.current_dir(project_dir);

    if release {
        cmd.arg("--release");
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Compilation(format!("Cargo build failed:\n{}", stderr)));
    }

    // Parse Cargo.toml to get binary name
    let metadata = parse_cargo_metadata(project_dir)?;
    let build_type = if release { "release" } else { "debug" };
    let binary_path = project_dir
        .join("target")
        .join(build_type)
        .join(&metadata.package_name);

    Ok(binary_path.to_string_lossy().to_string())
}
```

### Manager Integration

```rust
// In src/debug/manager.rs

"rust" => {
    let adapter = RustAdapter;
    adapter.log_selection();
    adapter.log_transport_init();

    // Step 1: Compile the Rust source
    info!("🔨 [RUST] Compiling source: {}", program);
    let binary_path = RustAdapter::compile(&program, false)  // false = debug build
        .await
        .map_err(|e| {
            error!("❌ [RUST] Compilation failed: {}", e);
            e
        })?;
    info!("✅ [RUST] Compiled to: {}", binary_path);

    // Step 2: Spawn CodeLLDB adapter
    let cmd = RustAdapter::command();
    let adapter_args = RustAdapter::args();
    let adapter_id = RustAdapter::adapter_id();
    let launch_args = RustAdapter::launch_args(
        &binary_path,  // Use compiled binary path
        &args,
        cwd.as_deref(),
        stop_on_entry,
    );

    adapter.log_transport_init();
    (cmd, adapter_args, adapter_id, launch_args)
}
```

### Dockerfile.rust

```dockerfile
# Multi-stage build for lean production image
# Stage 1: Build the Rust MCP server binary
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Create app directory
WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary with static linking
RUN cargo build --release

# Stage 2: Create Rust debugging runtime image
FROM rust:1.83-alpine

# Install runtime dependencies for Rust debugging
# - rustc: Rust compiler (for single-file programs)
# - cargo: Rust package manager (for Cargo projects)
# - wget: To download CodeLLDB
# - lldb: LLDB debugger (CodeLLDB backend)
RUN apk add --no-cache \
    rustc \
    cargo \
    wget \
    lldb \
    && rm -rf /var/cache/apk/*

# Download and install CodeLLDB
# Version: v1.11.0 (latest with STDIO support)
RUN cd /tmp && \
    wget -q https://github.com/vadimcn/codelldb/releases/download/v1.11.0/codelldb-x86_64-linux.vsix && \
    unzip -q codelldb-x86_64-linux.vsix -d /usr/local/lib/codelldb && \
    ln -s /usr/local/lib/codelldb/extension/adapter/codelldb /usr/local/bin/codelldb && \
    rm codelldb-x86_64-linux.vsix && \
    chmod +x /usr/local/bin/codelldb

# Verify installations
RUN echo "=== Rust Debugging Environment ===" && \
    echo "Rust version: $(rustc --version)" && \
    echo "Cargo version: $(cargo --version)" && \
    echo "LLDB version: $(lldb --version | head -1)" && \
    echo "CodeLLDB: $(ls -la /usr/local/bin/codelldb)" && \
    echo "✅ Rust debugging environment ready"

# Create non-root user
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser

# Copy MCP server binary from builder
COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/debugger_mcp

# Set ownership
RUN chown mcpuser:mcpuser /usr/local/bin/debugger_mcp

# Switch to non-root user
USER mcpuser

# Set working directory
WORKDIR /workspace

# Default command
CMD ["debugger_mcp", "serve"]

# Metadata
LABEL org.opencontainers.image.title="debugger-mcp-rust"
LABEL org.opencontainers.image.description="DAP MCP Server - Rust Debugging Support"
LABEL org.opencontainers.image.source="https://github.com/Govinda-Fichtner/debugger-mcp"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.variant="rust"
```

**Image size estimate**: ~800-900 MB (Rust toolchain is large)

### Testing Strategy

**Test program**: FizzBuzz with deliberate bug

```rust
// /home/vagrant/projects/fizzbuzz-rust-test/fizzbuzz.rs

fn fizzbuzz(n: i32) -> String {
    if n % 15 == 0 {
        "FizzBuzz".to_string()
    } else if n % 3 == 0 {
        "Fizz".to_string()
    } else if n % 4 == 0 {  // BUG: Should be n % 5
        "Buzz".to_string()
    } else {
        n.to_string()
    }
}

fn main() {
    for i in 1..=20 {
        println!("{}: {}", i, fizzbuzz(i));
    }
}
```

**Test workflow**:
1. Start session with stopOnEntry
2. Set breakpoint at line 9 (the buggy line)
3. Continue execution
4. Stop at breakpoint
5. Evaluate: `n`, `n % 4`, `n % 5`
6. Verify bug: uses `n % 4` instead of `n % 5`
7. Step through execution
8. Disconnect cleanly

**Integration test** (`tests/test_rust_integration.rs`):
```rust
#[tokio::test]
#[ignore]  // Requires CodeLLDB
async fn test_rust_fizzbuzz_debugging_workflow() {
    let source = "/workspace/fizzbuzz-rust-test/fizzbuzz.rs";

    // 1. Start debugging (will compile automatically)
    let session_id = start_rust_session(source, true).await.unwrap();

    // 2. Wait for stopped (entry point)
    wait_for_stopped(&session_id, 3000).await.unwrap();

    // 3. Set breakpoint at buggy line
    let bp = set_breakpoint(&session_id, source, 9).await.unwrap();
    assert!(bp.verified);

    // 4. Continue
    continue_execution(&session_id).await.unwrap();

    // 5. Wait for breakpoint hit
    wait_for_stopped(&session_id, 5000).await.unwrap();

    // 6. Evaluate to find bug
    let n_val = evaluate(&session_id, "n", None).await.unwrap();
    let mod4 = evaluate(&session_id, "n % 4", None).await.unwrap();
    let mod5 = evaluate(&session_id, "n % 5", None).await.unwrap();

    // Verify bug detection
    assert_eq!(mod5, "0");  // Should use this
    assert_ne!(mod4, "0");  // But using this (bug!)

    // 7. Disconnect
    disconnect(&session_id).await.unwrap();
}
```

---

## Implementation Plan

### Phase 1: Basic Single-File Support (Week 1)

**Goal**: Debug simple Rust files like `fizzbuzz.rs`

**Tasks**:
1. Create `src/adapters/rust.rs` (250 lines)
   - `RustAdapter` struct
   - `command()`, `args()`, `adapter_id()`
   - `launch_args()` configuration
   - `compile_single_file()` with rustc
   - `DebugAdapterLogger` implementation

2. Add to `src/adapters/mod.rs`
   - Export `RustAdapter`

3. Add Rust case to `src/debug/manager.rs` (50 lines)
   - Compile source
   - Spawn CodeLLDB
   - Create session

4. Add compilation error type to `src/error.rs`
   - `Error::Compilation(String)`

5. Create `Dockerfile.rust` (80 lines)
   - Install Rust toolchain
   - Download CodeLLDB
   - Verify installations

**Deliverable**: Can debug single-file Rust programs in Docker

### Phase 2: Testing and Validation (Week 1-2)

**Tasks**:
6. Create test directory:
   - `/home/vagrant/projects/fizzbuzz-rust-test/`
   - `fizzbuzz.rs` with bug
   - `README.md` usage guide
   - `RUST_DEBUGGING_PROMPT.md` (similar to Node.js/Ruby)

7. Create integration tests:
   - `tests/test_rust_integration.rs` (300 lines)
   - Test compilation
   - Test basic debugging workflow
   - Test FizzBuzz bug detection
   - Test expression evaluation

8. Manual E2E testing with Claude:
   - Build Docker image
   - Test complete workflow
   - Verify container paths work
   - Document any issues

**Deliverable**: All tests passing, E2E validated

### Phase 3: Cargo Project Support (Week 2-3)

**Goal**: Debug full Cargo projects

**Tasks**:
9. Implement `compile_cargo_project()`:
   - Find Cargo.toml
   - Run `cargo build`
   - Parse metadata for binary name
   - Handle multi-binary projects

10. Add Cargo detection logic:
    - Check for Cargo.toml
    - Choose compilation strategy

11. Test with real Cargo project:
    - Create test Cargo project
    - Add integration test
    - Verify workspace handling

**Deliverable**: Can debug both single files and Cargo projects

### Phase 4: Documentation and Polish (Week 3)

**Tasks**:
12. Create documentation:
    - `docs/RUST_DEBUGGING_ARCHITECTURE.md`
    - `docs/RUST_SUPPORT_ANALYSIS.md`
    - Update `docs/CONTAINER_PATH_GUIDE.md` with Rust examples
    - Update `docs/EXPRESSION_SYNTAX_GUIDE.md` with Rust expressions
    - Update `docs/TROUBLESHOOTING.md` with Rust issues

13. Update README.md:
    - Add Rust to supported languages
    - Update status checklist
    - Add Rust examples
    - Update Quick Links

14. Add to CI/CD:
    - Build Dockerfile.rust
    - Run Rust integration tests

**Deliverable**: Production-ready, documented Rust support

---

## Risk Assessment

### Low Risk ✅

1. **STDIO transport** - CodeLLDB supports it natively, proven approach (Python)
2. **Native stopOnEntry** - No workarounds needed (unlike Node.js)
3. **Single session** - Simple architecture (like Python)
4. **Mature debugger** - CodeLLDB widely used, stable

### Medium Risk ⚠️

1. **Compilation step** - Adds complexity:
   - **Mitigation**: Clear error messages, test extensively
   - **Fallback**: User can pre-compile and provide binary path

2. **Binary path derivation** - Determining correct binary:
   - **Mitigation**: Start with simple single-file logic
   - **Enhancement**: Add Cargo metadata parsing later

3. **Docker image size** - Rust toolchain is large (~800 MB):
   - **Mitigation**: Acceptable for production use
   - **Alternative**: Multi-stage build keeps runtime image as small as possible

### High Risk ❌

None identified. This is a straightforward implementation following proven patterns.

---

## Timeline Estimate

- **Phase 1**: Basic support - 3-4 days
- **Phase 2**: Testing - 2-3 days
- **Phase 3**: Cargo support - 3-4 days
- **Phase 4**: Documentation - 2 days

**Total**: 10-13 days to production-ready Rust support

**Critical path**: Compilation logic → Testing → Cargo support

---

## Success Criteria

1. ✅ Build Docker image successfully
2. ✅ Compile single-file Rust programs
3. ✅ Debug with breakpoints, stepping, evaluation
4. ✅ FizzBuzz test passes (bug detection)
5. ✅ Container paths work correctly
6. ✅ No compiler warnings
7. ✅ Documentation complete
8. ✅ E2E validation with Claude
9. ✅ (Stretch) Cargo projects work

---

## Open Questions

### Q1: CodeLLDB download strategy?

**Options**:
- A) Download from GitHub releases (current proposal)
- B) Build from source (slow, complex)
- C) Use system package manager (may not have latest)

**Recommendation**: Option A (GitHub releases) - Fast, reliable, specific version control

### Q2: How to handle multi-binary Cargo projects?

**Example**: A project with multiple binaries in `[[bin]]` sections

**Options**:
- A) Use first binary found (simple)
- B) User specifies which binary (explicit)
- C) Build all, let user choose (complex)

**Recommendation**: Start with Option A, add Option B if needed

### Q3: Support for release builds?

**Recommendation**: Yes, but **document that debug builds are preferred**. Add `release: bool` parameter to MCP tool later.

---

## Comparison with Existing Languages

| Aspect | Python | Ruby | Node.js | **Rust (Proposed)** |
|--------|--------|------|---------|---------------------|
| **Transport** | STDIO | Socket | Socket | **STDIO** ✅ |
| **Adapter** | debugpy | rdbg | vscode-js-debug | **CodeLLDB** |
| **Compilation** | No | No | No | **Yes** 🔨 |
| **Program input** | Source | Source | Source | **Source → Binary** |
| **stopOnEntry** | Native | Workaround | Workaround | **Native** ✅ |
| **Sessions** | Single | Single | Multi (parent+child) | **Single** ✅ |
| **Complexity** | Low | Medium | High | **Low-Medium** |
| **Image size** | 120 MB | 100 MB | 200 MB | **~800 MB** 📦 |

**Rust characteristics**:
- ✅ Simple architecture (STDIO, single session)
- ✅ No workarounds needed (native stopOnEntry)
- 🔨 Adds compilation step (new complexity)
- 📦 Larger image (Rust toolchain required)

---

## Conclusion

**Recommendation**: Proceed with CodeLLDB + STDIO approach.

**Why this is the right choice**:
1. ✅ Proven pattern (STDIO like Python)
2. ✅ Mature, stable debugger (CodeLLDB)
3. ✅ Simple architecture (single session)
4. ✅ Native features (stopOnEntry works)
5. ✅ Production-ready (widely used)

**Key innovation**: Compilation step is new, but straightforward.

**Confidence level**: High - This follows our successful Python pattern with one additional step (compilation).

---

**Next steps**: Upon approval, begin Phase 1 implementation.

**Questions?** Ready to clarify any aspect of this proposal.
