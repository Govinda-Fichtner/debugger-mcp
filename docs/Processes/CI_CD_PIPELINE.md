# CI/CD Pipeline Architecture

This document describes the complete CI/CD pipeline for the debugger_mcp project, including unit tests, integration tests, code coverage, security scanning, and multi-platform builds.

## Table of Contents

- [Overview](#overview)
- [Pipeline Workflows](#pipeline-workflows)
- [Testing Strategy](#testing-strategy)
- [Code Coverage Strategy](#code-coverage-strategy)
- [Security & Quality Checks](#security--quality-checks)
- [Build & Release Process](#build--release-process)
- [Performance & Cost](#performance--cost)
- [Troubleshooting](#troubleshooting)

---

## Overview

### Design Philosophy

The CI/CD pipeline is designed with these core principles:

1. **Fast Feedback**: Developers get results in < 15 minutes
2. **High Confidence**: Comprehensive testing before merge
3. **Language Coverage**: Tests all 5 supported debuggers
4. **Cost Efficient**: Stays within GitHub free tier (2,000 min/month)
5. **Easy to Debug**: Clear logs and summaries in GitHub Actions

### Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PR Opened / Push to Main                  │
└────────────────────┬────────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌───────────────┐         ┌───────────────┐
│  Unit Tests   │         │  Integration  │
│   Workflow    │         │     Tests     │
│   (ci.yml)    │         │   Workflow    │
└───────┬───────┘         └───────┬───────┘
        │                         │
        ├──> Linting              ├──> Build Docker
        ├──> Security             ├──> Run All Tests
        ├──> Dependencies         ├──> Coverage Report
        ├──> Unit Tests           │
        ├──> Code Coverage        │
        │                         │
        └─────────┬───────────────┘
                  │
                  ▼
          ┌───────────────┐
          │    Codecov    │
          │   (Merges)    │
          └───────┬───────┘
                  │
                  ▼
          ┌───────────────┐
          │  Combined     │
          │  Coverage     │
          │  42-45%       │
          └───────────────┘
```

---

## Pipeline Workflows

### Workflow 1: Unit Tests & Quality (`ci.yml`)

**Triggers**:
- Every push to `main` branch
- Every pull request to `main` branch

**Duration**: ~5-7 minutes

**Jobs**:

#### 1. Linting with Clippy

**Purpose**: Enforce Rust code quality standards

**What it does**:
- Runs `cargo fmt` to check formatting
- Runs `cargo clippy` to catch common mistakes
- Generates JSON report for GitHub Actions summary
- Counts warnings and errors

**Exit criteria**: Must have 0 errors (warnings allowed)

**Benefits**:
- ✅ Catches bugs at compile time
- ✅ Enforces consistent code style
- ✅ Prevents common Rust anti-patterns

#### 2. Security Scanning

**Purpose**: Identify security vulnerabilities in dependencies

**What it does**:
- Runs `cargo audit` to check for known CVEs
- Categorizes by severity (Critical, High, Medium, Low)
- Generates security report artifact

**Exit criteria**: Non-blocking (reports only)

**Benefits**:
- ✅ Early warning of security issues
- ✅ Tracks vulnerability history
- ✅ Helps prioritize updates

#### 3. Dependency Review

**Purpose**: Validate dependency licenses and policies

**What it does**:
- Runs `cargo deny` to check licenses
- Validates dependency sources
- Checks for duplicate dependencies

**Exit criteria**: Non-blocking (reports only)

**Benefits**:
- ✅ Ensures license compliance
- ✅ Prevents supply chain attacks
- ✅ Optimizes dependency tree

#### 4. Test Suite with Nextest

**Purpose**: Run fast unit tests

**What it does**:
- Runs `cargo nextest run --lib` (library tests only)
- Parallel test execution for speed
- Tests only `src/` code, not integration tests
- Generates test summary

**Test count**: 179 unit tests

**Exit criteria**: All tests must pass (0 failures)

**Benefits**:
- ✅ Fast feedback (< 2 minutes)
- ✅ Tests core functionality
- ✅ Catches regressions early

#### 5. Code Coverage

**Purpose**: Measure unit test coverage

**What it does**:
- Runs `cargo tarpaulin --lib --exclude-files 'tests/*'`
- Generates coverage reports (JSON, XML, HTML)
- Uploads to Codecov with flag `unit-tests`
- Enforces 27% minimum threshold

**Coverage**: ~27.96% (unit tests only)

**Exit criteria**: Coverage must be ≥ 27%

**Benefits**:
- ✅ Tracks which code is tested
- ✅ Prevents coverage regressions
- ✅ Identifies untested code paths

#### 6. Multi-Platform Builds

**Purpose**: Ensure compatibility across platforms

**What it does**:
- Builds release binaries for:
  - Linux x86_64
  - macOS ARM64 (M1/M2/M3)
  - macOS x86_64 (Intel)
  - Windows x86_64
- Uses matrix strategy for parallel builds
- Uploads artifacts for each platform

**Exit criteria**: All platforms must build successfully

**Benefits**:
- ✅ Cross-platform compatibility
- ✅ Ready-to-use binaries
- ✅ Catches platform-specific issues

---

### Workflow 2: Integration Tests (`integration-tests.yml`)

**Triggers**:
- Pull requests to `main` (only if relevant files change)
- Push to `main` branch

**Relevant file paths**:
- `src/**`
- `tests/**`
- `Cargo.toml`, `Cargo.lock`
- `Dockerfile.integration-tests`
- `.github/workflows/integration-tests.yml`

**Duration**: ~10-12 minutes

**What it does**:

#### Step 1: Build Docker Image

**Purpose**: Create isolated test environment with all debuggers

**Process**:
```dockerfile
# Install all debuggers in single image:
- Python 3.11 + debugpy
- Ruby 3.1 + debug gem
- Node.js 18 + @vscode/js-debug
- Rust (rustc) + LLDB
- Go 1.21 + Delve
- cargo-nextest (test runner)
- cargo-tarpaulin (coverage)
```

**Optimization**: Docker layer caching via GitHub Actions
- First build: ~10 minutes
- Cached builds: ~2-3 minutes
- Cache scope: `integration-tests`

**Benefits**:
- ✅ Consistent environment (same as local Docker)
- ✅ All debuggers pre-installed
- ✅ Fast with caching
- ✅ Reproducible builds

#### Step 2: Run Integration Tests

**Purpose**: Test actual debugger communication end-to-end

**Process**:
```bash
docker run debugger-mcp:integration-tests \
  cargo tarpaulin \
    --all \
    --ignore-panics \
    --exclude-files 'tests/bin/*' \
    --out Json --out Xml --out Html \
    --output-dir coverage-integration \
    -- --include-ignored
```

**Test breakdown**:
- Python: ~5 tests (`integration_test.rs`)
- Ruby: ~18 tests (`test_ruby_*.rs`)
- Node.js: ~7 tests (`test_nodejs_integration.rs`)
- Rust: ~15 tests (`test_rust_integration.rs`)
- Go: ~2 tests (`test_golang_integration.rs`)

**Total**: ~47 integration tests

**What gets tested**:
- ✅ Debugger spawn and connection
- ✅ DAP protocol communication
- ✅ Breakpoint setting and hitting
- ✅ Variable evaluation
- ✅ Stack trace retrieval
- ✅ Step commands (over, into, out)
- ✅ Multi-file project debugging
- ✅ Session lifecycle management

**Benefits**:
- ✅ Catches integration bugs
- ✅ Tests real debugger behavior
- ✅ Validates multi-language support
- ✅ Exercises code paths not hit by unit tests

#### Step 3: Upload Coverage to Codecov

**Purpose**: Contribute integration test coverage to combined report

**Process**:
- Uploads `coverage-integration/cobertura.xml`
- Uses flag `integration-tests`
- Codecov merges with `unit-tests` flag automatically

**Coverage contribution**: ~14-15% additional coverage

**Benefits**:
- ✅ Shows what integration tests cover
- ✅ Combined with unit test coverage
- ✅ Identifies gaps in testing

#### Step 4: Generate Summary

**Purpose**: Clear reporting in GitHub Actions UI

**What it shows**:
- Total tests run
- Passed/Failed/Ignored counts
- Coverage percentage
- List of languages tested
- Failed test details (if any)

**Benefits**:
- ✅ Quick status at a glance
- ✅ Easy to identify failures
- ✅ No need to dig through logs

---

## Testing Strategy

### Two-Tier Testing Approach

```
┌─────────────────────────────────────────────────────────┐
│                     Test Pyramid                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│              /\                Integration Tests         │
│             /  \               ~47 tests                 │
│            /    \              Docker-based              │
│           /      \             10-12 minutes             │
│          /        \            Tests: E2E flows          │
│         /          \                                     │
│        /            \                                    │
│       /   Unit Tests \          Unit Tests               │
│      /     ~179 tests  \        ~179 tests               │
│     /    Native cargo    \      Fast (< 2 min)           │
│    /   Tests: Functions   \     Tests: Functions         │
│   /     & Logic            \                             │
│  /________________________  \                            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Unit Tests (Fast & Focused)

**Philosophy**: Test individual functions and modules in isolation

**Characteristics**:
- **Fast**: < 2 minutes for all 179 tests
- **Isolated**: No external dependencies
- **Focused**: One function/module at a time
- **Mocked**: Uses mock DAP clients, not real debuggers

**Example Test**:
```rust
#[test]
fn test_go_launch_args_multifile_package() {
    let program = "/workspace/mypackage/";
    let launch_args = GoAdapter::launch_args_with_options(
        program, &[], None, false
    );

    assert_eq!(launch_args["program"], "/workspace/mypackage/");
    assert_eq!(launch_args["mode"], "debug");
}
```

**What gets tested**:
- ✅ Configuration parsing
- ✅ Launch args generation
- ✅ Error handling logic
- ✅ State transitions
- ✅ Protocol serialization

**Coverage**: ~27.96% (src/ only)

### Integration Tests (Comprehensive & Realistic)

**Philosophy**: Test complete workflows with real debuggers

**Characteristics**:
- **Comprehensive**: Tests entire adapter stack
- **Realistic**: Uses actual debuggers (debugpy, delve, etc)
- **Slower**: ~10-12 minutes (includes Docker build)
- **End-to-end**: From spawn to breakpoint to evaluation

**Example Test**:
```rust
#[tokio::test]
#[ignore] // Requires dlv installed
async fn test_go_multifile_package_debug() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/go/multifile");

    // Actually spawns Delve and connects via TCP
    let result = golang::GoAdapter::spawn(
        fixture_path.to_str().unwrap(), &[], false
    ).await;

    assert!(result.is_ok());
    assert!(result.unwrap().process.id().is_some());
}
```

**What gets tested**:
- ✅ Real debugger spawning
- ✅ TCP/STDIO connection handling
- ✅ DAP protocol communication
- ✅ Breakpoint setting and hitting
- ✅ Variable evaluation accuracy
- ✅ Multi-file project support
- ✅ Error recovery and cleanup

**Coverage**: ~14-15% (additional code paths)

### Combined Coverage

```
Total Coverage: ~42-45%

Unit Test Coverage (27.96%):
├── MCP protocol layer:     ~90%
├── DAP types:              ~85%
├── Error handling:         ~80%
├── Adapter config:         ~75%
└── Session state:          ~70%

Integration Test Coverage (14-15%):
├── Adapter spawn:          ~85%
├── Socket connections:     ~90%
├── DAP communication:      ~80%
├── Multi-file debugging:   ~75%
└── Debugger integration:   ~70%

Low Coverage Areas (<30%):
├── Transport layer internals
├── Some error edge cases
└── Platform-specific code
```

### Why This Strategy Works

#### Fast Feedback Loop
- Unit tests run first (2 min)
- Developers know immediately if basic logic breaks
- Integration tests run in parallel
- Total feedback time: ~12 minutes

#### Comprehensive Coverage
- Unit tests verify logic correctness
- Integration tests verify real-world behavior
- Combined coverage shows true test quality
- ~42-45% coverage vs 27.96% unit-only

#### Cost Effective
- Unit tests are free (fast, no special setup)
- Integration tests use Docker caching
- Total CI time: ~12 min/PR × 20 PR/month = 240 min/month
- Well within free tier (2,000 min/month)

#### Developer Experience
- Clear separation of concerns
- Easy to run locally (Docker or native)
- Same environment in CI and locally
- Detailed failure reporting

---

## Code Coverage Strategy

### The Challenge: Combining Two Coverage Sources

**Problem**:
- Unit tests only cover `src/` code (--lib)
- Integration tests cover everything (--all)
- Need combined view without double-counting

**Solution**: Codecov Flags

### How Codecov Flags Work

#### Upload 1: Unit Test Coverage

```yaml
# ci.yml workflow
- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v4
  with:
    files: ./coverage/cobertura.xml
    token: ${{ secrets.CODECOV_TOKEN }}
    flags: unit-tests                    # ← Flag 1
    name: unit-tests-coverage
```

**Characteristics**:
- File: `coverage/cobertura.xml`
- Generated by: `cargo tarpaulin --lib`
- Covers: `src/` only (no integration tests)
- Coverage: ~27.96%

#### Upload 2: Integration Test Coverage

```yaml
# integration-tests.yml workflow
- name: Upload integration coverage to Codecov
  uses: codecov/codecov-action@v4
  with:
    files: ./coverage-integration/cobertura.xml
    token: ${{ secrets.CODECOV_TOKEN }}
    flags: integration-tests             # ← Flag 2
    name: integration-tests
```

**Characteristics**:
- File: `coverage-integration/cobertura.xml`
- Generated by: `cargo tarpaulin --all -- --include-ignored`
- Covers: Everything (src/ + tests/ + integration)
- Coverage: Full codebase with integration paths

### Codecov Automatic Merging

**How it works**:
1. Both workflows upload to same commit SHA
2. Codecov detects two uploads with different flags
3. Codecov intelligently merges coverage data
4. Lines covered by either upload count as covered
5. Result: Combined coverage percentage

**Visual Representation**:

```
Commit: abc123

Upload 1 (unit-tests):
├── src/adapters/golang.rs:     30% (config/args covered)
├── src/dap/socket_helper.rs:   0%  (not tested by unit tests)
└── src/mcp/protocol.rs:        90% (well tested)

Upload 2 (integration-tests):
├── src/adapters/golang.rs:     55% (spawn/connect covered)
├── src/dap/socket_helper.rs:   90% (used by integration tests)
└── src/mcp/protocol.rs:        90% (same as unit tests)

Codecov Merged:
├── src/adapters/golang.rs:     85% (30% + 55% = 85% combined)
├── src/dap/socket_helper.rs:   90% (0% + 90% = 90%)
└── src/mcp/protocol.rs:        90% (90% ∪ 90% = 90%)

Total Combined Coverage: ~42-45%
```

### Viewing Coverage in Codecov

#### Dashboard View

**URL**: `https://codecov.io/gh/YOUR_ORG/debugger-mcp`

**Default view shows**:
```
debugger-mcp
├── Coverage: 42.5%
├── Files: 45
├── Lines: 2,500 (1,062 covered)
└── Commits: 150
```

**Flag breakdown** (click "Flags" tab):
```
Flags:
├── unit-tests:        27.96%  (179 tests)
│   └── Last upload:   2 hours ago
└── integration-tests: 14.54%  (47 tests)
    └── Last upload:   1 hour ago
```

#### File View

Click any file to see line-by-line coverage:

```rust
// src/adapters/golang.rs

impl GoAdapter {
    pub fn command() -> &'static str {        // ✅ Covered (unit test)
        "dlv"
    }

    pub async fn spawn(                       // ✅ Covered (integration test)
        program: &str,
        args: &[String],
        stop_on_entry: bool,
    ) -> Result<GoDebugSession> {
        let port = find_free_port()?;         // ✅ Covered (integration test)
        // ...
    }

    fn internal_helper() -> String {          // ❌ Not covered
        "helper".to_string()
    }
}
```

**Coverage markers**:
- 🟢 Green: Covered by unit tests
- 🟡 Yellow: Covered by integration tests
- 🔴 Red: Not covered by any tests

#### PR Comment

Codecov bot automatically comments on PRs:

```
## Codecov Report
Attention: Patch coverage is 85.0% with 15 lines in your changes
are missing coverage. Please review.

> Project coverage is 42.5%. Comparing base (abc123) to head (def456).

| Files | Coverage Δ | Complexity Δ |
|-------|------------|--------------|
| src/adapters/golang.rs | 85.0% (+85.0%) | 15 (+15) |

Flags with carried forward coverage won't be shown.
Click here to find out more.
```

### Coverage Configuration (Optional)

Create `.codecov.yml` for advanced settings:

```yaml
# .codecov.yml (optional)

coverage:
  precision: 2
  round: down
  range: "70...100"

  status:
    project:
      default:
        target: 40%              # Combined coverage threshold
        threshold: 1%            # Allow 1% drop
    patch:
      default:
        target: 60%              # New code must be 60% covered

flags:
  unit-tests:
    paths:
      - src/
    carryforward: true           # Reuse if workflow skipped

  integration-tests:
    paths:
      - src/
      - tests/
    carryforward: true

comment:
  layout: "reach, diff, flags, files"
  behavior: default
  require_changes: false
  require_base: false
  require_head: true

ignore:
  - "tests/**/*"                 # Don't report test coverage
```

### Benefits of Combined Coverage

#### 1. True Coverage Metric

**Without integration tests**:
```
Coverage: 27.96%
Reality: Much critical code untested
Problem: False confidence
```

**With integration tests**:
```
Coverage: 42.5%
Reality: Most code paths exercised
Benefit: True confidence
```

#### 2. Identify Gaps

Can see which code paths are:
- ✅ Covered by unit tests only
- ✅ Covered by integration tests only
- ✅ Covered by both (best!)
- ❌ Not covered at all (needs work)

#### 3. Track Improvements

**Coverage history**:
```
Jan 2025: 27.96% (unit only)
Feb 2025: 42.50% (unit + integration)
Mar 2025: 48.00% (added more tests)
Apr 2025: 55.00% (target: 60%)
```

#### 4. Better PR Reviews

Reviewers can see:
- Which new lines are tested
- Whether tests are unit or integration
- If coverage went up or down
- Which files need more tests

---

## Security & Quality Checks

### Security Scanning (cargo audit)

**Purpose**: Detect known vulnerabilities in dependencies

**How it works**:
```bash
cargo audit --json > cargo-audit.json
```

**Checks**:
- RustSec Advisory Database
- Known CVEs in dependencies
- Unmaintained crates
- Yanked versions

**Output categories**:
```
Security Report:
├── Critical: 0  (Block PR if > 0)
├── High:     0  (Warning)
├── Medium:   2  (Informational)
└── Low:      1  (Informational)
```

**Benefits**:
- ✅ Early warning of security issues
- ✅ Automated tracking
- ✅ Helps prioritize updates
- ✅ Compliance documentation

### Dependency Review (cargo deny)

**Purpose**: Enforce dependency policies

**How it works**:
```bash
cargo deny check
```

**Checks**:
- License compliance (MIT, Apache-2.0, etc.)
- Banned dependencies
- Duplicate dependencies
- Dependency sources (crates.io only)

**Example policies**:
```toml
# deny.toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause"]
deny = ["GPL-3.0"]                    # No GPL in our project

[bans]
deny = [
    { name = "openssl" }              # Use rustls instead
]

[sources]
allow-git = []                        # Only crates.io
```

**Benefits**:
- ✅ License compliance
- ✅ Supply chain security
- ✅ Prevents dependency bloat
- ✅ Audit trail

### Code Quality (clippy)

**Purpose**: Catch common mistakes and enforce idioms

**How it works**:
```bash
cargo clippy --all-targets -- -D warnings
```

**Categories of checks**:

1. **Correctness** (errors):
   - Potential bugs
   - Logic errors
   - Type mismatches

2. **Performance**:
   - Inefficient code patterns
   - Unnecessary allocations
   - Clone optimizations

3. **Style**:
   - Rust idioms
   - Naming conventions
   - Code readability

4. **Complexity**:
   - Overly complex functions
   - Deeply nested code
   - Cognitive complexity

**Example warnings**:
```
warning: this expression creates a reference which is immediately dereferenced
  --> src/adapters/golang.rs:45:20
   |
45 |     let result = &value.to_string();
   |                  ^^^^^^^^^^^^^^^^^^^ help: change this to: `value.to_string()`
```

**Benefits**:
- ✅ Catches bugs early
- ✅ Enforces best practices
- ✅ Improves maintainability
- ✅ Reduces cognitive load

### Pre-commit Hooks (Local)

**Not in CI, but important for developers**:

```bash
# Install hooks
pre-commit install --install-hooks
```

**Hooks run on commit**:
- `cargo fmt` (formatting)
- `cargo clippy` (linting)
- `gitleaks` (secret scanning)
- `commitizen` (commit message format)

**Hooks run on push**:
- `cargo test` (unit tests)
- `cargo tarpaulin` (coverage check)
- `cargo audit` (security)
- `cargo deny` (dependencies)

**Benefits**:
- ✅ Catches issues before CI
- ✅ Faster feedback (local)
- ✅ Reduces CI failures
- ✅ Teaches best practices

---

## Build & Release Process

### Multi-Platform Builds

**Platforms supported**:
- Linux x86_64 (ubuntu-latest)
- macOS ARM64 (macos-latest, M1/M2/M3)
- macOS x86_64 (macos-13, Intel)
- Windows x86_64 (windows-latest)

**Build process**:
```yaml
strategy:
  matrix:
    include:
      - platform: Linux x86_64
        runner: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - platform: macOS ARM64
        runner: macos-latest
        target: aarch64-apple-darwin
      # ... etc
```

**Steps for each platform**:
1. Install Rust toolchain
2. Add target architecture
3. Cache dependencies
4. Build release binary
5. Upload artifact

**Artifacts**:
```
debugger-mcp-x86_64-unknown-linux-gnu/
├── debugger_mcp             (Linux binary)

debugger-mcp-aarch64-apple-darwin/
├── debugger_mcp             (macOS ARM binary)

debugger-mcp-x86_64-apple-darwin/
├── debugger_mcp             (macOS Intel binary)

debugger-mcp-x86_64-pc-windows-msvc/
├── debugger_mcp.exe         (Windows binary)
```

**Benefits**:
- ✅ Cross-platform compatibility verified
- ✅ Ready-to-use binaries for releases
- ✅ Catches platform-specific issues
- ✅ Parallel builds (fast)

### Release Process (Manual)

**Current process** (to be automated later):

1. **Tag release**:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0: Go support"
   git push origin v0.2.0
   ```

2. **Download artifacts** from GitHub Actions

3. **Create GitHub Release**:
   - Title: `v0.2.0 - Go (Delve) Support`
   - Description: Changelog
   - Attach: All 4 platform binaries

4. **Publish to crates.io** (optional):
   ```bash
   cargo publish
   ```

**Future automation** (TODO):
- Automatic release on tag push
- Changelog generation
- Binary signing
- Docker image publishing

---

## Performance & Cost

### CI Performance Metrics

#### Workflow: Unit Tests (ci.yml)

```
Job                     Duration    Parallelization
─────────────────────────────────────────────────────
Linting                 1.5 min     ✓ Parallel
Security                1.5 min     ✓ Parallel
Dependencies            1.5 min     ✓ Parallel
Unit Tests              2.0 min     Sequential (needs: linting)
Code Coverage           3.0 min     Sequential (needs: test)
Build (4 platforms)     8.0 min     ✓ Parallel
─────────────────────────────────────────────────────
Total (critical path)   ~7 min      Optimized
Total (all jobs)        ~18 min     With parallelization
```

#### Workflow: Integration Tests (integration-tests.yml)

```
Step                         First Run    Cached
──────────────────────────────────────────────────
Build Docker Image           10 min       2-3 min
Run Integration Tests        5-7 min      5-7 min
Upload Coverage              30 sec       30 sec
Generate Summary             15 sec       15 sec
──────────────────────────────────────────────────
Total                        ~16 min      ~10 min
```

#### Combined Pipeline Duration

```
PR Workflow Timeline:
─────────────────────────────────────────────────

0 min  ├─ PR opened/updated
       │
1 min  ├─ Linting ──────────────────┐
       ├─ Security ─────────────────┤
       ├─ Dependencies ─────────────┤  All parallel
       └─ Docker Build ─────────────┤
       │                            │
3 min  ├─ Unit Tests ────────────┐  │
       │                         │  │
5 min  ├─ Code Coverage ─────────┤  │
       │                         │  │
7 min  ├─ Builds ────────────────┤  │
       │  (4 platforms parallel) │  │
       │                         │  │
10 min ├─ Integration Tests ─────┤  │
       │  (in Docker)            │  │
       │                         │  │
12 min └─ All Complete ───────────┴──┘

Critical Path: ~12 minutes
```

### Cost Analysis

#### GitHub Actions Free Tier

- **Minutes/month**: 2,000
- **Storage**: 500 MB
- **Concurrent jobs**: 20

#### Estimated Usage

```
Per PR:
├── Unit Tests:           7 min
├── Integration Tests:   10 min
└── Total:              17 min per PR

Monthly (20 PRs):
├── PRs: 20 × 17 min = 340 min
├── Direct commits: ~50 min
└── Total: ~390 min/month

Utilization: 390 / 2,000 = 19.5% of free tier ✅
```

#### Optimization Strategies

**Docker Layer Caching**:
```
Savings per PR: 7 minutes
Cost: ~0 (GitHub Actions cache)
Benefit: 40% faster integration tests
```

**Parallel Job Execution**:
```
Without parallelization: 25 min
With parallelization:    12 min
Savings:                13 min (52% faster)
```

**Selective Workflow Triggers**:
```yaml
# Only run integration tests on relevant changes
on:
  pull_request:
    paths:
      - 'src/**'
      - 'tests/**'
      - 'Cargo.*'
```

**Savings**: Skip integration tests on docs-only changes
- Docs PRs: 7 min (unit only)
- Code PRs: 12 min (full)

**Result**: ~30% overall time savings

### Performance Improvements Over Time

```
Timeline     Unit Tests   Integration   Total    Improvement
────────────────────────────────────────────────────────────
Oct 2024     5 min        N/A           5 min    Baseline
Jan 2025     2 min        16 min       18 min    First integration
Feb 2025     2 min        10 min       12 min    Docker caching
Mar 2025     2 min        10 min       12 min    Stable
────────────────────────────────────────────────────────────
Target       < 2 min      < 8 min      < 10 min  Goal
```

---

## Troubleshooting

### Common CI Failures

#### 1. Docker Build Timeout

**Symptom**:
```
Error: The operation was canceled.
```

**Cause**: Docker build takes > 60 minutes (GitHub Actions timeout)

**Solution**:
```yaml
# Increase timeout
- name: Build integration test Docker image
  timeout-minutes: 120  # ← Add this
  uses: docker/build-push-action@v5
```

**Alternative**: Pre-build and cache Docker image in GHCR

#### 2. Integration Tests Fail (Debugger Not Found)

**Symptom**:
```
Error: Failed to spawn dlv: No such file or directory
```

**Cause**: Debugger not in PATH inside Docker

**Solution**: Check Dockerfile paths
```dockerfile
# Ensure binaries are in PATH
ENV PATH="/usr/local/go/bin:/root/go/bin:${PATH}"

# Verify installation
RUN which dlv || echo "Delve not found!"
```

#### 3. Coverage Upload Fails

**Symptom**:
```
Error: Unable to upload coverage to Codecov
```

**Cause**: Missing or invalid CODECOV_TOKEN

**Solution**:
1. Go to Codecov.io
2. Get upload token for repository
3. Add as GitHub secret: `CODECOV_TOKEN`
4. Verify in workflow:
```yaml
token: ${{ secrets.CODECOV_TOKEN }}
```

#### 4. Coverage Below Threshold

**Symptom**:
```
Error: Coverage is below threshold 27.96% < 28.00%
```

**Cause**: New code added without tests

**Solutions**:

**Option A**: Add more tests
```rust
#[test]
fn test_new_feature() {
    // Test the new code
}
```

**Option B**: Adjust threshold (temporary)
```yaml
# In ci.yml
cargo tarpaulin --fail-under 27  # Lower threshold
```

**Option C**: Exclude specific files
```yaml
cargo tarpaulin --exclude-files 'src/new_experimental.rs'
```

#### 5. Tests Pass Locally But Fail in CI

**Symptom**: Green locally, red in CI

**Possible causes**:

**A. Timing issues**:
```rust
// Bad: Assumes immediate response
tokio::time::timeout(
    Duration::from_millis(100),  // Too short!
    debugger.wait_for_stop()
).await
```

**Solution**: Increase timeouts in CI
```rust
let timeout = if cfg!(test) {
    Duration::from_secs(10)  // Longer in CI
} else {
    Duration::from_secs(5)
};
```

**B. Platform differences**:
```rust
// Bad: Assumes Unix paths
let path = "/home/user/file.txt";

// Good: Platform-agnostic
let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/file.txt");
```

**C. Missing debuggers**:
```bash
# Test locally with Docker (same as CI)
docker build -f Dockerfile.integration-tests -t test .
docker run --rm -v $(pwd):/workspace test
```

### Debugging CI Issues

#### View Detailed Logs

**In GitHub Actions UI**:
1. Go to Actions tab
2. Click failing workflow run
3. Click failing job
4. Expand failing step
5. Click "View raw logs" for full output

**Download logs**:
```bash
gh run view <run-id> --log > ci-logs.txt
```

#### Re-run with Debug Logging

**Enable debug logs**:
```yaml
env:
  RUST_BACKTRACE: full  # Rust backtrace
  RUST_LOG: debug       # Verbose logging
```

**Or set as secrets**:
- `ACTIONS_RUNNER_DEBUG=true`
- `ACTIONS_STEP_DEBUG=true`

#### SSH into CI Runner (Advanced)

**Using tmate action**:
```yaml
- name: Setup tmate session
  uses: mxschmitt/action-tmate@v3
  if: failure()  # Only on failure
```

**Connect**:
```bash
# Output will show:
# SSH: ssh <random-id>@nyc1.tmate.io
```

---

## Summary & Best Practices

### Key Achievements

✅ **Fast Feedback**: 12-minute full pipeline
✅ **High Coverage**: 42-45% combined (unit + integration)
✅ **Multi-Language**: Tests all 5 debuggers
✅ **Cost Effective**: 19.5% of free tier used
✅ **Clear Reporting**: GitHub Actions summaries
✅ **Easy Debugging**: Detailed logs and artifacts

### Best Practices for Developers

#### Before Committing

```bash
# Run locally to catch issues early
cargo test              # Unit tests
cargo clippy            # Linting
cargo fmt               # Formatting

# Optional: Run full integration tests
docker build -f Dockerfile.integration-tests -t test .
docker run --rm -v $(pwd):/workspace test
```

#### Writing Tests

**DO**:
- ✅ Write unit tests for new functions
- ✅ Write integration tests for new adapters
- ✅ Test both success and error paths
- ✅ Use descriptive test names

**DON'T**:
- ❌ Skip tests because "it's obvious"
- ❌ Test implementation details
- ❌ Create flaky tests (timing issues)
- ❌ Forget to test edge cases

#### Pull Request Checklist

Before opening PR:
- [ ] All unit tests pass locally
- [ ] No clippy warnings
- [ ] Code formatted with rustfmt
- [ ] Coverage doesn't drop
- [ ] Integration tests pass (Docker)
- [ ] Commit messages follow convention
- [ ] Updated documentation if needed

### Continuous Improvement

**Current Focus** (Q1 2025):
- ✅ Implement integration tests
- ✅ Achieve 40%+ combined coverage
- ✅ Optimize Docker build times
- ⏳ Add more language adapters

**Next Steps** (Q2 2025):
- Automate release process
- Add performance benchmarks
- Implement mutation testing
- Increase coverage to 60%

**Long-term Goals** (2025):
- 80%+ code coverage
- < 10 minute full pipeline
- Automated security patching
- Continuous deployment to crates.io

---

## Additional Resources

### Documentation
- [Integration Tests Guide](INTEGRATION_TESTS.md) - Running tests locally
- [Integration Test CI Proposal](INTEGRATION_TEST_CI_PROPOSAL.md) - Architecture decisions
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common issues

### Tools & Links
- [GitHub Actions](https://github.com/features/actions)
- [Codecov](https://codecov.io/)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [cargo-nextest](https://nexte.st/)
- [cargo-audit](https://github.com/rustsec/rustsec)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)

### Support
- Open an issue: [GitHub Issues](https://github.com/YOUR_ORG/debugger-mcp/issues)
- Check logs: Actions tab → Failed workflow → View logs
- Local testing: See [INTEGRATION_TESTS.md](INTEGRATION_TESTS.md)

---

**Last Updated**: October 10, 2025
**Version**: 1.0
**Status**: Production Ready
