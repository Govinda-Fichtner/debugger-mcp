# Integration Tests Guide

This guide explains how to run integration tests that require language-specific debuggers (Python, Ruby, Node.js, Rust, Go).

## Quick Start with Docker (Recommended)

The easiest way to run integration tests is using our pre-configured Docker image:

```bash
# Build the integration test image
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# Run all integration tests
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests

# Run with coverage report
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests \
  cargo tarpaulin --all --ignore-panics \
  --exclude-files 'tests/bin/*' \
  --out Html --output-dir coverage-integration \
  -- --include-ignored

# View coverage report
open coverage-integration/index.html  # macOS
xdg-open coverage-integration/index.html  # Linux
```

## Integration Tests by Language

### Python (debugpy)

**Tests**: `tests/integration_test.rs` (~5 tests)
**Requires**: Python 3.8+ and debugpy

```bash
# Install debugpy
pip install debugpy

# Run Python integration tests
cargo test --test integration_test -- --include-ignored
```

### Ruby (rdbg)

**Tests**: `tests/test_ruby_*.rs` (~18 tests)
**Requires**: Ruby 3.0+ and debug gem

```bash
# Install debug gem
gem install debug

# Run Ruby integration tests
cargo test --test test_ruby_integration -- --include-ignored
cargo test --test test_ruby_socket_adapter -- --include-ignored
cargo test --test test_ruby_workflow -- --include-ignored
```

### Node.js (vscode-js-debug)

**Tests**: `tests/test_nodejs_integration.rs` (~7 tests)
**Requires**: Node.js 14+ and vscode-js-debug

```bash
# Install vscode-js-debug
npm install -g @vscode/js-debug

# Run Node.js integration tests
cargo test --test test_nodejs_integration -- --include-ignored
```

### Rust (CodeLLDB)

**Tests**: `tests/test_rust_integration.rs` (~15 tests)
**Requires**: rustc and lldb

```bash
# Install LLDB component
rustup component add lldb

# Run Rust integration tests
cargo test --test test_rust_integration -- --include-ignored
```

### Go (Delve)

**Tests**: `tests/test_golang_integration.rs` (~2 tests)
**Requires**: Go 1.22+ and Delve 1.20+

**Important**: Delve 1.25+ requires Go 1.22 or higher. Using Go 1.21 will result in:
```
Failed to launch: Go version go1.21.0 is too old for this version of Delve (minimum supported version 1.22)
```

```bash
# Install Go 1.23.1 (example for Linux amd64)
curl -L https://go.dev/dl/go1.23.1.linux-amd64.tar.gz | sudo tar -C /usr/local -xz
export PATH="/usr/local/go/bin:$PATH"

# For ARM64, use:
# curl -L https://go.dev/dl/go1.23.1.linux-arm64.tar.gz | sudo tar -C /usr/local -xz

# Install Delve
go install github.com/go-delve/delve/cmd/dlv@latest
export PATH="$HOME/go/bin:$PATH"

# Verify versions
go version  # Should show go1.22 or higher
dlv version # Should show Delve 1.20.0 or higher

# Run Go integration tests
cargo test --test test_golang_integration -- --include-ignored
```

## Running All Integration Tests (Native)

If you have all debuggers installed:

```bash
# Run ALL tests including integration tests
cargo test -- --include-ignored

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --all --ignore-panics \
  --exclude-files 'tests/bin/*' \
  --out Html --output-dir coverage-all \
  -- --include-ignored
```

## CI/CD Integration

### GitHub Actions

Integration tests run automatically in CI using the Docker image:

- **Workflow**: `.github/workflows/integration-tests.yml`
- **Trigger**: On PR to main, changes to src/tests/Cargo files
- **Duration**: ~10-12 minutes
- **Coverage**: Combined with unit tests, uploaded to Codecov

### Local CI Simulation

Test the CI workflow locally:

```bash
# Build Docker image (same as CI)
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# Run tests (same as CI)
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests \
  cargo nextest run --no-fail-fast -- --include-ignored
```

## Coverage Reporting

