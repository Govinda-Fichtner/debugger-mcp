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

**Purpose:** Validate end-to-end debugging across all 5 supported languages with AI agents

**When it runs:**
- Pull requests to `main` (when code/tests/scripts change)
- Pushes to `main`
- Manual trigger (`workflow_dispatch`)

### End-to-End AI Integration Testing

**What it tests:** Real AI agents (Claude Code and Codex) autonomously debug programs through the MCP server, executing actual debugging workflows end-to-end.

**Why dual AI clients:**
- **Claude Code**: Tests MCP protocol implementation with Anthropic's official client
- **Codex**: Tests OpenAI integration and validates language-agnostic design
- **Both**: Ensures MCP server works with multiple AI providers (not Claude-specific)

**Test coverage:** 10 test combinations (5 languages × 2 AI clients) running in parallel

Each test:
1. AI client receives debugging task via prompt
2. Client uses MCP tools to control debugger (`debugger_start`, `debugger_set_breakpoint`, etc.)
3. Server translates MCP calls to DAP commands for language-specific debugger
4. AI validates results and reports success/failure

### Architecture

```
┌──────────────────┐       ┌──────────────────┐
│  Build Docker    │       │ Build Release    │
│     Image        │       │     Binary       │  ← Build once, reuse
└────────┬─────────┘       └────────┬─────────┘
         │                          │
         ├──────────────────────────┴──────────────────┬──────────────────┬──────────────────┬──────────────────┐
         │                                             │                  │                  │                  │
         ↓                                             ↓                  ↓                  ↓                  ↓
    ┌─────────┐  ┌─────────┐  ┌────────┐  ┌────────┐  ┌───────┐  ┌───────┐  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐
    │ Python  │  │ Python  │  │  Ruby  │  │  Ruby  │  │Node.js│  │Node.js│  │  Go  │  │  Go  │  │ Rust │  │ Rust │
    │ Claude  │  │  Codex  │  │ Claude │  │  Codex │  │ Claude│  │  Codex│  │Claude│  │ Codex│  │Claude│  │ Codex│
    └─────────┘  └─────────┘  └────────┘  └────────┘  └───────┘  └───────┘  └──────┘  └──────┘  └──────┘  └──────┘
         │            │            │            │           │           │          │         │         │         │
         └────────────┴────────────┴────────────┴───────────┴───────────┴──────────┴─────────┴─────────┴─────────┘
                                                          ↓
                                                  ┌──────────────┐
                                                  │ Test Summary │  ← Aggregate all 10 results
                                                  └──────────────┘
```

### Test Architecture Details

#### Test Fixtures (FizzBuzz Programs)

Each language uses a simple FizzBuzz implementation for consistency:

```
tests/fixtures/
├── fizzbuzz.py      # Python test program
├── fizzbuzz.rb      # Ruby test program
├── fizzbuzz.js      # Node.js test program
├── fizzbuzz.go      # Go test program
└── fizzbuzz.rs      # Rust test program
```

**Why FizzBuzz?**
- ✅ Simple algorithm everyone understands
- ✅ Exercises all debugging operations:
  - **Loops** - For setting breakpoints at repeating locations
  - **Conditionals** - For testing step-over logic
  - **Variables** - For expression evaluation (`n`, `i`, results)
  - **Functions** - For stack trace inspection
- ✅ Fast execution - Completes in milliseconds (requires `stopOnEntry: true`)
- ✅ Language-agnostic - Same logic implementable in all languages

**Example (Python):**
```python
def fizzbuzz(n):
    if n % 15 == 0: return "FizzBuzz"
    elif n % 3 == 0: return "Fizz"
    elif n % 5 == 0: return "Buzz"
    else: return str(n)

for i in range(1, 16):
    print(fizzbuzz(i))
```

#### AI Client Test Prompt Structure

Integration tests inject standardized prompts to AI clients (found around lines 1000-1200 in `tests/integration/lang/*_integration_test.rs`):

**Prompt template:**
```
You are an AI debugging assistant. Your task is to debug this {language} program
using the MCP debugging tools available to you.

### 1. Start Debug Session
**Tool**: `debugger_start`
**Parameters**:
```json
{
  "language": "{language}",
  "program": "{path}/fizzbuzz.{ext}",
  "args": [],
  "cwd": null,
  "stopOnEntry": true  ← CRITICAL: Prevents race condition
}
```

### 2. Set Breakpoint
**Tool**: `debugger_set_breakpoint`
[... 8 numbered steps total ...]

### 8. Disconnect Session
**Tool**: `debugger_disconnect`

---

**IMPORTANT**: Create a file `test-results.json` with this exact format:
```json
{
  "test_run": {
    "language": "{language}",
    "timestamp": "ISO-8601",
    "overall_success": true/false,
    "ai_client": "claude"/"codex"
  },
  "operations": {
    "session_started": true/false,
    "breakpoint_set": true/false,
    "breakpoint_verified": true/false,
    "execution_continued": true/false,
    "stopped_at_breakpoint": true/false,
    "stack_trace_retrieved": true/false,
    "variable_evaluated": true/false,
    "session_disconnected": true/false
  },
  "errors": []
}
```
```

