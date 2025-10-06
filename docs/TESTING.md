# Testing Guide

## Test Suite Overview

The debugger MCP server has comprehensive test coverage across multiple levels:

### Test Categories

1. **Unit Tests** (143 tests) - No external dependencies
2. **Integration Tests** (9 tests) - Require Docker
3. **Ruby Socket Tests** (15 tests) - 9 unit + 6 integration

**Total**: 167 tests

## Running Tests

### Quick Start - All Unit Tests

```bash
cargo test
```

**Requirements**: None (Rust only)

**Time**: ~2 seconds

**Expected**: 152 passed, 15 ignored

---

### Ruby Socket Tests - Unit Only

```bash
cargo test --test test_ruby_socket_adapter
```

**Requirements**: None (Rust only)

**Time**: ~0.5 seconds

**Expected**: 9 passed, 6 ignored

**Tests**:
- Socket helper functions (port finding, retry, timeout)
- DapTransport socket mode
- Ruby adapter configuration

---

### Ruby Socket Tests - Integration (Requires rdbg)

#### Prerequisites

The 6 Ruby integration tests require:

1. **Ruby** ≥ 3.0
2. **rdbg** (debug gem ≥ 1.0.0)
3. **Build tools** (for gem compilation)

#### Option 1: Automated Setup (Recommended)

Run the provided test script:

```bash
./scripts/test_ruby_integration.sh
```

**What it does**:
1. Checks if running in Ruby container
2. Installs build dependencies
3. Installs debug gem
4. Runs integration tests

**Output**:
```
✅ Ruby and rdbg installed
rdbg 1.11.0

running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored
```

#### Option 2: Docker (Isolated Environment)

Run tests in Ruby container:

```bash
docker run --rm \
  -v $(pwd):/app \
  -w /app \
  ruby:3.3-alpine \
  sh /app/scripts/test_ruby_integration.sh
```

**Advantages**:
- ✅ No local Ruby installation needed
- ✅ Clean environment
- ✅ Same setup as CI/CD

**Time**: ~20 seconds (includes gem installation)

#### Option 3: Manual Setup

If you have Ruby installed locally:

```bash
# 1. Install debug gem
gem install debug

# 2. Verify installation
rdbg --version

# 3. Run tests
cargo test --test test_ruby_socket_adapter -- --ignored
```

**Requirements**:
- Ruby ≥ 3.0
- Build tools (gcc, make) for native extensions

---

### Python Integration Tests

Some tests require Python and debugpy:

```bash
cargo test -- --ignored
```

**Prerequisites**:
```bash
pip install debugpy
```

---

## Test Prerequisites Summary

| Test Suite | Requirements | How to Run |
|------------|--------------|------------|
| **All Unit Tests** | Rust only | `cargo test` |
| **Ruby Socket (Unit)** | Rust only | `cargo test --test test_ruby_socket_adapter` |
| **Ruby Socket (Integration)** | Ruby + rdbg | `./scripts/test_ruby_integration.sh` |
| **Python Integration** | Python + debugpy | `cargo test -- --ignored` |

## Automated Prerequisite Checking

### Test Script Features

The `scripts/test_ruby_integration.sh` script automatically:

1. **Detects environment** - Checks if Ruby is available
2. **Installs dependencies** - Adds build tools if needed
3. **Installs rdbg** - Via `gem install debug`
4. **Verifies installation** - Shows rdbg version
5. **Runs tests** - With proper flags

### Creating Similar Scripts

For Python tests, you can create `scripts/test_python_integration.sh`:

```bash
#!/bin/bash
set -e

echo "=== Checking Python installation ==="
python3 --version || { echo "❌ Python not found"; exit 1; }

echo "=== Installing debugpy ==="
pip install debugpy > /dev/null 2>&1

echo "✅ Python and debugpy installed"
echo ""

echo "=== Running Python integration tests ==="
cargo test -- --ignored
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test

  ruby-integration:
    runs-on: ubuntu-latest
    container: ruby:3.3-alpine
    steps:
      - uses: actions/checkout@v3
      - name: Install dependencies
        run: apk add --no-cache build-base rust cargo
      - name: Install rdbg
        run: gem install debug
      - name: Run Ruby tests
        run: cargo test --test test_ruby_socket_adapter -- --ignored

  python-integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      - name: Install debugpy
        run: pip install debugpy
      - name: Run Python tests
        run: cargo test -- --ignored
```

## Test Isolation

### Why Integration Tests are Marked `#[ignore]`

Integration tests require external tools (rdbg, debugpy) that may not be installed:

```rust
#[tokio::test]
#[ignore] // Requires rdbg to be installed
async fn test_ruby_adapter_spawn_real_rdbg() {
    // Test code...
}
```

