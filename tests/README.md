# Debugger MCP Tests

## Test Categories

### 1. Unit Tests (Automated)
Tests that don't require a running debugger. Run with:
```bash
cargo test --lib
cargo test --test test_rust_integration test_rust_adapter_metadata test_rust_adapter_command
```

### 2. Regression Tests (Manual via Claude Code)
Integration tests marked with `#[ignore]` that require full debugging environment:
- `test_rust_stack_trace_uses_correct_thread_id()` - Verifies thread ID fix
- `test_rust_evaluate_uses_watch_context()` - Verifies evaluation context fix

**Purpose**: These tests serve as:
1. **Documentation** of expected behavior and known bugs
2. **Regression protection** - will fail if bugs are reintroduced
3. **Verification** when run manually with Claude Code

**Why not automated?**: CodeLLDB cannot debug processes when running inside a Docker container that's also running the tests. These tests need the actual MCP server → CodeLLDB → target program flow.

**How to run**: Use Claude Code with the MCP server running in Docker to execute the debugging workflow described in each test.

## Running Tests

### Automated Tests (Unit tests)
```bash
./scripts/test-rust-docker.sh
```

### Manual Verification (Regression tests)
1. Start Docker container with MCP server
2. Use Claude Code to connect
3. Execute debugging commands that match the test scenarios
4. Verify behavior matches test expectations

See test results in `/home/vagrant/projects/fizzbuzz-rust-test/docs/` for manual test verification.
