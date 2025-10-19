# Testing Guide

## Overview

The debugger MCP server has comprehensive test coverage across multiple levels:

### Test Categories

1. **Unit Tests** (193 tests) - No external dependencies
2. **Integration Tests** (5 languages) - End-to-end debugging validation
3. **Language-specific Tests** - Python, Ruby, Node.js, Go, Rust

---

## Running Tests

### Quick Start - All Unit Tests

```bash
cargo test
```

**Requirements**: None (Rust only)

**Time**: ~2 seconds

**Expected**: 193 passed

---

### Integration Tests

Integration tests validate end-to-end debugging across all supported languages.

#### Prerequisites

Each language requires its debugger installed:

**Python:**
```bash
pip install debugpy
```

**Ruby:**
```bash
gem install debug
```

**Node.js:**
```bash
npm install -g node-debug2
# or built-in Node.js debugger (no installation needed)
```

**Go:**
```bash
go install github.com/go-delve/delve/cmd/dlv@latest
```

**Rust:**
```bash
# CodeLLDB extension (VS Code) or lldb-vscode
```

#### Running Integration Tests

**All languages:**
```bash
cargo test --test '*integration*' -- --ignored
```

**Specific language:**
```bash
cargo test --test python_integration_test -- --ignored
cargo test --test ruby_integration_test -- --ignored
cargo test --test nodejs_integration_test -- --ignored
cargo test --test go_integration_test -- --ignored
cargo test --test rust_integration_test -- --ignored
```

#### Docker-based Integration Tests (Recommended)

Run tests in isolated containers:

```bash
# Build integration test image
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# Run tests inside Docker
docker run -it debugger-mcp:integration-tests \
  cargo test --test '*integration*' -- --ignored --nocapture
```

**Advantages:**
- ✅ No local debugger installation needed
- ✅ Clean environment
- ✅ Same setup as CI/CD
- ✅ All language debuggers pre-installed

---

## Testing Strategy

### Unit Testing Philosophy

**Red-Green-Refactor Cycle:**
1. **Red**: Write failing test first
2. **Green**: Write minimal code to pass
3. **Refactor**: Improve code quality
4. **Repeat**: For each feature

### Coverage Goals

**Current Status:**
- Unit Tests: 193 tests
- Coverage: ~85% (code coverage)

**Target Coverage:**
- 🎯 Unit Tests: ≥ 90%
- 🎯 Integration Tests: All critical paths
- 🎯 Coverage: ≥ 95%

### Test Isolation

Integration tests are marked `#[ignore]` because they require external tools:

```rust
#[tokio::test]
#[ignore] // Requires debugpy to be installed
async fn test_python_debugging() {
    // Test code...
}
```

**Benefits:**
- ✅ Unit tests always pass (no dependencies)
- ✅ Integration tests opt-in (`--ignored` flag)
- ✅ Clear separation of concerns
- ✅ Fast feedback loop

---

## Test Architecture

### Dependency Injection for Testability

The codebase uses trait-based dependency injection to enable testing without real processes:

```rust
// Production: Real process spawning
let client = DapClient::spawn("debugpy", &args).await?;

// Test: Mock transport
let client = DapClient::new_with_transport(mock_stdin, mock_stdout).await?;
```

### Mock DAP Transport

Using `mockall` for type-safe mocking:

```rust
use mockall::mock;

mock! {
    pub DapTransport {}

    #[async_trait]
    impl DapTransportTrait for DapTransport {
        async fn read_message(&mut self) -> Result<Message>;
        async fn write_message(&mut self, msg: &Message) -> Result<()>;
    }
}

#[tokio::test]
async fn test_transport() {
    let mut mock_transport = MockDapTransport::new();

    mock_transport
        .expect_read_message()
        .times(1)
        .returning(|| Ok(Message::Response(/* ... */)));

    let msg = mock_transport.read_message().await.unwrap();
    // Assertions...
}
```

### In-Memory Channels for Testing

Testing async I/O without real processes:

