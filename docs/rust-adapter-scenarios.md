# Rust Adapter: Supported Scenarios

**Version:** 0.1.0
**Last Updated:** 2025-10-18

---

## Overview

The Rust adapter in the Debugger MCP Server handles multiple Rust project types automatically. It detects the project structure and compiles appropriately.

---

## Supported Scenarios

### Scenario 1: Standalone Source File

**User provides:** Path to `.rs` file not in any Cargo project

**Example:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "stopOnEntry": true
}
```

**What the server does:**
1. Detects no `Cargo.toml` in parent directories
2. Treats as standalone file
3. Compiles with `rustc`:
   ```bash
   rustc /tmp/hello.rs -g -C opt-level=0 -o /tmp/debugger-mcp-XXXX/hello
   ```
4. Launches CodeLLDB with compiled binary

**Compilation flags:**
- `-g` - Include debug symbols for LLDB
- `-C opt-level=0` - No optimizations (better debugging)

---

### Scenario 2: Cargo Project Source File

**User provides:** Path to `.rs` file that is part of a Cargo project

**Example:**
```json
{
  "language": "rust",
  "program": "/workspace/my-app/src/main.rs",
  "stopOnEntry": true
}
```

**Project structure:**
```
/workspace/my-app/
├── Cargo.toml
└── src/
    └── main.rs  ← User provides this path
```

**What the server does:**
1. Walks up directory tree, finds `/workspace/my-app/Cargo.toml`
2. Checks if `src/main.rs` is under `src/` → YES
3. Treats as Cargo project
4. Compiles with `cargo build`:
   ```bash
   cd /workspace/my-app && cargo build --message-format=json
   ```
5. Parses JSON output to find binary: `/workspace/my-app/target/debug/my-app`
6. Launches CodeLLDB with compiled binary

**Cargo subdirectories recognized:**
- `src/` - Main source code
- `examples/` - Example programs
- `tests/` - Integration tests
- `benches/` - Benchmarks
- `bin/` - Binary targets

**EXCEPTION:** `tests/fixtures/` is treated as standalone (see Scenario 4)

---

### Scenario 3: Pre-Compiled Binary

**User provides:** Path to already-compiled binary (no `.rs` extension)

**Example:**
```json
{
  "language": "rust",
  "program": "/workspace/target/debug/my-app",
  "stopOnEntry": true
}
```

**What the server does:**
1. Detects path does NOT end with `.rs`
2. Uses binary path as-is (no compilation)
3. Launches CodeLLDB directly

**Use case:** User pre-compiles with specific flags or wants faster startup

---

### Scenario 4: Test Fixture Files (Standalone in Cargo Project)

**User provides:** Path to `.rs` file in `tests/fixtures/` directory

**Example:**
```json
{
  "language": "rust",
  "program": "/workspace/tests/fixtures/fizzbuzz.rs",
  "stopOnEntry": true
}
```

**Project structure:**
```
/workspace/
├── Cargo.toml
├── src/
│   └── main.rs
└── tests/
    └── fixtures/
        └── fizzbuzz.rs  ← User provides this path
```

**What the server does:**
1. Walks up directory tree, finds `/workspace/Cargo.toml`
2. Checks if path is under `tests/` → YES
3. **EXCEPTION CHECK:** Path starts with `tests/fixtures/` → YES
4. Treats as standalone file (NOT Cargo project member)
5. Compiles with `rustc`:
   ```bash
   rustc /workspace/tests/fixtures/fizzbuzz.rs -g -C opt-level=0 -o /workspace/tests/fixtures/target/debug/fizzbuzz
   ```
6. Launches CodeLLDB with compiled binary

**Why this exception exists:**
- Test fixtures are often simple standalone programs for testing
- They are NOT listed in `Cargo.toml` as build targets
- Running `cargo build` would fail (target not found)
- Compiling with `rustc` works correctly

**Other exception patterns (future):**
- `examples/standalone/` - Standalone example files
- Any path with `/fixtures/` or `/standalone/`

---

## Configuration Options

### stopOnEntry

**Type:** `boolean`
**Default:** `false`

**Description:** Whether to stop at program entry point before executing any code.

**Example:**
```json
{
  "language": "rust",
  "program": "/workspace/src/main.rs",
  "stopOnEntry": true  ← Stop at entry
}
```

---

### args

**Type:** `array of strings`
**Default:** `[]`

**Description:** Command-line arguments to pass to the program.

**Example:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "args": ["arg1", "arg2", "--flag"]
}
```

Program receives: `argv = ["hello", "arg1", "arg2", "--flag"]`

---

### cwd

**Type:** `string` (path)
**Default:** `/workspace` (in Docker) or parent directory of source file

**Description:** Working directory for the debugged program. Affects relative file paths in the program.

