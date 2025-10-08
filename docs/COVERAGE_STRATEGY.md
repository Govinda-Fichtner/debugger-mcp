# Code Coverage Strategy

This document explains how code coverage is enforced in the debugger-mcp project.

## Overview

**Target:** 33% minimum code coverage (will increase with TDD to 60%+)
**Tool:** cargo-tarpaulin
**Enforcement:** Pre-push hooks + CI

## Why 33% (for now)?

- **Current baseline:** Project is at 33% coverage
- **Will increase:** Target is 60%+ as TDD progresses
- **Prevents regression:** Blocks coverage from dropping
- **Pragmatic:** Doesn't block development while building up tests

## Where Coverage is Checked

### 1. Pre-Push Hook (Local)

**When:** Before `git push origin main`
**Command:** `cargo tarpaulin --lib --exclude-files "tests/*" --fail-under 60 --skip-clean`
**Duration:** ~30 seconds
**Purpose:** Catch coverage drops before push

**Why pre-push and not pre-commit?**
- Pre-commit should be fast (<5s) for good UX
- Coverage takes ~30s - too slow for every commit
- Pre-push is perfect: enforced but not annoying

### 2. CI Pipeline (GitHub Actions)

**When:** On every PR to main
**Command:** Same as pre-push hook
**Duration:** ~30 seconds
**Purpose:** Source of truth, catches what hooks miss

**Why CI is essential:**
- Hooks can be bypassed (`git push --no-verify`)
- Clean environment guarantees consistency
- Required for branch protection

### 3. Manual Testing (Optional)

**When:** Before committing, if desired
**Command:** `./scripts/test-tarpaulin.sh`
**Purpose:** Quick feedback during development

## How to Use

### Quick Start

```bash
# 1. Make code changes
vim src/lib.rs

# 2. Run unit tests
cargo test --lib

# 3. Check coverage (optional)
./scripts/test-tarpaulin.sh

# 4. Commit (fast checks only)
git commit -m "feat: add new feature"

# 5. Push (coverage + all tests)
git push origin main
```

### Understanding Output

#### Success

```
||  Tested/Total Lines:
||      src/debug/manager.rs: 145/180 +80.56%
||      src/mcp/tools/mod.rs: 98/120 +81.67%
||
||  66.73% coverage, 1234/1850 lines covered
||
✅ Coverage check passed!
```

#### Failure

```
||  62.45% coverage, 1155/1850 lines covered
||
ERROR: Coverage is below 60% threshold!

Files with low coverage:
  src/new_module.rs: 12/50 (24.00%)

Add tests to increase coverage.
```

### Fixing Low Coverage

1. **Identify untested code:**
   ```bash
   cargo tarpaulin --lib --out Html
   # Open tarpaulin-report.html in browser
   ```

2. **Write tests:**
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_new_function() {
           // Test logic
       }
   }
   ```

3. **Verify improvement:**
   ```bash
   ./scripts/test-tarpaulin.sh
   ```

## Troubleshooting

### Permission Error

You may see:
```
ERROR cargo_tarpaulin::cargo: Cargo clean failed: Permission denied (os error 13)
```

**This is a known tarpaulin issue and doesn't break the build.** The `--skip-clean` flag prevents the actual clean operation, but tarpaulin still logs the error.

**Workaround:** Ignore it - tarpaulin continues and works correctly.

### Coverage Drops Unexpectedly

**Cause:** New code added without tests
**Fix:** Add tests for the new code

```bash
# Check which files need coverage
cargo tarpaulin --lib --out Html
open tarpaulin-report.html
```

### "Test failed during run"

**Cause:** Failing tests prevent coverage from running
**Fix:** Fix the failing tests first

```bash
# Run tests to see failures
cargo test --lib

# Fix the failures, then check coverage
./scripts/test-tarpaulin.sh
```

### Bypassing (NOT RECOMMENDED)

If you need to push despite low coverage (e.g., WIP branch):

```bash
git push --no-verify
```

**Warning:** CI will still fail. Use only for non-main branches.

## Integration with CI

The `.github/workflows/ci.yml` runs the same coverage check:

```yaml
coverage:
  name: Code Coverage
  runs-on: ubuntu-latest
  needs: test
  steps:
    - run: cargo tarpaulin --lib --exclude-files 'tests/*' --out Xml --fail-under 60
```

**Branch protection enforces this:**
- "Code Coverage" check must pass
- Merging blocked if coverage < 60%

## Best Practices

### During Development

1. **Write tests first** (TDD)
2. **Check coverage frequently:** `./scripts/test-tarpaulin.sh`
3. **Aim for >60%** - leave buffer for refactoring

### Before Committing

```bash
# Fast checks (pre-commit runs automatically)
git commit -m "feat: add feature"

# If you want to check coverage early:
./scripts/test-tarpaulin.sh
```

### Before Pushing

```bash
# Pre-push hook runs automatically:
# - cargo tarpaulin (coverage)
# - cargo test --lib (all unit tests)

git push origin main
```

## Coverage Exclusions

### What's Excluded

- **Integration tests:** `tests/*` directory
- **Test code:** `#[cfg(test)]` modules
- **Generated code:** Build scripts, macros

### Why?

- **Integration tests** test behavior, not coverage
- **Test code** doesn't need tests
- **Generated code** is validated upstream

### What's Included

- **Library code:** `src/**/*.rs`
- **Module tests:** `#[cfg(test)]` modules (for structure)
- **Public API:** All exported functions

## Future Enhancements

Potential improvements:

- **Differential coverage:** Only check changed files
- **Coverage trending:** Track coverage over time
- **Per-module targets:** Higher bar for critical modules
- **Codecov integration:** Pretty graphs and reports

## FAQ

**Q: Why not 80% or 100% coverage?**
A: 60% is pragmatic for a young codebase with TDD. Can increase later.

**Q: Can I run coverage on just my changes?**
A: Not yet. Run full coverage with `./scripts/test-tarpaulin.sh`.

**Q: Why does coverage take 30 seconds?**
A: Instrumentation + test execution + report generation. Normal for tarpaulin.

**Q: Can I skip coverage checks?**
A: Yes: `git push --no-verify`. But CI will still enforce it.

**Q: What if a file is inherently untestable?**
A: Rare, but can add `#[cfg(not(tarpaulin_include))]` to exclude.

## Related Documentation

- `.pre-commit-config.yaml` - Hook configuration
- `.github/workflows/ci.yml` - CI pipeline
- `scripts/test-tarpaulin.sh` - Manual test script

---

**Last Updated:** 2025-10-08
**Coverage Tool:** cargo-tarpaulin v0.31+