```rust
use tokio::io::duplex;

#[tokio::test]
async fn test_dap_client() {
    // Create bidirectional in-memory channels
    let (client_writer, adapter_reader) = duplex(4096);
    let (adapter_writer, client_reader) = duplex(4096);

    // Spawn simulated DAP adapter
    tokio::spawn(simulate_dap_adapter(adapter_reader, adapter_writer));

    // Create client with in-memory transport
    let client = DapClient::new_with_transport(client_writer, client_reader)
        .await
        .unwrap();

    // Test operations
    let capabilities = client.initialize("test-adapter").await.unwrap();
    assert!(capabilities.supports_configuration_done_request);
}
```

---

## Writing Tests

### Test Naming Convention

Use descriptive names that explain what is being tested:

```rust
// ✅ Good
#[test]
fn test_python_adapter_spawns_debugpy_with_correct_arguments() { }

// ❌ Bad
#[test]
fn test_python_1() { }
```

### Comprehensive Coverage Pattern

Test happy path, error cases, and edge cases:

```rust
// Happy path
#[test]
fn test_function_success() { }

// Error cases
#[test]
fn test_function_invalid_input() { }

#[test]
fn test_function_timeout() { }

#[test]
fn test_function_connection_failure() { }

// Edge cases
#[test]
fn test_function_empty_string() { }

#[test]
fn test_function_maximum_value() { }
```

### Test Organization

```rust
// src/module.rs
pub fn my_function() -> Result<()> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function_success() {
        let result = my_function();
        assert!(result.is_ok());
    }

    #[test]
    fn test_my_function_error_case() {
        // Test error handling
    }
}
```

---

## Integration Test Structure

### FizzBuzz Validation Scenario

The main validation test exercises all core debugging features:

```rust
#[tokio::test]
#[ignore]
async fn test_fizzbuzz_debugging() {
    // 1. Start debugger
    let session_id = start_debugger("python", "fizzbuzz.py").await?;

    // 2. Set breakpoint
    let bp = set_breakpoint(&session_id, "fizzbuzz.py", 3).await?;
    assert!(bp.verified);

    // 3. Continue and wait for breakpoint
    continue_execution(&session_id).await?;
    wait_for_stopped(&session_id, "breakpoint").await?;

    // 4. Inspect variables
    let n = evaluate(&session_id, "n").await?;
    assert_eq!(n.result, "1");

    // 5. Step through
    step_over(&session_id).await?;

    // 6. Continue to completion
    continue_execution(&session_id).await?;
    wait_for_terminated(&session_id).await?;
}
```

**Why FizzBuzz?**
- Simple algorithm everyone understands
- Exercises loops, conditionals, functions
- Tests breakpoints, stepping, evaluation
- Same test validates all 5 languages

### Test Results Schema

Integration tests generate `test-results.json`:

```json
{
  "test_run": {
    "language": "python",
    "timestamp": "2025-10-19T12:00:00Z",
    "overall_success": true
  },
  "operations": {
    "session_started": true,
    "breakpoint_set": true,
    "breakpoint_verified": true,
    "execution_continued": true,
    "stopped_at_breakpoint": true,
    "stack_trace_retrieved": true,
    "variable_evaluated": true,
    "session_disconnected": true
  },
  "errors": []
}
```

**Operations (SBCTED):**
- **S**ession Start
- **B**reakpoint Set
- **C**ontinue Execution
- **T**race Stack
- **E**valuate Expression
- **D**isconnect Session

---

## CI/CD Integration

### GitHub Actions

Integration tests run in CI via matrix strategy:

```yaml
strategy:
  matrix:
    language: [python, ruby, nodejs, go, rust]

steps:
  - name: Run ${{ matrix.language }} integration test
    run: |
      cargo test --test ${{ matrix.language }}_integration_test \
        -- --ignored --nocapture
```

See [CI Workflows Documentation](PROCESS_CI_WORKFLOWS.md) for complete CI setup.

### Pre-commit Hooks

Automated quality checks before commit:

```bash
# Install hooks
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Hooks run automatically on:
# - commit: Format, clippy, unit tests
# - push: All tests including integration tests
```

---

## Troubleshooting Tests

### Common Issues

**"debugpy: command not found"**
```bash
pip install debugpy
python -m debugpy --version
```

**"rdbg: command not found"**
```bash
gem install debug
rdbg --version
```

