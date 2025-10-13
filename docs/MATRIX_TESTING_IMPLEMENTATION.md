# Matrix Testing Implementation

## Overview

This document describes the matrix-based parallel testing strategy implemented for the Debugger MCP server integration tests.

## Implementation Date

October 12, 2025

## Motivation

**Previous State:**
- Single CI job running all language tests sequentially
- Total runtime: ~136 seconds
- No isolation between language failures
- Single artifact for all test outputs

**New State:**
- Parallel CI jobs, one per language
- Expected runtime: ~30-40 seconds (5x faster)
- Clear failure isolation per language
- Separate artifacts per language

## Architecture

### CI Workflow Structure

```yaml
jobs:
  1. build-docker       # Build Docker image once (shared)
  2. build-binary       # Build release binary once (shared)
  3. test-language      # Matrix: Run 5 languages in parallel
     - python
     - ruby
     - nodejs
     - go
     - rust
  4. test-summary       # Aggregate results
```

### Matrix Strategy

```yaml
strategy:
  fail-fast: false  # Continue other languages if one fails
  matrix:
    language: [python, ruby, nodejs, go, rust]
    include:
      - language: python
        test_file: integration_test
        emoji: 🐍
        adapter: debugpy
      # ... etc for each language
```

## Files Created

### Test Files

1. **tests/nodejs_integration_test.rs** - Node.js integration tests
   - `test_nodejs_language_detection()`
   - `test_nodejs_adapter_spawning()`
   - `test_nodejs_fizzbuzz_debugging_integration()`
   - `test_nodejs_claude_code_integration()`

2. **tests/go_integration_test.rs** - Go integration tests
   - `test_go_language_detection()`
   - `test_go_adapter_spawning()`
   - `test_go_fizzbuzz_debugging_integration()`
   - `test_go_claude_code_integration()`

3. **tests/rust_integration_test.rs** - Rust integration tests
   - `test_rust_language_detection()`
   - `test_rust_adapter_spawning()`
   - `test_rust_fizzbuzz_debugging_integration()` (stub - needs compilation)
   - `test_rust_claude_code_integration()` (stub - needs compilation)

4. **tests/ruby_integration_test.rs** - Enhanced with Claude Code test
   - Added `test_ruby_claude_code_integration()`

### Test Fixtures

5. **tests/fixtures/fizzbuzz.go** - Go test program
   - FizzBuzz implementation with breakpoint targets

### CI Workflows

6. **.github/workflows/integration-tests-matrix.yml** - New matrix-based workflow
   - Parallel execution
   - Per-language artifacts
   - Aggregate summary

7. **.github/workflows/integration-tests.yml** - Original (kept for reference)
   - Can be removed once matrix workflow is validated

## Test Pattern

Each language follows the same pattern:

### 1. Direct DAP Integration Test

```rust
#[tokio::test]
#[ignore]
async fn test_{lang}_fizzbuzz_debugging_integration() {
    // 1. Check if debugger is available
    // 2. Start debug session with stopOnEntry
    // 3. Set breakpoint
    // 4. Continue execution
    // 5. Get stack trace
    // 6. Evaluate expressions
    // 7. Query resources
    // 8. Disconnect
}
```

### 2. Claude Code CLI Integration Test

```rust
#[tokio::test]
#[ignore]
async fn test_{lang}_claude_code_integration() {
    // 1. Check Claude CLI availability
    // 2. Create test environment
    // 3. Register MCP server
    // 4. Run Claude with debugging prompt
    // 5. Verify protocol log created
    // 6. Cleanup
}
```

## Language Support Matrix

| Language | Adapter | Fixture | Direct DAP Test | Claude Code Test | Status |
|----------|---------|---------|-----------------|------------------|--------|
| Python   | debugpy | ✅ fizzbuzz.py | ✅ Passing | ✅ Passing | **Complete** |
| Ruby     | rdbg    | ✅ fizzbuzz.rb | ✅ Passing | ✅ Added | **Complete** |
| Node.js  | js-debug | ✅ fizzbuzz.js | ✅ Added | ✅ Added | **New** |
| Go       | Delve   | ✅ fizzbuzz.go | ✅ Added | ✅ Added | **New** |
| Rust     | CodeLLDB | ✅ fizzbuzz.rs | ⚠️ Stub | ⚠️ Stub | **Partial** |