**Benefits**:
- ✅ Unit tests always pass (no dependencies)
- ✅ Integration tests opt-in (`--ignored` flag)
- ✅ Clear separation of concerns
- ✅ Fast feedback loop

### Running Specific Test Groups

```bash
# Only unit tests (fast)
cargo test

# Only ignored tests (integration)
cargo test -- --ignored

# Everything (unit + integration)
cargo test -- --include-ignored

# Specific test
cargo test test_ruby_adapter_spawn_real_rdbg -- --ignored
```

## Troubleshooting

### "rdbg: command not found"

**Solution**:
```bash
gem install debug
```

**Verify**:
```bash
rdbg --version
```

### "Failed to build gem native extension"

**Cause**: Missing build tools

**Solution (Alpine)**:
```bash
apk add --no-cache build-base
```

**Solution (Debian/Ubuntu)**:
```bash
apt-get install build-essential
```

**Solution (macOS)**:
```bash
xcode-select --install
```

### "debugpy not found"

**Solution**:
```bash
pip install debugpy
```

**Verify**:
```bash
python -m debugpy --version
```

### Docker Socket Permission Denied

**Cause**: Docker daemon not accessible

**Solution**:
```bash
# Add user to docker group
sudo usermod -aG docker $USER
# Logout and login again
```

## Performance Benchmarks

### Expected Test Times

| Test Suite | Time | Notes |
|------------|------|-------|
| All unit tests | ~2s | No I/O |
| Ruby socket (unit) | ~0.5s | No external deps |
| Ruby socket (integration) | ~3-4s | Spawns rdbg 6 times |
| Python integration | ~5-10s | Spawns debugpy |
| **Total** | ~10-16s | All tests |

### Optimization Tips

1. **Parallel execution**: `cargo test -- --test-threads=4`
2. **Skip integration**: `cargo test` (default)
3. **Incremental builds**: `cargo test --no-fail-fast`

## Test Coverage Goals

### Current Status (as of 2025-10-07)

✅ **Unit Tests**: 152 tests
✅ **Integration Tests**: 15 tests
✅ **Coverage**: ~85% (estimated)

### Target Coverage

🎯 **Unit Tests**: ≥ 90%
🎯 **Integration Tests**: All critical paths
🎯 **Coverage**: ≥ 95%

## Adding New Tests

### Checklist for New Features

When adding new functionality:

1. ☐ Write unit tests first (TDD)
2. ☐ Mark external dependencies with `#[ignore]`
3. ☐ Document prerequisites
4. ☐ Update test scripts if needed
5. ☐ Run full test suite
6. ☐ Update this document

### Example: Adding Node.js Support

```rust
// tests/test_node_adapter.rs

// Unit test (no dependencies)
#[test]
fn test_node_adapter_command() {
    assert_eq!(NodeAdapter::command(), "node");
}

// Integration test (requires Node.js)
#[tokio::test]
#[ignore] // Requires Node.js and node-inspect
async fn test_node_adapter_spawn() {
    // Prerequisites documented in TESTING.md
    let session = NodeAdapter::spawn("app.js", &[], true).await;
    assert!(session.is_ok());
}
```

Then update `TESTING.md`:

```markdown
### Node.js Integration Tests

**Prerequisites**:
- Node.js ≥ 14
- node-inspect (built-in)

**Run**:
```bash
cargo test --test test_node_adapter -- --ignored
```
```

## Best Practices

### 1. Fast Unit Tests
- ✅ No I/O operations
- ✅ Mock external dependencies
- ✅ Run in parallel
- ✅ Always pass

### 2. Realistic Integration Tests
- ✅ Use real tools (rdbg, debugpy)
- ✅ Test full workflows
- ✅ Mark with `#[ignore]`
- ✅ Document prerequisites

### 3. Clear Test Names
```rust
// ✅ Good
test_ruby_adapter_spawn_with_args_passes_arguments_correctly()

// ❌ Bad
test_ruby_1()
```

### 4. Comprehensive Coverage
```rust
// Test the happy path
test_function_success()

// Test error cases
test_function_invalid_input()
test_function_timeout()
test_function_connection_failure()

// Test edge cases
test_function_empty_string()
test_function_maximum_value()
```

## Summary

**Quick Start**:
```bash
# Unit tests (always work)
cargo test

# Ruby integration (needs rdbg)
./scripts/test_ruby_integration.sh

# Everything
cargo test -- --include-ignored
```

**Prerequisites**:
- Rust: Always required
- Ruby + rdbg: For Ruby integration tests
- Python + debugpy: For Python integration tests
- Docker: Alternative to local installation

**Resources**:
- Unit tests: No setup needed
- Integration tests: Use provided scripts
- CI/CD: See GitHub Actions example
- Troubleshooting: Check common errors section

---

**For detailed test results, see**: `docs/RUBY_SOCKET_TEST_RESULTS.md`
