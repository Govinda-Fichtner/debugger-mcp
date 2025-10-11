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
**Requires**: Go 1.21+ and Delve

```bash
# Install Go (example for Linux)
curl -L https://go.dev/dl/go1.21.0.linux-amd64.tar.gz | sudo tar -C /usr/local -xz
export PATH="/usr/local/go/bin:$PATH"

# Install Delve
go install github.com/go-delve/delve/cmd/dlv@latest
export PATH="$HOME/go/bin:$PATH"

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
  - Go 1.21 + Delve
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
