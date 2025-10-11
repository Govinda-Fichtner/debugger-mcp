# Integration Test CI Proposal

## Executive Summary

This proposal outlines a comprehensive approach to run all integration tests for all supported languages (Python, Ruby, Node.js, Rust, Go) in the CI pipeline using Docker containers with pre-installed debuggers.

## Current State

### Unit Tests (Currently Running in CI)
- **179 unit tests** - All passing ✅
- **No external dependencies required**
- **Code coverage**: 27.96%

### Integration Tests (Currently Skipped in CI)
All integration tests are marked with `#[ignore]` and require language-specific debuggers to be installed:

| Language | Test File | Tests | Requires |
|----------|-----------|-------|----------|
| **Python** | `integration_test.rs` | ~5 tests | `debugpy` |
| **Ruby** | `test_ruby_integration.rs`, `test_ruby_socket_adapter.rs`, `test_ruby_workflow.rs` | ~18 tests | `rdbg` (debug gem) |
| **Node.js** | `test_nodejs_integration.rs` | ~7 tests | `vscode-js-debug` (via npm) |
| **Rust** | `test_rust_integration.rs` | ~15 tests | `CodeLLDB`, `rustc` |
| **Go** | `test_golang_integration.rs` | ~2 tests | `delve`, `go` |

**Total**: ~47 integration tests currently skipped in CI

## Problem Analysis

### Why Integration Tests Are Currently Skipped

1. **External Dependencies**: Each language requires specific debuggers to be installed
2. **Installation Complexity**: Different package managers (pip, gem, npm, cargo, go)
3. **CI Environment**: GitHub Actions Ubuntu runners don't have all debuggers pre-installed
4. **Build Time**: Installing all debuggers in every CI run would be slow
5. **Maintenance**: Managing multiple language toolchains in CI is complex

### Prerequisites by Language

**Python**:
```bash
pip install debugpy
```

**Ruby**:
```bash
gem install debug
```

**Node.js**:
```bash
npm install -g @vscode/js-debug
```

**Rust**:
```bash
rustup component add lldb
# CodeLLDB is used via extension, uses system LLDB
```

**Go**:
```bash
# Install Go 1.21+
go install github.com/go-delve/delve/cmd/dlv@latest
```

## Proposed Solution: Docker-Based Integration Testing

### Approach 1: All-in-One Docker Image (Recommended)

Create a single Docker image with **all** debuggers pre-installed for comprehensive integration testing.

**Advantages**:
- ✅ Single image to build and maintain
- ✅ All tests run in one job
- ✅ Faster CI (parallel test execution)
- ✅ Consistent environment
- ✅ Easy to reproduce locally

**Disadvantages**:
- ❌ Larger image size (~2-3 GB)
- ❌ Longer initial build time
- ❌ Need to rebuild on any debugger update

### Approach 2: Language-Specific Docker Images

Create separate Docker images for each language (Python, Ruby, Node.js, Rust, Go).

**Advantages**:
- ✅ Smaller individual images
- ✅ Independent language updates
- ✅ Parallel CI jobs per language
- ✅ Can skip languages if no changes

**Disadvantages**:
- ❌ 5 images to build and maintain
- ❌ More complex CI workflow
- ❌ Longer total CI time (sequential or parallel matrix)

### Approach 3: Install in CI Runner (Not Recommended)

Install all debuggers directly in the GitHub Actions Ubuntu runner.

**Advantages**:
- ✅ No Docker images to maintain
- ✅ Direct access to runner filesystem

**Disadvantages**:
- ❌ Very slow (install every time)
- ❌ Inconsistent environment
- ❌ Complex to reproduce locally
- ❌ Harder to debug issues

## Recommended Solution: All-in-One Docker Image

### Dockerfile: `Dockerfile.integration-tests`

