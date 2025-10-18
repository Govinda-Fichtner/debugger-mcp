# Rust Compilation Flow Analysis

**Date:** 2025-10-18
**Issue:** Rust breakpoint verification failure (verified: false)
**Root Cause Investigation:** Unclear whether MCP server or test should handle compilation

---

## Current Architecture

### Two Compilation Paths

```
┌─────────────────────────────────────────────────────────────────┐
│ Path 1: Test Compiles OUTSIDE Docker (Current Implementation)  │
└─────────────────────────────────────────────────────────────────┘

Integration Test (Host)
  ↓ compile_rust_fixture()
  ↓ rustc fizzbuzz.rs -g -o tests/fixtures/target/fizzbuzz
Binary: /workspace/tests/fixtures/target/fizzbuzz

  ↓ Claude Code Prompt
  ↓ "Start debugging session for /workspace/tests/fixtures/target/fizzbuzz"

MCP Server (Docker)
  ↓ Receives: program="/workspace/tests/fixtures/target/fizzbuzz"
  ↓ Detects: NOT .rs file → Uses as-is
  ↓ CodeLLDB: launch with binary_path
✅ Should work


┌─────────────────────────────────────────────────────────────────┐
│ Path 2: MCP Server Compiles INSIDE Docker (Designed Capability)│
└─────────────────────────────────────────────────────────────────┘

Integration Test (Host)
  ↓ No compilation
  ↓ Claude Code Prompt
  ↓ "Start debugging session for /workspace/tests/fixtures/fizzbuzz.rs"

MCP Server (Docker)
  ↓ Receives: program="/workspace/tests/fixtures/fizzbuzz.rs"
  ↓ Detects: .rs file → Auto-compile
  ↓ RustAdapter::compile() inside Docker
  ↓   → rustc fizzbuzz.rs -g -o /tmp/debugger-mcp-XXXX/fizzbuzz
  ↓ Binary: /tmp/debugger-mcp-XXXX/fizzbuzz
  ↓ CodeLLDB: launch with binary_path
✅ Should work (but requires rustc in Docker)
```

---

## What Actually Happened (CI Run #18615517518)

### Evidence from MCP Protocol Log

**Line 58:** Claude Code sent to MCP server:
```json
{
  "language": "rust",
  "program": "/workspace/tests/fixtures/fizzbuzz.rs",  ← SOURCE FILE
  "stopOnEntry": true
}
```

### Evidence from Test Output

**Line 381:** Binary was compiled BEFORE Claude Code ran:
```
🔨 Compiling Rust fixture...
   Source: /workspace/tests/fixtures/fizzbuzz.rs
   Output: /workspace/tests/fixtures/target/fizzbuzz
✅ Compilation successful
✅ Debug symbols verified (.debug_info section present)
```

**Line 614:** Prompt told Claude to debug the BINARY:
```rust
let prompt = format!(
    r#"...
Start debugging session for {}
...
    fizzbuzz_binary.display()  // /workspace/tests/fixtures/target/fizzbuzz
);
```

### The Mystery

1. ✅ Test compiled binary: `/workspace/tests/fixtures/target/fizzbuzz`
2. ✅ Prompt included binary path: `fizzbuzz_binary.display()`
3. ❌ Claude Code sent source path: `/workspace/tests/fixtures/fizzbuzz.rs`
4. ❌ No `[RUST]` compilation logs from MCP server
5. ❌ Breakpoint not verified

**Question:** Why did Claude Code send the source file path instead of the binary path from the prompt?

---

## Theory: Claude Code Inferred Source Path

Claude Code might have:
1. Read the prompt mentioning binary path
2. Looked at workspace files
3. Found `fizzbuzz.rs` source file
4. **Assumed it should debug the source** (like Python/Ruby)
5. Sent `.rs` path instead of binary path

This would explain:
- Why MCP server received `.rs` path
- Why there are no `[RUST]` compilation logs (never tried to compile)
- Why breakpoint verification failed (CodeLLDB got source file, not binary)

---

## Compilation Location Analysis

### Current Docker Image: Does it have rustc?

