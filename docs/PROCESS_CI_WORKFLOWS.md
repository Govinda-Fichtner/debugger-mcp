# CI Workflows

## Overview

The project uses two complementary GitHub Actions workflows for continuous integration:

1. **`ci.yml`** - Code quality, security, and unit tests
2. **`integration-tests-matrix.yml`** - End-to-end language integration tests

Both workflows run on pull requests and pushes to `main`, ensuring code quality before merging.

---

## Workflow 1: CI (`ci.yml`)

**Purpose:** Validate code quality, security, dependencies, and unit tests

**When it runs:**
- Pull requests to `main`
- Pushes to `main`

### Jobs

| Job | Purpose | Key Checks |
|-----|---------|------------|
| **Code Quality** | Linting and formatting | • `cargo fmt`<br>• `cargo clippy` (zero warnings) |
| **Security Scanning** | Vulnerability detection | • `gitleaks` (secrets)<br>• `cargo audit` (CVEs) |
| **Dependency Review** | Dependency analysis | • `cargo deny` (license, bans) |
| **Test Suite** | Unit tests | • `cargo nextest` (193 tests) |
| **Code Coverage** | Coverage metrics | • `cargo tarpaulin` (60% minimum) |
| **Build (Multi-platform)** | Cross-compilation | • Linux ARM/x86<br>• macOS ARM/x86<br>• Windows x86 |

### Success Criteria ✅

**All jobs must pass:**
- ✅ No formatting issues (`cargo fmt --check`)
- ✅ Zero clippy warnings
- ✅ No security vulnerabilities (gitleaks, cargo-audit)
- ✅ All dependencies allowed (cargo-deny)
- ✅ All 193 unit tests pass
- ✅ Code coverage ≥ 60%
- ✅ Builds successfully on all 5 platforms

### Failure Criteria ❌

**Any of these will fail the workflow:**
- ❌ Code not formatted correctly
- ❌ Clippy warnings present
- ❌ Security vulnerabilities detected
- ❌ Banned dependencies found
- ❌ Any unit test fails
- ❌ Coverage below 60%
- ❌ Build fails on any platform

### Artifacts

| Artifact | Contents | Retention |
|----------|----------|-----------|
| `clippy-report` | Linting issues (JSON) | 30 days |
| `test-results` | Test output | 30 days |
| `security-report` | Security scan results | 30 days |
| `dependency-check-report` | Dependency analysis | 30 days |
| `coverage-report` | HTML coverage report | 30 days |
| `debugger-mcp-{platform}` | Compiled binaries (5 platforms) | 30 days |

---

## Workflow 2: Integration Tests (`integration-tests-matrix.yml`)

**Purpose:** Validate end-to-end debugging across all 5 supported languages

**When it runs:**
- Pull requests to `main` (when code/tests/scripts change)
- Pushes to `main`
- Manual trigger (`workflow_dispatch`)

### Architecture

```
┌──────────────────┐
│  Build Docker    │  ← Build once, reuse
│     Image        │
└────────┬─────────┘
         │
         ├──────────────────┬──────────────────┬──────────────────┬──────────────────┐
         ↓                  ↓                  ↓                  ↓                  ↓
    ┌────────┐         ┌────────┐         ┌────────┐         ┌────────┐         ┌────────┐
    │ Python │         │  Ruby  │         │Node.js │         │   Go   │         │  Rust  │
    │  Test  │         │  Test  │         │  Test  │         │  Test  │         │  Test  │
    └────────┘         └────────┘         └────────┘         └────────┘         └────────┘
         │                  │                  │                  │                  │
         └──────────────────┴──────────────────┴──────────────────┴──────────────────┘
                                          ↓
                                  ┌──────────────┐
                                  │ Test Summary │  ← Aggregate results
                                  └──────────────┘
```

### Jobs

#### 1. Build Docker Image
- Builds integration test Docker image once
- Caches image layers for speed
- Uploads image as artifact for reuse

#### 2. Build Release Binary
- Builds release binary inside Docker (ensures correct GLIBC)
- Uploads binary for Claude Code tests

#### 3. Test Languages (Matrix)
Runs in parallel for each language: Python, Ruby, Node.js, Go, Rust

**Each language test:**
1. Loads Docker image
2. Runs Claude Code integration test
3. Validates all debugging operations:
   - **S**ession Start
   - **B**reakpoint Set
   - **C**ontinue Execution
   - **T**race Stack
   - **E**valuate Expression
   - **D**isconnect Session