### Combined Coverage (Unit + Integration)

The integration test workflow generates combined coverage:

```bash
# Generate combined coverage
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests

# Coverage files are in coverage-integration/
ls coverage-integration/
# cobertura.xml    # For Codecov
# tarpaulin-report.json
# index.html       # Human-readable report
```

### Codecov Integration

Both unit and integration tests upload to Codecov with different flags:

- **Unit Tests**: Flag `unit-tests` (from `ci.yml`)
- **Integration Tests**: Flag `integration-tests` (from `integration-tests.yml`)
- **Combined**: Codecov automatically merges both reports

View combined coverage at: `https://codecov.io/gh/YOUR_ORG/debugger-mcp`

## Troubleshooting

### Docker Build Fails

```bash
# Clear Docker build cache
docker builder prune -a

# Rebuild without cache
docker build --no-cache -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .
```

### Tests Timeout

Some integration tests may timeout if debuggers are slow to start:

```bash
# Increase timeout (default is 60s)
RUST_TEST_TIMEOUT=120 cargo test -- --include-ignored
```

### Permission Denied Errors

If you get permission errors in Docker:

```bash
# Run with user permissions
docker run --rm -u $(id -u):$(id -g) -v $(pwd):/workspace debugger-mcp:integration-tests
```

### Specific Language Failures

Test individual languages to isolate issues:

```bash
# Test only Python
cargo test --test integration_test -- --include-ignored

# Test only Ruby
cargo test --test test_ruby_integration -- --include-ignored

# Test only Node.js
cargo test --test test_nodejs_integration -- --include-ignored

# Test only Rust
cargo test --test test_rust_integration -- --include-ignored

# Test only Go
cargo test --test test_golang_integration -- --include-ignored
```

### Reproducing CI Failures Locally

When CI integration tests fail, reproduce locally using Docker for better visibility:

**For AI client tests (Claude Code / Codex):**

```bash
# 1. Build Docker image (matches CI environment exactly)
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# 2. Build release binary (for Claude Code tests)
docker run --rm \
  -v $(pwd):/workspace \
  debugger-mcp:integration-tests \
  cargo build --release

# 3. Run specific AI client test
docker run --rm \
  -v $(pwd):/workspace \
  -e RUST_BACKTRACE=1 \
  -e OPENAI_API_KEY="sk-..." \
  debugger-mcp:integration-tests \
  cargo test --test python_integration_test test_python_codex_code_integration \
  -- --include-ignored --nocapture
```

**Environment variables needed:**
- `OPENAI_API_KEY` - For Codex tests (get from OpenAI dashboard)
- `ANTHROPIC_API_KEY` - For Claude Code tests (get from Anthropic Console)
- `RUST_BACKTRACE=1` - Show Rust stack traces on panic

**Check test output files:**
Tests create files in workspace root:
- `test-results.json` - Operation success/failure (8 operations validated)
- `mcp_protocol_log.md` - Full MCP communication transcript
- `{language}-{ai_client}-test.txt` - Complete test output capture

**Example: Reproducing Python Codex test failure:**

```bash
# Run test and capture output
docker run --rm \
  -v $(pwd):/workspace \
  -e RUST_BACKTRACE=1 \
  -e OPENAI_API_KEY="sk-proj-..." \
  debugger-mcp:integration-tests \
  cargo test --test python_integration_test test_python_codex_code_integration \
  -- --include-ignored --nocapture > python-codex-debug.txt 2>&1

# Check test results
cat test-results.json | jq .

# Review full output
less python-codex-debug.txt
```

### Common AI Client Test Failures

#### Timeout (180s) with Zero Output

**Symptom:** Test hangs at "Step 8: Running Codex..." or "Step 8: Running Claude Code..." with no output

**Cause:** Fast-completing programs (like fizzbuzz) finish execution before breakpoints can be set

**Root Cause:** With `stopOnEntry: false`, the program runs to completion in milliseconds, terminating the debug session before the AI can set breakpoints. The AI then enters an infinite retry loop trying to set breakpoints on a dead session.