**Dockerfile.integration-tests inspection needed:**
```bash
# Check if rustc is installed in Docker image
docker run --rm debugger-mcp:integration-tests rustc --version
```

**If rustc IS installed:**
- Path 2 (MCP server compiles) SHOULD work
- Need to test why it didn't compile

**If rustc NOT installed:**
- Path 2 CANNOT work
- Must use Path 1 (test compiles, pass binary)
- Need to fix Claude Code prompt to ensure it uses binary path

---

## Test Scenarios

### Scenario A: Pass Source File to MCP Server
**Expectation:** MCP server compiles inside Docker

```bash
# In Docker container
echo '{"language":"rust","program":"/workspace/tests/fixtures/fizzbuzz.rs","stopOnEntry":true}' | \
  /workspace/target/release/debugger_mcp serve
```

**Expected behavior:**
1. MCP server detects `.rs` extension
2. Calls `RustAdapter::compile()`
3. Runs `rustc` inside Docker
4. Gets binary path: `/tmp/debugger-mcp-XXXX/fizzbuzz`
5. Launches CodeLLDB with binary

**Logs to check:**
```
🔨 [RUST] Compiling Rust source before debugging
📄 [RUST] Compiling single file with rustc
✅ [RUST] Compilation successful: /tmp/...
```

### Scenario B: Pass Pre-Compiled Binary to MCP Server
**Expectation:** MCP server uses binary as-is

```bash
# Compile outside Docker first
rustc tests/fixtures/fizzbuzz.rs -g -o tests/fixtures/target/fizzbuzz

# In Docker container
echo '{"language":"rust","program":"/workspace/tests/fixtures/target/fizzbuzz","stopOnEntry":true}' | \
  /workspace/target/release/debugger_mcp serve
```

**Expected behavior:**
1. MCP server detects NOT `.rs` extension
2. Uses path as-is: `/workspace/tests/fixtures/target/fizzbuzz`
3. Launches CodeLLDB with binary

**Logs to check:**
```
🎯 [RUST] Using pre-compiled binary: /workspace/tests/fixtures/target/fizzbuzz
```

---

## Key Questions to Answer

1. **Does Docker image have rustc?**
   - If YES: Why didn't MCP server compile when it received `.rs` path?
   - If NO: Tests MUST pass pre-compiled binary

2. **Why did Claude Code send source path instead of binary path?**
   - Prompt parsing issue?
   - Claude Code making assumptions about Rust?
   - Need to be more explicit in prompt?

3. **Where should compilation happen for reliability?**
   - **Option A:** Test compiles (host), pass binary to MCP
     - ✅ Works in CI (binary in workspace volume)
     - ✅ Doesn't require rustc in Docker
     - ❌ Less flexible (can't debug arbitrary .rs files)

   - **Option B:** MCP server compiles (Docker)
     - ✅ More flexible (debug any .rs file)
     - ✅ Matches Python/Ruby pattern (source files)
     - ❌ Requires rustc in Docker
     - ❌ Compilation happens on every debug session

   - **Option C:** Hybrid (recommended)
     - Accept BOTH .rs (compile in Docker) and binary paths
     - Tests can use either approach
     - Users can pass source or pre-compiled binary

---

## Recommended Solution

### 1. Verify Docker has rustc
```bash
docker run --rm debugger-mcp:integration-tests rustc --version
```

### 2. Test MCP server with both paths
Run Scenarios A and B above

### 3. Fix based on results

**If rustc available:**
- Check why compilation didn't happen
- Add verbose logging to compilation path
- May need to fix Claude Code prompt to be more explicit

**If rustc NOT available:**
- Add rustc to Docker image, OR
- Ensure tests always pass pre-compiled binary
- Make prompt crystal clear about using binary path

---

## Next Steps

1. ✅ Check if rustc is in Docker image
2. ✅ Test Scenario A (source file → MCP server compiles)
3. ✅ Test Scenario B (pre-compiled binary → MCP server uses as-is)
4. ✅ Analyze why Claude Code sent source path instead of binary path
5. ⏸️ Implement fix based on findings