4. Generates `test-results.json`
5. Uploads results as artifact

#### 4. Test Summary
- Downloads all language test results
- Analyzes operation success rates
- Generates summary table

### Success Criteria ✅

**Overall:** All 5 languages must be **100% functional**

**Per-language criteria:**
- ✅ `overall_success: true` in `test-results.json`
- ✅ All 6 operations complete (SBCTED)
- ✅ No errors in `errors` array
- ✅ Session starts successfully
- ✅ Breakpoint verified
- ✅ Program stops at breakpoint
- ✅ Stack trace retrieved
- ✅ Variable evaluated
- ✅ Clean disconnection

**Example successful result:**
```json
{
  "test_run": {
    "language": "python",
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

**Summary output when successful:**

| Language | Status | Pass Rate | Functionality | Operations |
|----------|--------|-----------|---------------|------------|
| Python   | ✅ PASS | 100%      | Fully Functional (JSON) | SBCTED     |
| Ruby     | ✅ PASS | 100%      | Fully Functional (JSON) | SBCTED     |
| Node.js  | ✅ PASS | 100%      | Fully Functional (JSON) | SBCTED     |
| Go       | ✅ PASS | 100%      | Fully Functional (JSON) | SBCTED     |
| Rust     | ✅ PASS | 100%      | Fully Functional (JSON) | SBCTED     |

### Failure Criteria ❌

**Any of these will fail the workflow:**

**Per-language failures:**
- ❌ `overall_success: false`
- ❌ Any operation returns `false`
- ❌ Non-empty `errors` array
- ❌ Missing `test-results.json`
- ❌ Invalid JSON format
- ❌ Claude Code execution timeout

**Critical operations** (these failing means language is broken):
- Session start fails → Cannot debug at all
- Breakpoint not verified → Debugger not working
- Program doesn't stop → Breakpoints ineffective

**Example failure:**
```json
{
  "test_run": {
    "overall_success": false
  },
  "operations": {
    "session_started": true,
    "breakpoint_set": true,
    "breakpoint_verified": false,  ← FAILED
    "execution_continued": false,
    "stopped_at_breakpoint": false,
    "stack_trace_retrieved": false,
    "variable_evaluated": false,
    "session_disconnected": true
  },
  "errors": [
    {
      "operation": "breakpoint_set",
      "message": "Breakpoint was not verified (likely missing debug symbols)"
    }
  ]
}
```

### Artifacts

| Artifact | Contents | Retention |
|----------|----------|-----------|
| `docker-image` | Integration test Docker image | 1 day |
| `release-binary` | Compiled debugger_mcp binary | 1 day |
| `{language}-test-results` | Test output for each language | 30 days |
| `test-analysis-summary` | Aggregated results table | 30 days |

---

## Workflow Relationship

### Complementary Roles

**`ci.yml`** - Pre-flight checks
- ✅ Code compiles
- ✅ No obvious bugs
- ✅ Passes unit tests
- ✅ No security issues

**`integration-tests-matrix.yml`** - Real-world validation
- ✅ Works with actual debuggers
- ✅ Claude can use it
- ✅ All languages functional

### Merge Gatekeeper

**Both workflows must pass** to merge a PR:
- CI ensures code quality
- Integration tests ensure functionality

**Common failure scenarios:**

| Scenario | CI Result | Integration Result | Action |
|----------|-----------|-------------------|--------|
| Unit test bug | ❌ Fail | ⏭️ Skipped | Fix unit test |
| Integration bug | ✅ Pass | ❌ Fail | Fix integration code |
| Breaking change | ✅ Pass | ❌ Fail | Fix breaking change |
| Formatting issue | ❌ Fail | ⏭️ Skipped | Run `cargo fmt` |

---

## Interpreting Results

### GitHub UI

**Check Runs Tab:**
- ✅ Green checkmark = All jobs passed
- ❌ Red X = One or more jobs failed
- 🟡 Yellow dot = In progress

**Click "Details" to see:**
- Individual job results
- Test summaries
- Downloadable artifacts

### Job Summaries

Both workflows generate rich summaries in GitHub Actions UI:

**CI Summary includes:**
- Clippy warnings/errors count
- Test results (passed/failed/skipped)
- Coverage percentage
- Build status per platform

**Integration Summary includes:**
- Language success table
- Operation success rates
- Overall success rate
- Failed tests (if any)

### Quick Diagnostics

**Integration tests failing for one language:**
→ Language-specific debugger issue

**Integration tests failing for all languages:**
→ Core MCP server issue

**CI tests failing but integration passing:**
→ Unit test issue (update tests)

**Both workflows failing:**
→ Breaking change in core functionality

---

## Local Development

### Running CI Checks Locally

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo nextest run

# Run coverage
cargo tarpaulin --out Html --output-dir coverage
```