**Solution:** Ensure `stopOnEntry: true` in test prompt (around lines 1000-1200 in test files)

```rust
// ❌ WRONG - program completes instantly, session terminates
"stopOnEntry": false

// ✅ CORRECT - pauses at entry point, allowing breakpoint setup
"stopOnEntry": true
```

**Files to check:**
- `tests/integration/lang/python_integration_test.rs` (line 1097)
- `tests/integration/lang/ruby_integration_test.rs` (line 1076)
- `tests/integration/lang/nodejs_integration_test.rs` (line 1003)
- `tests/integration/lang/go_integration_test.rs` (line 1110)
- `tests/integration/lang/rust_integration_test.rs` (line 1202)

**Fix applied in commit `cfc4004`:** Python and Ruby tests had `stopOnEntry: false`, causing 180s timeouts. Changed to `true`, now passing in 30-40s.

---

#### Shows 1 Operation Instead of 8

**Symptom:** CI test summary shows `1 total operation` instead of `8 (SBCTED)`

**Causes:**
1. **Missing `test-results.json`** - Test didn't complete, AI crashed or timed out
2. **AI authentication failed** - Invalid API key (OPENAI_API_KEY or ANTHROPIC_API_KEY)
3. **MCP server crashed early** - Check for panics in test output
4. **Test harness error** - Rust test wrapper failed before spawning AI

**Debug steps:**

```bash
# 1. Check if test-results.json exists
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests \
  ls -la test-results.json

# 2. Run with full output to see authentication errors
docker run --rm \
  -v $(pwd):/workspace \
  -e RUST_BACKTRACE=1 \
  -e OPENAI_API_KEY="sk-..." \
  debugger-mcp:integration-tests \
  cargo test --test python_integration_test test_python_codex_code_integration \
  -- --include-ignored --nocapture > full-debug.txt 2>&1

# 3. Search for specific error patterns
grep -i "authentication\|api key\|invalid" full-debug.txt
grep -i "panic\|error\|failed" full-debug.txt | head -20

# 4. Check MCP server logs for crashes
grep "ERROR\|WARN" mcp_protocol_log.md
```

**Common patterns:**
- `Error: API key invalid` → Check OPENAI_API_KEY format (should start with `sk-proj-`)
- `Authentication failed` → API key expired or deactivated
- `thread 'main' panicked at` → MCP server crash (check stack trace)

---

#### Variable Evaluation Fails in Compiled Languages

**Symptom:** Codex reports "variable not in scope" or evaluation returns error for Go/Rust

**Cause:** Compiler optimizations can remove variables or delay their initialization until after the breakpoint line

**Expected behavior:** AI should automatically use `debugger_step_over` to advance execution to where the variable is accessible

**Example (Go - variable `i` not in scope at line 5):**
```
AI: debugger_evaluate(expression="i")  ← Fails
AI: debugger_step_over()               ← Adapts strategy
AI: debugger_evaluate(expression="i")  ← Now succeeds, i=1
```

**Not a test failure if:** AI successfully adapts and completes all 8 operations (using stepping to work around optimization)

**Observed in:**
- **Go (Delve)**: Variable `i` often requires stepping to bring into scope
- **Rust (CodeLLDB)**: Variables optimized out at function entry, accessible after first step
- **Python/Ruby/Node.js**: Interpreted languages rarely have this issue

**Test still passes:** As long as `variable_evaluated: true` in final `test-results.json`

---

#### Differences Between Claude Code and Codex

| Aspect | Claude Code | Codex (OpenAI) |
|--------|-------------|----------------|
| **Retry strategy** | Less aggressive, clear errors | More aggressive retries (may timeout) |
| **Error handling** | Detailed error messages | May retry silently on errors |
| **Variable evaluation** | Direct evaluation | May require stepping first |
| **Typical duration** | 30-90 seconds | 30-270 seconds (Go slowest) |
| **Reasoning output** | Concise | Verbose troubleshooting |
| **Breakpoint strategy** | Sets once | May retry multiple times |