**"Failed to build gem native extension"**

Missing build tools:
```bash
# Alpine
apk add --no-cache build-base

# Debian/Ubuntu
apt-get install build-essential

# macOS
xcode-select --install
```

**Test timeout**

Increase timeout in test:
```rust
#[tokio::test(flavor = "multi_thread")]
#[timeout(Duration::from_secs(60))]
async fn test_long_running() { }
```

### Running Specific Tests

```bash
# Only unit tests (fast)
cargo test

# Only ignored tests (integration)
cargo test -- --ignored

# Everything (unit + integration)
cargo test -- --include-ignored

# Specific test with output
cargo test test_name -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4
```

### Test Debugging

```bash
# Run with verbose output
cargo test -- --nocapture

# Run with trace logging
RUST_LOG=trace cargo test

# Run single test with full backtrace
RUST_BACKTRACE=full cargo test test_name -- --nocapture
```

---

## Performance Benchmarks

### Expected Test Times

| Test Suite | Time | Notes |
|------------|------|-------|
| All unit tests | ~2s | No I/O |
| Python integration | ~5-10s | Spawns debugpy |
| Ruby integration | ~3-4s | Spawns rdbg |
| Node.js integration | ~5-8s | Spawns node debugger |
| Go integration | ~6-10s | Spawns delve |
| Rust integration | ~8-12s | Spawns CodeLLDB |
| **Total (all tests)** | ~30-50s | All 5 languages |

### Optimization Tips

1. **Parallel execution**: `cargo test -- --test-threads=4`
2. **Skip integration**: `cargo test` (default)
3. **Use Docker cache**: Pre-built images for faster runs
4. **Incremental builds**: Keep target/ directory

---

## Best Practices

### 1. Fast Unit Tests
- ✅ No I/O operations
- ✅ Mock external dependencies
- ✅ Run in parallel
- ✅ Always pass locally

### 2. Realistic Integration Tests
- ✅ Use real debuggers
- ✅ Test full workflows
- ✅ Mark with `#[ignore]`
- ✅ Document prerequisites

### 3. Test Maintainability
- ✅ Clear test names
- ✅ Test one thing per test
- ✅ Use helper functions for common setup
- ✅ Keep tests independent

### 4. Error Testing
- ✅ Test error paths explicitly
- ✅ Verify error messages
- ✅ Test recovery mechanisms
- ✅ Test edge cases

---

## Adding Tests for New Features

### Checklist

When adding new functionality:

1. ☐ Write unit tests first (TDD)
2. ☐ Mark external dependencies with `#[ignore]`
3. ☐ Document prerequisites
4. ☐ Update test scripts if needed
5. ☐ Run full test suite
6. ☐ Update this document

### Example: Adding New Language Support

```rust
// Unit test (no dependencies)
#[test]
fn test_java_adapter_command() {
    assert_eq!(JavaAdapter::command(), "java");
}

// Integration test (requires Java + debugger)
#[tokio::test]
#[ignore] // Requires Java and jdwp
async fn test_java_adapter_debug_session() {
    let session = JavaAdapter::spawn("Main.java", &[], true).await;
    assert!(session.is_ok());
}
```

Then update this guide with Java-specific setup instructions.

---

## Resources

### Documentation
- [Getting Started](CONTRIBUTING_GETTING_STARTED.md) - Development setup
- [CI Workflows](PROCESS_CI_WORKFLOWS.md) - CI/CD pipeline details
- [Architecture](ARCHITECTURE_PROPOSAL.md) - System design

### External Resources
- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Mockall Documentation](https://docs.rs/mockall/)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)

---

## Summary

**Quick Commands:**
```bash
# Unit tests (fast, always work)
cargo test

# Integration tests (requires debuggers)
cargo test -- --ignored

# Docker integration tests (recommended)
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .
docker run -it debugger-mcp:integration-tests cargo test -- --ignored

# Pre-commit checks
pre-commit run --all-files
```

**Coverage Target:** 95%+ with comprehensive unit and integration tests

**Philosophy:** Write tests first, keep them fast, make them reliable

---

*Last Updated: 2025-10-19*