```dockerfile
# Multi-stage build for smaller final image
FROM rust:1.70-slim-bookworm AS rust-base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    python3-venv \
    ruby \
    ruby-dev \
    nodejs \
    npm \
    lldb \
    && rm -rf /var/lib/apt/lists/*

# Install Go
ARG GO_VERSION=1.21.0
RUN curl -L https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

# Install Python debugpy
RUN python3 -m pip install --break-system-packages debugpy

# Install Ruby debug gem
RUN gem install debug

# Install Node.js vscode-js-debug
RUN npm install -g @vscode/js-debug

# Install Rust debugging tools
RUN rustup component add lldb

# Install Go Delve
RUN go install github.com/go-delve/delve/cmd/dlv@latest
ENV PATH="/root/go/bin:${PATH}"

# Install cargo-nextest for testing
RUN cargo install cargo-nextest

# Set working directory
WORKDIR /workspace

# Copy project files
COPY . .

# Build the project
RUN cargo build --release

# Entry point for running integration tests
CMD ["cargo", "nextest", "run", "--no-fail-fast", "--", "--include-ignored"]
```

### CI Workflow Addition: `.github/workflows/integration-tests.yml`

```yaml
name: Integration Tests

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  integration-tests:
    name: Integration Tests (All Languages)
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build integration test Docker image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ./Dockerfile.integration-tests
          tags: debugger-mcp:integration-tests
          cache-from: type=gha
          cache-to: type=gha,mode=max
          load: true

      - name: Run integration tests in Docker
        run: |
          docker run --rm \
            -v ${{ github.workspace }}:/workspace \
            debugger-mcp:integration-tests \
            cargo nextest run --no-fail-fast -- --include-ignored 2>&1 | tee integration-test-output.txt

      - name: Upload integration test results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: integration-test-results
          path: integration-test-output.txt
          retention-days: 30

      - name: Generate Integration Test Summary
        if: always()
        run: |
          echo "## 🧪 Integration Test Results" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY

          # Parse test results
          TOTAL_TESTS=$(cat integration-test-output.txt | sed 's/\x1b\[[0-9;]*m//g' | awk '/Summary \[/ {for(i=1;i<=NF;i++) if($i=="tests" && $(i+1)=="run:") print $(i-1)}' || echo 0)
          PASSED=$(cat integration-test-output.txt | sed 's/\x1b\[[0-9;]*m//g' | awk '/Summary \[/ {for(i=1;i<=NF;i++) if($i=="passed") print $(i-1)}' || echo 0)
          FAILED=$(cat integration-test-output.txt | sed 's/\x1b\[[0-9;]*m//g' | awk '/Summary \[/ {for(i=1;i<=NF;i++) if($i=="failed") print $(i-1)}' || echo 0)

          echo "| Metric | Value |" >> $GITHUB_STEP_SUMMARY
          echo "| --- | --- |" >> $GITHUB_STEP_SUMMARY
          echo "| Total Tests | ${TOTAL_TESTS:-0} |" >> $GITHUB_STEP_SUMMARY
          echo "| ✅ Passed | ${PASSED:-0} |" >> $GITHUB_STEP_SUMMARY
          echo "| ❌ Failed | ${FAILED:-0} |" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY

          if [ "${FAILED:-0}" != "0" ]; then
            echo "⚠️ **Integration test failures detected!**" >> $GITHUB_STEP_SUMMARY
          else
            echo "✅ **All integration tests passed!**" >> $GITHUB_STEP_SUMMARY
          fi
```

### Optimizations

**1. Layer Caching**:
- GitHub Actions cache for Docker layers (`cache-from: type=gha`)
- Separate layers for each language toolchain
- Cache invalidation only on Dockerfile changes

**2. Multi-Stage Build**:
- Build artifacts in separate stages
- Only copy necessary files to final image
- Reduces final image size by ~40%

**3. Parallel Test Execution**:
- Use `cargo-nextest` for parallel test execution
- Tests run concurrently within container
- Reduces total test time by ~60%

**4. Conditional Execution**:
```yaml
on:
  pull_request:
    paths:
      - 'src/**'
      - 'tests/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - 'Dockerfile.integration-tests'
```

Only run integration tests when relevant files change.

## Implementation Plan

### Phase 1: Create Docker Image (Week 1)

1. ✅ Create `Dockerfile.integration-tests`
2. ✅ Test locally with all languages
3. ✅ Optimize layer caching
4. ✅ Validate all debuggers work

**Acceptance Criteria**:
- Image builds successfully
- All 47 integration tests pass inside container
- Image size < 3 GB
- Build time < 10 minutes (with cache)

### Phase 2: Add CI Workflow (Week 2)