**Key design decisions:**
- **Explicit step numbering** - Helps AI track progress through workflow
- **stopOnEntry: true** - Critical for fast-completing programs (prevents race condition)
- **JSON output format** - Enables automated validation by CI workflow
- **8 operations** - Covers complete debugging lifecycle (SBCTED)

#### Operation Validation (SBCTED)

Tests validate 8 operations via `test-results.json` created by AI:

| Letter | Operation | MCP Tool | Validates |
|--------|-----------|----------|-----------|
| **S** | Session Start | `debugger_start` | Debugger launches, connects to program |
| **B** | Breakpoint | `debugger_set_breakpoint` | Breakpoint set and verified |
| **C** | Continue | `debugger_continue` | Execution resumes after pause |
| **T** | Trace | `debugger_stack_trace` | Stack frames retrieved |
| **E** | Evaluate | `debugger_evaluate` | Expression evaluated in scope |
| **D** | Disconnect | `debugger_disconnect` | Clean session termination |

**Additional implicit validations:**
- **`execution_continued`** - Program runs after continue command
- **`stopped_at_breakpoint`** - Breakpoint actually hit (not skipped)

**All 8 must be `true`** for test to pass.

**Example successful result:**
```json
{
  "test_run": {
    "language": "python",
    "ai_client": "codex",
    "timestamp": "2025-10-25T15:07:19Z",
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

#### MCP Protocol Flow

**Complete interaction for Python Codex test (~36 seconds):**

```
1. CI Workflow starts test job
   └─> Spawns Rust test harness (cargo test)
       └─> Rust test spawns Codex CLI process
           └─> Codex receives debugging prompt
               │
               ├─> MCP Request: initialize
               ├─> MCP Request: tools/list
               │   Response: [debugger_start, debugger_set_breakpoint, ...]
               │
               ├─> MCP Request: tools/call "debugger_start"
               │   └─> MCP Server spawns debugpy adapter
               │       └─> DAP: initialize → launch → configurationDone
               │   Response: { session_id: "abc123" }
               │
               ├─> MCP Request: tools/call "debugger_set_breakpoint"
               │   └─> DAP: setBreakpoints
               │   Response: { verified: true }
               │
               ├─> MCP Request: tools/call "debugger_continue"
               │   └─> DAP: continue
               │   └─> DAP Event: stopped (reason: "breakpoint")
               │
               ├─> MCP Request: tools/call "debugger_stack_trace"
               │   └─> DAP: stackTrace
               │   Response: { frames: [...] }
               │
               ├─> MCP Request: tools/call "debugger_evaluate"
               │   └─> DAP: evaluate (expression: "n", frameId: 0)
               │   Response: { result: "1", type: "int" }
               │
               └─> MCP Request: tools/call "debugger_disconnect"
                   └─> DAP: disconnect