### Running Integration Tests Locally

```bash
# Build Docker image
docker build -f Dockerfile.integration-tests -t debugger-mcp:integration-tests .

# Run integration tests inside Docker
docker run -it debugger-mcp:integration-tests \
  cargo test --test rust_integration_test -- --ignored --nocapture

# Or use the pre-commit hooks (recommended)
pre-commit run --all-files
```

### Pre-commit Hooks

The project uses git hooks to run checks before committing:

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

## Debugging Workflow Failures

### CI Workflow (`ci.yml`)

**Clippy warnings:**
```bash
# Run clippy locally
cargo clippy --all-targets --all-features -- -D warnings

# Fix issues or allow specific warnings (use sparingly)
#[allow(clippy::warning_name)]
```

**Test failures:**
```bash
# Run specific test
cargo test test_name -- --nocapture

# Run all tests with output
cargo nextest run --no-capture
```

**Coverage failures:**
```bash
# Generate coverage report
cargo tarpaulin --out Html

# Open coverage/index.html to see uncovered lines
```

### Integration Tests (`integration-tests-matrix.yml`)

**Download test artifacts:**
```bash
# Using GitHub CLI
gh run download <run-id> -n python-test-results

# Check test-results.json
cat test-results.json | jq .
```

**Common integration test failures:**

| Error | Cause | Fix |
|-------|-------|-----|
| Breakpoint not verified | Missing debug symbols | Check compilation flags (-g) |
| Session timeout | Debugger not starting | Check debugger installation |
| Invalid JSON | Claude Code didn't write file | Check prompt/permissions |
| Operation failed | DAP protocol issue | Check MCP server logs |

**Run integration test manually:**
```bash
# Inside Docker container
docker run -it debugger-mcp:integration-tests /bin/bash

# Run specific language test
cargo test --test python_integration_test -- --ignored --nocapture
```

---

## Maintenance

### Adding New Language

When adding a new language (e.g., Java):

1. **Update `integration-tests-matrix.yml`:**
   ```yaml
   matrix:
     language:
       - python
       - ruby
       - nodejs
       - go
       - rust
       - java  # New language
   ```

2. **Add integration test:**
   - Create `tests/integration/lang/java_integration_test.rs`
   - Follow existing pattern from other languages

3. **Update Docker image:**
   - Add Java debugger to `Dockerfile.integration-tests`

### Updating Success Criteria

If adding new debugging operations:

1. **Update test-results.json schema:**
   ```json
   "operations": {
     ...,
     "new_operation": true
   }
   ```

2. **Update analysis script:**
   - Modify `scripts/analyze-test-results.sh`
   - Add new operation to SBCTED legend

3. **Update documentation:**
   - This file
   - README.md

---

## Performance

### Typical Run Times

| Workflow | Duration | Parallelization |
|----------|----------|-----------------|
| CI | ~5-7 minutes | 10 parallel jobs |
| Integration Tests | ~8-10 minutes | 5 languages in parallel |

### Optimization Strategies

1. **Caching** - Both workflows cache:
   - Cargo registry
   - Cargo build artifacts
   - Docker layers

2. **Parallelization:**
   - CI: Jobs run in parallel
   - Integration: Languages run in parallel

3. **Fail-fast disabled** - Integration tests continue even if one language fails (to see all results)

---

## Future Improvements

### Planned Enhancements

- [ ] Add benchmark job to CI
- [ ] Add release notes generation
- [ ] Add changelog validation
- [ ] Add dependency update checks
- [ ] Add documentation link checking
- [ ] Add spell checking

### Under Consideration

- [ ] Split CI into separate workflows
- [ ] Add nightly builds
- [ ] Add performance regression tests
- [ ] Add mutation testing
- [ ] Add fuzz testing
