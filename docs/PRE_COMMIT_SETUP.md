# Pre-commit Setup Guide

This document explains how to set up and use the pre-commit hooks for the debugger-mcp project.

## Overview

The project uses [pre-commit](https://pre-commit.com/) framework to run automated checks before commits are finalized. This ensures code quality, security, and consistency across all contributions.

## Hook Stages

Checks are split across three git hook stages for optimal developer experience:

### 1. Pre-commit (Fast Checks)
Run on every `git commit`:
- **File consistency**: Line endings, whitespace, large files
- **Format validation**: YAML, TOML, JSON syntax
- **Rust formatting**: `cargo fmt` check
- **Rust linting**: `cargo clippy` with all warnings denied
- **Unit tests**: `cargo test --lib` (fast subset)
- **Secret scanning**: `gitleaks` prevents credential leaks
- **Security audit**: `cargo audit` checks for vulnerabilities
- **Dependency policy**: `cargo deny` enforces license compliance

**Typical runtime**: 10-30 seconds

### 2. Commit-msg
Validates commit message format:
- **Commitlint**: Enforces Conventional Commits + Tim Pope's guidelines
  - Type required (feat, fix, docs, etc.)
  - Subject max 50 characters
  - Imperative mood
  - Proper formatting

### 3. Pre-push (Comprehensive Checks)
Run on `git push` (slower, thorough validation):
- **Code coverage**: `cargo tarpaulin` with 60% minimum threshold
- **All tests**: `cargo test --all-targets` including integration tests

**Typical runtime**: 1-5 minutes

## Installation

### 1. Install Required Tools

#### Pre-commit Framework (Already Installed)
```bash
# Check installation
pre-commit --version
# Should show: pre-commit 4.3.0
```

#### Commitizen (Already Installed)
```bash
# Check installation
cz version
# Should show: 4.9.1
```

#### Rust Components
```bash
# Install rustfmt and clippy via rustup (recommended)
rustup component add clippy rustfmt

# Or via apt (if not using rustup)
sudo apt install rustfmt cargo-clippy
```

#### Cargo Tools
```bash
# Code coverage
cargo install cargo-tarpaulin

# Security audit
cargo install cargo-audit

# Dependency policy enforcement
cargo install cargo-deny
```

#### Secret Scanner
```bash
# Download gitleaks binary
cd /tmp
wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.1/gitleaks_8.18.1_linux_x64.tar.gz
tar -xzf gitleaks_8.18.1_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
sudo chmod +x /usr/local/bin/gitleaks

# Verify installation
gitleaks version
```

### 2. Install Pre-commit Hooks

```bash
# Navigate to project root
cd /home/vagrant/projects/debugger_mcp

# Install git hooks
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Configure local author validation (PRIVATE - not committed)
# This validates your commits use the correct author
# Replace with YOUR name and email
git config --local author.name "Your Name"
git config --local author.email "your.email@example.com"

# Verify installation
pre-commit --version
```

**Note**: The author validation hook (`.git/hooks/pre-commit-local`) reads your expected author from git config. Your email stays private and is never committed to the repository.

### 3. Initial Run (Optional)

Run all hooks against all files to verify setup:

```bash
# Run all pre-commit hooks
pre-commit run --all-files

# Run specific hook
pre-commit run cargo-fmt --all-files
```

## Usage

### Normal Workflow

Pre-commit hooks run automatically:

```bash
# Stage changes
git add src/mcp/tools/breakpoint.rs

# Commit triggers pre-commit hooks automatically
git commit -m "feat(mcp): add breakpoint verification"
# Hooks run: fmt, clippy, unit tests, gitleaks, audit, deny, commitlint

# Push triggers pre-push hooks automatically
git push
# Hooks run: tarpaulin (coverage), all tests
```

### Manual Execution

Run hooks manually without committing:

```bash
# Run all pre-commit stage hooks
pre-commit run --all-files

# Run specific hook
pre-commit run cargo-clippy

# Run hooks on staged files only
pre-commit run
```

### Skipping Hooks (Not Recommended)

```bash
# Skip pre-commit hooks (use only in emergencies)
git commit --no-verify

# Skip pre-push hooks
git push --no-verify
```

**Warning**: Skipping hooks may cause CI failures. Only use when absolutely necessary.

## Hook Details

### Cargo Fmt
**Purpose**: Enforce consistent Rust code formatting
**Command**: `cargo fmt --all -- --check`
**Fix**: Run `cargo fmt --all` to auto-format code

### Cargo Clippy
**Purpose**: Catch common Rust mistakes and anti-patterns
**Command**: `cargo clippy --all-targets --all-features -- -D warnings`
**Fix**: Review clippy suggestions and fix issues manually
**Note**: All warnings are treated as errors (`-D warnings`)

### Cargo Test (Unit)
**Purpose**: Fast unit test execution for quick feedback
**Command**: `cargo test --lib`
**Scope**: Library code only (excludes integration tests)
**Runtime**: ~5-15 seconds

### Gitleaks
**Purpose**: Prevent committing secrets (API keys, tokens, passwords)
**Command**: `gitleaks protect --verbose --redact --staged`
**Fix**: Remove secrets, use environment variables or secret managers
**False positives**: Add to `.gitleaksignore` if needed

### Cargo Audit
**Purpose**: Check dependencies for known security vulnerabilities
**Command**: `cargo audit`
**Database**: RustSec Advisory Database
**Fix**: Update vulnerable dependencies: `cargo update`

### Cargo Deny
**Purpose**: Enforce dependency policies (licenses, sources, bans)
**Command**: `cargo deny check`
**Config**: `deny.toml`
**Checks**:
- Security advisories
- License compliance (MIT, Apache-2.0, BSD allowed)
- Banned crates
- Duplicate dependencies

### Commitizen
**Purpose**: Validate commit message format
**Config**: `.commitlintrc.yml`
**Format**: `type(scope): subject`
**Types**: feat, fix, docs, style, refactor, perf, test, chore
**Rules**:
- Subject max 50 characters
- Body lines max 72 characters
- Imperative mood

### Cargo Tarpaulin
**Purpose**: Measure code coverage
**Command**: `cargo tarpaulin --lib --exclude-files 'tests/*' --out Stdout --fail-under 60`
**Threshold**: 60% minimum coverage
**Stage**: pre-push (slower check)
**Config**: `tarpaulin.toml`

### Cargo Test (All)
**Purpose**: Run complete test suite including integration tests
**Command**: `cargo test --all-targets`
**Scope**: Unit tests + integration tests + doc tests
**Stage**: pre-push
**Runtime**: ~30-120 seconds

## Troubleshooting

### Hook Fails on First Run
```bash
# Update hook repositories
pre-commit autoupdate

# Re-run installation
pre-commit install --install-hooks
```

### Cargo Clippy Fails
```bash
# See detailed clippy output
cargo clippy --all-targets --all-features

# Fix automatically where possible
cargo clippy --fix
```

### Coverage Below Threshold
```bash
# Generate detailed coverage report
cargo tarpaulin --lib --exclude-files 'tests/*' --out Html

# View report
firefox coverage/index.html
```

### Gitleaks False Positive
Add to `.gitleaksignore`:
```
# Example: ignore test fixtures
tests/fixtures/sample_token.txt:1
```

### Pre-commit Hooks Not Running
```bash
# Verify hooks are installed
ls -la .git/hooks/

# Should see: pre-commit, commit-msg, pre-push

# Reinstall if missing
pre-commit install --install-hooks --overwrite
pre-commit install --hook-type commit-msg --overwrite
pre-commit install --hook-type pre-push --overwrite
```

### Update Hook Versions
```bash
# Update all hooks to latest versions
pre-commit autoupdate

# Review changes
git diff .pre-commit-config.yaml
```

## Configuration Files

- `.pre-commit-config.yaml` - Main hook configuration
- `.commitlintrc.yml` - Commit message rules
- `deny.toml` - Dependency policy
- `tarpaulin.toml` - Coverage settings
- `.gitleaksignore` - False positive suppressions (create if needed)

## Best Practices

1. **Run hooks before pushing**: Catch issues early
2. **Don't skip hooks**: They catch real problems
3. **Fix issues immediately**: Don't accumulate technical debt
4. **Update dependencies regularly**: Keep security patches current
5. **Review clippy suggestions**: They improve code quality
6. **Maintain test coverage**: Aim for 70%+ over time
7. **Write good commit messages**: Follow the template

## Commit Message Template

```
type(scope): imperative subject line (max 50 chars)

Detailed explanation of what changed and why. Wrap body
lines at 72 characters for readability in git log and
various git tools.

- Bullet points are fine
- Use present tense ("Add feature" not "Added feature")
- Reference issues and PRs

Closes #123
Refs #456
```

**Examples**:

```
feat(dap): add breakpoint verification

Implement DAP breakpoint verification to ensure breakpoints
are actually set before continuing execution. This prevents
race conditions where execution continues before breakpoint
is ready.

- Add verification state to breakpoint model
- Implement retry logic for unverified breakpoints
- Add timeout handling (30s max)

Closes #42
```

```
fix(nodejs): correct Docker adapter path

The vscode-js-debug adapter is installed at /usr/local/lib/
vscode-js-debug/ but code looked for /usr/local/lib/js-debug/.

Impact: All Node.js debugging failed in Docker containers.

Refs #38
```

## Next Steps

After setting up pre-commit hooks:
1. ✅ Install all required tools
2. ✅ Run `pre-commit install` with all hook types
3. ✅ Test with a sample commit
4. ✅ Review and adjust `deny.toml` for your needs
5. 🔄 Configure GitHub Actions (Part 2 - CI/CD pipeline)

## Support

- Pre-commit docs: https://pre-commit.com/
- Conventional Commits: https://www.conventionalcommits.org/
- Cargo-deny: https://embarkstudios.github.io/cargo-deny/
- Gitleaks: https://github.com/gitleaks/gitleaks