## Benefits

### 1. **Parallel Execution (5x Speedup)**
- Before: ~136s sequential
- After: ~30-40s parallel
- Savings: ~90-100 seconds per CI run

### 2. **Failure Isolation**
- Clear identification of which language failed
- Other languages continue testing
- Separate logs per language

### 3. **Better Reporting**
- Language-specific test summaries
- Emoji indicators (🐍 🟢 💎 🐹 🦀)
- Separate artifacts for debugging

### 4. **Scalability**
- Easy to add new languages
- Just add to matrix array
- No workflow restructuring needed

### 5. **Developer Experience**
- Can re-run just failed language
- Faster feedback loops
- Clear failure attribution

## Usage

### Running Locally

```bash
# Run all integration tests for specific language
cargo test --test python_integration_test -- --include-ignored --nocapture
cargo test --test ruby_integration_test -- --include-ignored --nocapture
cargo test --test nodejs_integration_test -- --include-ignored --nocapture
cargo test --test go_integration_test -- --include-ignored --nocapture
cargo test --test rust_integration_test -- --include-ignored --nocapture
```

### Running in CI

The matrix workflow runs automatically on:
- Pull requests to `main`
- Pushes to `main`
- Manual trigger via `workflow_dispatch`

### Viewing Results

Each language job produces:
- `{language}-test-output.txt` - Raw output with ANSI codes
- `{language}-test-clean.txt` - Stripped output for parsing
- GitHub step summary with pass/fail counts

## Future Enhancements

### Short Term
1. **Rust Compilation** - Add pre-compilation step for Rust tests
2. **Test Sharding** - Split large test files within languages
3. **Caching** - Cache compiled binaries between runs

### Long Term
1. **Performance Tracking** - Track test duration trends
2. **Flaky Test Detection** - Identify and quarantine flaky tests
3. **Test Coverage** - Per-language coverage reports
4. **Multiple OS** - Test on Linux, macOS, Windows

## Migration Path

### Phase 1: Validation (Current)
- Keep both workflows active
- Compare results between sequential and parallel
- Validate all languages pass

### Phase 2: Switch (After 1-2 successful runs)
- Make matrix workflow primary
- Rename `integration-tests-matrix.yml` → `integration-tests.yml`
- Archive old workflow

### Phase 3: Optimization (After validation)
- Add caching strategies
- Optimize Docker image layers
- Add test sharding if needed

## Known Limitations

### Rust Tests
- **Issue**: Requires pre-compiled binary (CodeLLDB debugs executables, not source)
- **Workaround**: Tests currently stub out with skip messages
- **Solution**: Add compilation step to test or use pre-compiled fixture

### Resource Usage
- **Issue**: 5 parallel jobs use more GitHub Actions minutes
- **Impact**: ~5x minutes usage (but 5x faster wall time)
- **Mitigation**: Only on important branches (main, PRs)

### Docker Image Size
- **Issue**: Image contains all language debuggers (~2GB)
- **Impact**: Longer initial build (but cached)
- **Mitigation**: Aggressive layer caching

## Metrics

### Expected Improvements

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| CI Runtime | ~136s | ~30-40s | **70% faster** |
| Failure Detection | All or nothing | Per-language | **5x granularity** |
| Re-run Time | Full suite | Single language | **80% faster** |
| Parallel Jobs | 1 | 5 | **5x parallelism** |

## Conclusion

The matrix testing strategy provides significant improvements in:
- ⚡ **Speed** - 5x faster CI runs
- 🎯 **Clarity** - Per-language failure attribution
- 🔄 **Efficiency** - Only re-run failed languages
- 📊 **Reporting** - Clear language-specific metrics

This implementation follows GitHub Actions best practices and scales well for future language additions.

## References

- [GitHub Actions Matrix Strategy](https://docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs)
- [Docker Build Caching](https://docs.docker.com/build/cache/)
- [Artifact Upload/Download](https://github.com/actions/upload-artifact)