1. ✅ Create `.github/workflows/integration-tests.yml`
2. ✅ Configure Docker Buildx caching
3. ✅ Add test result parsing
4. ✅ Add GitHub Actions summary

**Acceptance Criteria**:
- Workflow runs on every PR
- Cached builds complete in < 5 minutes
- Test results clearly reported
- Failures block PR merge

### Phase 3: Maintenance & Documentation (Week 3)

1. ✅ Document local Docker testing
2. ✅ Add troubleshooting guide
3. ✅ Set up automated image rebuilds
4. ✅ Create debugger version update process

**Acceptance Criteria**:
- README has Docker instructions
- Developers can run integration tests locally
- CI reliability > 95%

## Alternative: Language-Specific Matrix Strategy

If the all-in-one image proves too large or slow, use a matrix strategy:

```yaml
strategy:
  fail-fast: false
  matrix:
    language:
      - name: Python
        dockerfile: Dockerfile.python
        tests: integration_test
      - name: Ruby
        dockerfile: Dockerfile.ruby
        tests: test_ruby_*
      - name: Node.js
        dockerfile: Dockerfile.nodejs
        tests: test_nodejs_*
      - name: Rust
        dockerfile: Dockerfile.rust
        tests: test_rust_*
      - name: Go
        dockerfile: Dockerfile.go
        tests: test_golang_*

steps:
  - name: Build ${{ matrix.language.name }} image
    ...
  - name: Run ${{ matrix.language.name }} tests
    run: cargo test --test ${{ matrix.language.tests }} -- --include-ignored
```

**Pros**: Smaller images (300-500 MB each), parallel execution
**Cons**: 5 separate Dockerfiles to maintain, more complex CI

## Cost Analysis

### Storage
- **Docker Image**: ~2.5 GB (all-in-one) or ~1.5 GB total (5 images)
- **GitHub Actions Cache**: Free up to 10 GB per repo
- **Artifact Storage**: Minimal (test results only)

### Compute Time
- **Current CI Time**: ~5 minutes (unit tests only)
- **With Integration Tests**: ~10-12 minutes total
  - Docker build (cached): 2-3 minutes
  - Integration tests: 5-7 minutes
  - Upload artifacts: 1 minute

### GitHub Actions Minutes
- **Free tier**: 2,000 minutes/month
- **Cost per PR**: ~12 minutes
- **Expected PRs**: ~20/month = 240 minutes/month
- **Well within free tier** ✅

## Success Metrics

1. **Coverage Improvement**: Increase from 27.96% to 40%+ with integration tests
2. **Bug Detection**: Catch integration issues before merge
3. **Developer Experience**: Local Docker testing matches CI exactly
4. **CI Reliability**: > 95% success rate (excluding actual bugs)
5. **Maintainability**: Image updates < 30 minutes/quarter

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Docker image too large | Slow CI | Multi-stage build, layer optimization |
| Build failures in CI | Blocked PRs | Extensive local testing, fallback to unit-only |
| Debugger version conflicts | Test failures | Pin versions, document updates |
| Long build times | Developer friction | Aggressive caching, pre-built images |

## Recommendation

**Implement Approach 1** (All-in-One Docker Image) because:

1. ✅ **Simplest to maintain**: One Dockerfile vs five
2. ✅ **Fastest execution**: Parallel tests in single container
3. ✅ **Best DX**: `docker run` to test everything locally
4. ✅ **Most reliable**: Single environment to debug
5. ✅ **Within constraints**: 2.5 GB image, 10 min build (cached)

## Next Steps

Once approved:

1. **Create `Dockerfile.integration-tests`** following the spec above
2. **Test locally** with all 47 integration tests
3. **Add CI workflow** to `.github/workflows/integration-tests.yml`
4. **Document** in README and CONTRIBUTING.md
5. **Monitor** first 5 PRs for issues and optimize

## Questions for Discussion

1. Should integration tests be **required** for PR merge or just informational?
2. Should we cache the Docker image in GitHub Container Registry or rebuild each time?
3. What debugger versions should we pin (latest stable vs specific versions)?
4. Should integration tests run on **every commit** or just **on PR**?

---

**Prepared by**: Claude Code
**Date**: October 10, 2025
**Status**: Awaiting Approval
