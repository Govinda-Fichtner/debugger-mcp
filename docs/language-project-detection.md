# Language Project Detection Logic

**Version:** 0.1.0
**Last Updated:** 2025-10-18

---

## Overview

This document describes which language adapters in the Debugger MCP Server have project detection logic and how they handle different project structures.

---

## Summary Table

| Language | Project Detection | Compilation Required | Build System | Detection Logic Location |
|----------|------------------|---------------------|--------------|-------------------------|
| **Rust** | ✅ YES | ✅ YES | rustc / Cargo | `src/adapters/rust.rs:218-274` |
| **Python** | ❌ NO | ❌ NO | N/A | No detection needed |
| **Ruby** | ❌ NO | ❌ NO | N/A | No detection needed |
| **Node.js** | ❌ NO | ❌ NO | N/A | No detection needed |
| **Go** | ❌ NO | ✅ YES | Delve (auto) | Delve handles internally |

---

## Detailed Analysis

### Rust: Complex Project Detection ✅

**Why needed:** Rust has two distinct compilation paths:
- **Standalone files** → Compile with `rustc`
- **Cargo projects** → Compile with `cargo build`

**Detection algorithm:** `src/adapters/rust.rs:218-274`

```rust
pub fn detect_project_type(source_path: &str) -> Result<RustProjectType>
```

**Detection logic:**
1. Walk up directory tree from source file
2. Look for `Cargo.toml` in parent directories
3. If found, check if source is under recognized Cargo subdirectories:
   - `src/` - Main source code
   - `examples/` - Example programs
   - `tests/` - Integration tests (EXCEPT `tests/fixtures/`)
   - `benches/` - Benchmarks
   - `bin/` - Binary targets
4. **Exception:** `tests/fixtures/` is treated as standalone
5. If under recognized subdirs → CargoProject
6. Otherwise → SingleFile

**Test coverage:**
- ✅ 6 comprehensive unit tests covering all scenarios
- ✅ Integration test with `tests/fixtures/fizzbuzz.rs`

**Documentation:**
- Detailed scenarios: `docs/rust-adapter-scenarios.md`
- Investigation notes: `docs/rust-compilation-flow-analysis.md`

---

### Python: No Detection Needed ❌

**File:** `src/adapters/python.rs`

**How it works:**
- Python is interpreted, not compiled
- debugpy debug adapter runs `.py` files directly
- No build system to detect (no `setup.py` vs standalone distinction needed)

**Launch flow:**
```rust
pub async fn launch(
    &self,
    program: String,
    args: Option<Vec<String>>,
    stop_on_entry: Option<bool>,
) -> Result<LaunchResponse>
```

**What happens:**
1. Receives path to `.py` file (e.g., `/workspace/app.py`)
2. Sends DAP `launch` request to debugpy with file path
3. debugpy starts Python interpreter with the script
4. No compilation, no project detection

**Example launch config:**
```json
{
  "request": "launch",
  "type": "python",
  "program": "/workspace/app.py",
  "args": ["--verbose"],
  "stopOnEntry": true
}
```

---

### Ruby: No Detection Needed ❌

**File:** `src/adapters/ruby.rs`

**How it works:**
- Ruby is interpreted, not compiled
- rdbg (ruby/debug) runs `.rb` files directly
- No build system to detect (no Bundler vs standalone distinction needed)

**Launch flow:**
```rust
pub async fn launch(
    &self,
    program: String,
    args: Option<Vec<String>>,
    stop_on_entry: Option<bool>,
) -> Result<LaunchResponse>
```

**What happens:**
1. Receives path to `.rb` file (e.g., `/workspace/app.rb`)
2. Sends DAP `launch` request to rdbg with file path
3. rdbg starts Ruby interpreter with the script
4. No compilation, no project detection

**Example launch config:**
```json
{
  "request": "launch",
  "type": "ruby",
  "program": "/workspace/app.rb",
  "args": [],
  "stopOnEntry": true
}
```

---

### Node.js: No Detection Needed ❌

**File:** `src/adapters/nodejs.rs`

**How it works:**
- JavaScript is interpreted (or JIT-compiled by V8)
- vscode-js-debug runs `.js` files directly
- No build system to detect (npm/yarn are for dependencies, not compilation)

**Launch flow:**
```rust
pub async fn launch(
    &self,
    program: String,
    args: Option<Vec<String>>,
    stop_on_entry: Option<bool>,
) -> Result<LaunchResponse>
```

**What happens:**
1. Receives path to `.js` file (e.g., `/workspace/app.js`)
2. Sends DAP `launch` request to vscode-js-debug with file path
3. Adapter starts Node.js with the script
4. No compilation, no project detection