**Key insight:** Both clients should pass, but execution times and strategies differ. Codex typically takes longer due to more extensive reasoning and retry logic.

**Expected test durations (from successful runs):**

| Language | Claude Code | Codex | Notes |
|----------|-------------|-------|-------|
| Python   | ~35s | ~36s | Fastest (interpreted) |
| Ruby     | ~30s | ~28s | Fastest (interpreted) |
| Node.js  | ~60s | ~189s | Codex does extensive breakpoint retries |
| Rust     | ~70s | ~121s | Variable stepping needed |
| Go       | ~90s | ~271s | Slowest (Delve startup + source file fallback) |

---

#### MCP Protocol Errors (Non-Blocking)

**Common error in logs:**
```
ERROR codex_mcp_client::mcp_client: failed to deserialize JSONRPCMessage:
data did not match any variant of untagged enum JSONRPCMessage;
line = {"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Notifications not yet supported"}}
```

**Impact:** ✅ **None** - Tests complete successfully despite this error

**Cause:** MCP server sends notification about unsupported feature; client logs as error but continues

**Action:** Ignore this error unless tests actually fail

---

## Docker Image Details

### Image Contents

The integration test Docker image (`Dockerfile.integration-tests`) includes:

- **Base**: `rust:1.70-slim-bookworm`
- **Size**: ~2.5 GB
- **Languages**:
  - Python 3.11 + debugpy
  - Ruby 3.1 + debug gem
  - Node.js 18 + @vscode/js-debug
  - Rust (rustc) + LLDB
  - Go 1.23.1 + Delve 1.20+ (requires Go 1.22+ for Delve 1.25+)
- **Tools**:
  - cargo-nextest (parallel test execution)
  - cargo-tarpaulin (code coverage)

### Building Custom Image

Customize the Dockerfile for your needs:

```dockerfile
# Add specific debugger version
RUN python3 -m pip install debugpy==1.8.0

# Add additional tools
RUN apt-get update && apt-get install -y vim gdb

# Change Go version
ARG GO_VERSION=1.22.0
```

Rebuild:

```bash
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .
```

## Best Practices

### 1. Test Locally Before Pushing

```bash
# Quick check: unit tests only
cargo test

# Full check: with integration tests
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests
```

### 2. Use Docker for Consistency

Always use Docker to ensure your environment matches CI:

```bash
# Don't: Run tests natively (may have different debugger versions)
cargo test -- --include-ignored

# Do: Run tests in Docker (same as CI)
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests
```

### 3. Monitor Coverage

Check that new code is covered by integration tests:

```bash
# Generate HTML coverage report
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests

# Open report and check your changes
open coverage-integration/index.html
```

### 4. Debug Failed Tests

When a test fails, run it individually with logs:

```bash
# Run single test with full output
cargo test test_python_fizzbuzz_debugging -- --include-ignored --nocapture

# Or in Docker
docker run --rm -it -v $(pwd):/workspace debugger-mcp:integration-tests \
  cargo test test_python_fizzbuzz_debugging -- --include-ignored --nocapture
```

## Performance Tips

### Speed Up Docker Builds

```bash
# Use BuildKit for better caching
export DOCKER_BUILDKIT=1
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# Use multi-stage builds (already configured in Dockerfile)
# Layers are cached independently
```

### Speed Up Test Execution

```bash
# Use nextest for parallel execution
docker run --rm -v $(pwd):/workspace debugger-mcp:integration-tests \
  cargo nextest run -- --include-ignored

# Skip slow tests during development
cargo test -- --include-ignored --skip slow_test
```

## Additional Resources

- [DAP MCP Server Proposal](DAP_MCP_SERVER_PROPOSAL.md) - Architecture overview
- [Integration Test CI Proposal](INTEGRATION_TEST_CI_PROPOSAL.md) - Detailed CI design
- [Codecov Documentation](https://docs.codecov.com/) - Coverage reporting
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) - Rust coverage tool
- [cargo-nextest](https://nexte.st/) - Fast test runner

---

**Questions?** Open an issue or check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