**Example:**
```json
{
  "language": "rust",
  "program": "/workspace/src/main.rs",
  "cwd": "/workspace/data"
}
```

**Use case:** Program reads files from current directory

---

## Future Enhancements

### 1. Release Build Mode

**Proposed parameter:** `release: boolean`

**Current behavior:** Always debug build (`-C opt-level=0`)

**Proposed behavior:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "release": true  ← Compile with optimizations
}
```

Server would use:
- Standalone: `rustc -O`
- Cargo: `cargo build --release`

---

### 2. Custom Compilation Flags

**Proposed parameter:** `rustcFlags: array of strings`

**Proposed behavior:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "rustcFlags": ["-C", "target-cpu=native", "--edition=2021"]
}
```

Server would append flags to `rustc` command

---

### 3. Skip Compilation (Use Cached Binary)

**Proposed parameter:** `skipCompilation: boolean`

**Use case:** Source file unchanged, avoid recompiling

**Proposed behavior:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "skipCompilation": true  ← Use /tmp/debugger-mcp-XXXX/hello if exists
}
```

---

### 4. Custom Binary Output Path

**Proposed parameter:** `outputBinary: string`

**Use case:** Control where binary is written

**Proposed behavior:**
```json
{
  "language": "rust",
  "program": "/tmp/hello.rs",
  "outputBinary": "/tmp/my-hello"
}
```

---

## Implementation Details

### Project Detection Algorithm

```rust
pub fn detect_project_type(source_path: &str) -> Result<RustProjectType> {
    // 1. Walk up directory tree from source file
    let mut current = source.parent();
    while let Some(dir) = current {
        let manifest = dir.join("Cargo.toml");

        // 2. Found Cargo.toml?
        if manifest.exists() {
            // 3. Check if source is under Cargo subdir
            if let Ok(relative) = source.strip_prefix(dir) {
                let first_component = relative.components().next();

                // 4. Is it src/, tests/, examples/, benches/, or bin/?
                if first_component in ["src", "tests", "examples", "benches", "bin"] {

                    // 5. EXCEPTION: tests/fixtures/ are standalone
                    if relative.starts_with("tests/fixtures/") {
                        return SingleFile;  // Compile with rustc
                    }

                    return CargoProject;  // Compile with cargo build
                }
            }
        }

        current = dir.parent();
    }

    // 6. No Cargo.toml found → standalone file
    return SingleFile;
}
```

### Compilation Caching

- **Cargo projects:** Cargo handles caching automatically
- **Standalone files:** Server uses unique temp directory per session
  - `/tmp/debugger-mcp-{session-id}/binary-name`
  - Cleaned up when session disconnects
  - Each debug session gets fresh compilation

---

## Testing

### Test Coverage

| Scenario | Test File | Status |
|----------|-----------|--------|
| Standalone file | `rust_integration_test.rs:268-420` | ✅ Passing |
| Cargo project | `rust_integration_test.rs:268-420` | ✅ Passing |
| Pre-compiled binary | Manual test | ✅ Passing |
| Test fixtures | `rust_integration_test.rs:550-750` | ✅ Fixed |

### Known Issues

None currently

---

## Debugging This Adapter

### Enable Verbose Logging

```bash
RUST_LOG=debug /path/to/debugger_mcp serve
```

**Look for:**
- `🔍 [RUST] Detecting project type for: /path/to/file.rs`
- `📦 [RUST] Found Cargo project: /path/to/root`
- `📄 [RUST] Single file project: /path/to/file.rs`
- `✅ [RUST] Compilation successful: /path/to/binary`

### Common Issues

**Issue:** Breakpoint not verified

**Causes:**
1. Binary compiled without debug symbols
   - **Solution:** Check for `-g` flag in compilation logs
2. Source file path mismatch
   - **Solution:** Check CodeLLDB receives correct `cwd`
3. File detected as Cargo project but not in Cargo.toml
   - **Solution:** Check project detection logs

**Issue:** Compilation failed

**Causes:**
1. rustc not installed in Docker
   - **Solution:** Install rustc in Dockerfile
2. Source file has syntax errors
   - **Solution:** Check rustc stderr output
3. Missing dependencies (for Cargo projects)
   - **Solution:** Run `cargo build` manually to diagnose

---

## Related Documentation

- [DAP MCP Server Proposal](./DAP_MCP_SERVER_PROPOSAL.md)
- [Rust Compilation Flow Analysis](./rust-compilation-flow-analysis.md)
- [CodeLLDB Documentation](https://github.com/vadimcn/codelldb)

---

**Status:** Production Ready
**Tested Platforms:** Linux (Docker), macOS (local)
**Required Tools:** rustc 1.83.0+, CodeLLDB 1.11.0+