**Example launch config:**
```json
{
  "request": "launch",
  "type": "node",
  "program": "/workspace/app.js",
  "args": [],
  "stopOnEntry": true
}
```

**Note:** While Node.js projects may use TypeScript (which needs compilation), that's handled by the user's build process before debugging. The debugger receives already-transpiled `.js` files.

---

### Go: Delve Handles Automatically ❌

**File:** `src/adapters/golang.rs`

**How it works:**
- Go requires compilation, but Delve (the debug adapter) handles it automatically
- Delve compiles Go code with debug flags internally
- No explicit project detection needed in MCP server

**Launch flow:**
```rust
pub async fn launch(
    &self,
    program: String,
    args: Option<Vec<String>>,
    stop_on_entry: Option<bool>,
) -> Result<LaunchResponse>
```

**What happens:**
1. Receives path to `.go` file or package (e.g., `/workspace/main.go`)
2. Sends DAP `launch` request to Delve with file/package path
3. **Delve internally:**
   - Detects if it's in a Go module (go.mod)
   - Compiles with `go build -gcflags="all=-N -l"` (disables optimizations)
   - Launches compiled binary with LLDB/GDB
4. MCP server doesn't need to know about Go modules

**Example launch config:**
```json
{
  "request": "launch",
  "type": "go",
  "program": "/workspace/main.go",
  "args": [],
  "stopOnEntry": true
}
```

**Why no detection needed:** Delve is smart enough to:
- Find `go.mod` if present
- Use correct module path
- Resolve dependencies
- Compile with debug symbols

---

## Why Only Rust Needs Detection

### The Key Difference: Compilation Responsibility

| Language | Who Compiles? | Build System Detection |
|----------|--------------|----------------------|
| **Rust** | MCP Server | ✅ Required (rustc vs cargo) |
| **Python** | N/A (interpreted) | ❌ Not needed |
| **Ruby** | N/A (interpreted) | ❌ Not needed |
| **Node.js** | N/A (interpreted/JIT) | ❌ Not needed |
| **Go** | Delve (debug adapter) | ❌ Delve handles it |

### Why Rust is Special

1. **Two compilation paths:**
   - `rustc file.rs` - For standalone files
   - `cargo build` - For Cargo projects

2. **Cargo project structure matters:**
   - Files in `src/`, `tests/`, etc. are part of the project
   - Files in `tests/fixtures/` are NOT part of the project
   - Using wrong compilation path causes breakpoint verification failure

3. **MCP server must decide:**
   - The Rust debug adapter (CodeLLDB) doesn't compile Rust code
   - CodeLLDB only debugs pre-compiled binaries
   - MCP server must compile before launching debugger
   - Wrong compilation choice = broken debugging

4. **No universal Rust project marker:**
   - Presence of `Cargo.toml` doesn't guarantee file is part of project
   - Test fixtures exist in Cargo projects but compile standalone
   - Need path-based logic to decide

### Why Go Doesn't Need It

Delve (Go debugger) is smarter:
- Delve compiles Go code itself
- Delve detects `go.mod` automatically
- Delve chooses correct compilation flags
- MCP server just passes the path and Delve handles the rest

---

## Adding Future Languages

### Compiled Languages

If adding a new compiled language, ask:

1. **Does the debug adapter compile the code?**
   - YES → No detection needed (like Go with Delve)
   - NO → Need detection logic (like Rust)

2. **Are there multiple build systems?**
   - YES → Need detection logic
   - NO → Can use single compilation approach

3. **Does project structure affect compilation?**
   - YES → Need path-based detection
   - NO → Can compile uniformly

### Interpreted Languages

No detection needed - just pass script path to debug adapter.

---

## Test Coverage Requirements

### Languages WITH Detection Logic

**Required tests:**
- ✅ Unit tests for detection algorithm (all scenarios)
- ✅ Integration tests for each project type
- ✅ Edge cases (fixtures, standalone in project dirs)

**Rust example:** 6 unit tests in `src/adapters/rust.rs:859-1030`

### Languages WITHOUT Detection Logic

**Required tests:**
- ✅ Integration test with sample script
- ❌ No detection logic to unit test

---

## Related Documentation

- [Rust Adapter Scenarios](./rust-adapter-scenarios.md) - Detailed Rust detection behavior
- [Rust Compilation Flow Analysis](./rust-compilation-flow-analysis.md) - Investigation notes
- [DAP MCP Server Proposal](./DAP_MCP_SERVER_PROPOSAL.md) - Overall architecture

---

**Key Takeaway:** Only Rust has project detection logic because it's the only language where the MCP server is responsible for compilation AND there are multiple build paths to choose from.