2. Codex creates test-results.json (all operations: true)
3. Rust test harness validates JSON format
4. CI workflow uploads artifact and marks test as PASSED
```

**Key observation:** Each MCP tool call maps to one or more DAP protocol messages. The MCP layer abstracts away DAP complexity from the AI client.

#### Test Implementation Location

Integration tests are in `tests/integration/lang/`:

```
tests/integration/lang/
├── python_integration_test.rs    (2 tests: Claude + Codex)
├── ruby_integration_test.rs      (2 tests: Claude + Codex)
├── nodejs_integration_test.rs    (2 tests: Claude + Codex)
├── go_integration_test.rs        (2 tests: Claude + Codex)
└── rust_integration_test.rs      (2 tests: Claude + Codex)
```

**Each file has 2 test functions:**
- `test_{language}_claude_code_integration()` - Tests with Claude Code
- `test_{language}_codex_code_integration()` - Tests with Codex (OpenAI)

**Test structure (Rust async test):**
```rust
#[tokio::test]
#[ignore] // Only run in CI or with --include-ignored
async fn test_python_codex_code_integration() -> Result<()> {
    // 1. Build release binary
    build_release_binary().await?;

    // 2. Prepare prompt with debugging instructions
    let prompt = format!("Debug this Python program:\n{}", DEBUGGING_PROMPT);

    // 3. Spawn Codex CLI with MCP server registered
    let output = Command::new("codex")
        .arg("exec")
        .arg("--dangerously-bypass-approvals-and-sandbox")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env("OPENAI_API_KEY", api_key)
        .spawn()?
        .wait_with_output()?;

    // 4. Validate test-results.json exists and has correct format
    let results: TestResults = serde_json::from_str(&fs::read_to_string("test-results.json")?)?;

    // 5. Assert all operations succeeded
    assert!(results.test_run.overall_success);
    assert!(results.operations.session_started);
    // ... (all 8 operations)

    Ok(())
}
```

#### Troubleshooting Reference

Common failure modes documented in [USAGE_INTEGRATION_TESTS.md](../USAGE_INTEGRATION_TESTS.md#common-ai-client-test-failures):

- **Timeout with zero output** - `stopOnEntry: false` race condition (fixed in commit `cfc4004`)
- **1 operation instead of 8** - Authentication failure or missing test-results.json
- **Variable evaluation fails** - Compiler optimizations (Go/Rust) - AI should step to fix
- **Claude vs Codex differences** - Different retry strategies, both should pass

**Local reproduction:** See [Reproducing CI Failures Locally](../USAGE_INTEGRATION_TESTS.md#reproducing-ci-failures-locally)

---

### Jobs

#### 1. Build Docker Image
- Builds integration test Docker image once
- Caches image layers for speed
- Uploads image as artifact for reuse

#### 2. Build Release Binary
- Builds release binary inside Docker (ensures correct GLIBC)
- Uploads binary for Claude Code tests

#### 3. Test Languages × AI Clients (Matrix)
Runs in parallel: 5 languages × 2 AI clients = 10 concurrent test jobs

**Each test (e.g., "Python + Codex"):**
1. Loads Docker image and release binary
2. AI client (Claude Code or Codex) receives debugging prompt
3. AI autonomously executes 8 debugging operations via MCP tools:
   - **S**ession Start
   - **B**reakpoint Set/Verify
   - **C**ontinue Execution
   - **T**race Stack (retrieve call stack)
   - **E**valuate Expression
   - **D**isconnect Session
4. AI creates `test-results.json` with operation results
5. Workflow uploads results as artifact

**Test validation:** AI must successfully complete all 8 operations and report `overall_success: true`

#### 4. Test Summary
- Downloads all language test results
- Analyzes operation success rates
- Generates summary table

### Success Criteria ✅

**Overall:** All 10 test combinations (5 languages × 2 AI clients) must pass

**Per-test criteria:**
- ✅ `overall_success: true` in `test-results.json`
- ✅ All 8 operations complete successfully:
  - `session_started: true`
  - `breakpoint_set: true`
  - `breakpoint_verified: true`
  - `execution_continued: true`
  - `stopped_at_breakpoint: true`
  - `stack_trace_retrieved: true`
  - `variable_evaluated: true`
  - `session_disconnected: true`
- ✅ Empty `errors` array (`errors: []`)

**Example successful result:**
```json
{
  "test_run": {
    "language": "python",
    "ai_client": "codex",
    "timestamp": "2025-10-24T06:52:39Z",
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

**Summary output when all tests pass (100% success rate):**

| Language       | Status      | Pass Rate | Functionality           | Operations |
|----------------|-------------|-----------|-------------------------|------------|
| Python (claude)| ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Python (codex) | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Ruby (claude)  | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Ruby (codex)   | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Node.js (claude)| ✅ PASS    | 100%      | Fully Functional (JSON) | SBCTED     |
| Node.js (codex)| ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Go (claude)    | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Go (codex)     | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Rust (claude)  | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |
| Rust (codex)   | ✅ PASS     | 100%      | Fully Functional (JSON) | SBCTED     |

**Legend:** S=Session Start, B=Breakpoint, C=Continue, T=Trace, E=Evaluate, D=Disconnect

**Overall:** 10/10 tests passed (100%)

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
| `test-output-{language}-{ai_client}` | Test output for each combination (e.g., `test-output-python-codex`) | 30 days |
| `json-files-{language}-{ai_client}` | All JSON files from test run | 30 days |
| `test-analysis-summary` | Aggregated results table for all 10 tests | 30 days |
| `test-analysis-debug-log` | Detailed analysis debug log | 30 days |

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
     include:
       # ... existing languages ...
       - language: java
         test_file: java_integration_test
         emoji: ☕
         adapter: jdwp
         ai_client: claude
       - language: java
         test_file: java_integration_test
         emoji: ☕
         adapter: jdwp
         ai_client: codex
   ```

2. **Add integration test file:**
   - Create `tests/integration/lang/java_integration_test.rs`
   - Implement both `test_java_claude_code_integration()` and `test_java_codex_code_integration()`
   - Follow existing pattern from other languages
   - Ensure both tests create `test-results.json` with 8 operations

3. **Update Docker image:**
   - Add Java debugger to `Dockerfile.integration-tests`

This adds 2 new test jobs (Java + Claude, Java + Codex), bringing total to 12 tests

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
| Integration Tests | ~8-12 minutes | 10 tests in parallel (5 languages × 2 AI clients) |

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
