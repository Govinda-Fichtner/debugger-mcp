# Pre-commit Tools Installation Checklist

This checklist guides you through installing all required tools for the pre-commit hooks on **Debian 13** (or similar Linux distributions).

## Current Status

### ✅ Already Installed
- [x] `pre-commit` 4.3.0
- [x] `commitizen` 4.9.1
- [x] `cargo` (Rust toolchain)

### ❌ Need Installation
- [ ] `rustfmt` (Rust formatter)
- [ ] `clippy` (Rust linter)
- [ ] `cargo-tarpaulin` (code coverage)
- [ ] `cargo-audit` (security scanner)
- [ ] `cargo-deny` (dependency policy)
- [ ] `gitleaks` (secret scanner)

---

## Installation Steps

### 1. Rust Components (rustfmt + clippy)

**Option A: Via rustup (Recommended)**

If you installed Rust via rustup:
```bash
# Add rustfmt and clippy components
rustup component add clippy rustfmt

# Verify installation
cargo fmt --version
cargo clippy --version
```

**Option B: Via apt**

If you installed Rust via apt (system package manager):
```bash
# Install rustfmt and clippy
sudo apt update
sudo apt install rustfmt cargo-clippy

# Verify installation
rustfmt --version
cargo clippy --version
```

**Option C: Install rustup first (Best practice)**

If you're using apt-based Rust and want to switch to rustup:
```bash
# Uninstall apt-based Rust (optional)
sudo apt remove cargo rustc

# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then reload shell
source $HOME/.cargo/env

# Add components
rustup component add clippy rustfmt

# Verify installation
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
```

---

### 2. Cargo Tools

Install Rust tooling via cargo:

```bash
# Code coverage tool
cargo install cargo-tarpaulin

# Security vulnerability scanner
cargo install cargo-audit

# Dependency policy enforcement
cargo install cargo-deny

# Verify installations
cargo tarpaulin --version
cargo audit --version
cargo deny --version
```

**Expected output**:
```
cargo-tarpaulin version: 0.27.x
cargo-audit 0.18.x
cargo-deny 0.14.x
```

**Installation time**: ~5-10 minutes (builds from source)

---

### 3. Secret Scanner (gitleaks)

**Option A: Binary download (Recommended - Faster)**

```bash
# Download latest release
cd /tmp
wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.1/gitleaks_8.18.1_linux_x64.tar.gz

# Extract
tar -xzf gitleaks_8.18.1_linux_x64.tar.gz

# Move to system path
sudo mv gitleaks /usr/local/bin/

# Set permissions
sudo chmod +x /usr/local/bin/gitleaks

# Verify installation
gitleaks version
```

**Expected output**:
```
v8.18.1
```

**Option B: Via Go (if Go is installed)**

```bash
# Install via go
go install github.com/gitleaks/gitleaks/v8@latest

# Verify (ensure $GOPATH/bin is in PATH)
gitleaks version
```

**Option C: Via Homebrew (Linux)**

```bash
# If you have Homebrew on Linux
brew install gitleaks

# Verify
gitleaks version
```

---

### 4. Install Pre-commit Hooks

After all tools are installed:

```bash
# Navigate to project root
cd /home/vagrant/projects/debugger_mcp

# Install git hooks
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Verify installation
pre-commit --version
ls -la .git/hooks/

# Run all hooks against all files (optional - tests setup)
pre-commit run --all-files
```

**Expected output**:
```
pre-commit 4.3.0

.git/hooks/:
-rwxr-xr-x pre-commit
-rwxr-xr-x commit-msg
-rwxr-xr-x pre-push
```

---

## Verification

Test that all tools are working:

```bash
# Rust components
cargo fmt --version
cargo clippy --version

# Cargo tools
cargo tarpaulin --version
cargo audit --version
cargo deny --version

# Secret scanner
gitleaks version

# Pre-commit framework
pre-commit --version

# Commit message validator
cz version
```

**All commands should succeed** and show version numbers.

---

## Test Pre-commit Hooks

Create a test commit to verify hooks run:

```bash
# Make a trivial change
echo "# Test" >> /tmp/test.md

# Stage it (won't work, just testing)
git add /tmp/test.md

# Try to commit (hooks should run)
git commit -m "test: verify pre-commit hooks"
```

**Expected behavior**:
- Hooks run automatically
- You should see output from each hook
- Commit succeeds if all hooks pass

---

## Troubleshooting

### Problem: "rustfmt: command not found"

**Solution**:
```bash
# Check if rustup is installed
rustup --version

# If yes, add component:
rustup component add rustfmt

# If no, install rustup (see Option C above)
```

### Problem: "cargo-tarpaulin install fails"

**Cause**: Missing system dependencies

**Solution**:
```bash
# Install required system libraries
sudo apt update
sudo apt install libssl-dev pkg-config

# Retry installation
cargo install cargo-tarpaulin
```

### Problem: "gitleaks: Permission denied"

**Solution**:
```bash
# Fix permissions
sudo chmod +x /usr/local/bin/gitleaks

# Verify
gitleaks version
```

### Problem: "pre-commit hooks not running"

**Solution**:
```bash
# Reinstall hooks
pre-commit uninstall
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Verify
ls -la .git/hooks/
```

### Problem: "cargo-audit: database not found"

**Solution**:
```bash
# Update advisory database
cargo audit --update

# Retry
cargo audit
```

---

## Quick Reference

### Installation Commands (Copy-Paste Friendly)

```bash
# 1. Rust components
rustup component add clippy rustfmt

# 2. Cargo tools
cargo install cargo-tarpaulin cargo-audit cargo-deny

# 3. Gitleaks
cd /tmp && \
wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.1/gitleaks_8.18.1_linux_x64.tar.gz && \
tar -xzf gitleaks_8.18.1_linux_x64.tar.gz && \
sudo mv gitleaks /usr/local/bin/ && \
sudo chmod +x /usr/local/bin/gitleaks

# 4. Install hooks
cd /home/vagrant/projects/debugger_mcp && \
pre-commit install --install-hooks && \
pre-commit install --hook-type commit-msg && \
pre-commit install --hook-type pre-push

# 5. Verify everything
cargo fmt --version && \
cargo clippy --version && \
cargo tarpaulin --version && \
cargo audit --version && \
cargo deny --version && \
gitleaks version && \
pre-commit --version && \
cz version
```

---

## Time Estimate

- **Rust components**: 1-2 minutes (download)
- **Cargo tools**: 5-10 minutes (compile from source)
- **Gitleaks**: 1 minute (binary download)
- **Hook installation**: < 1 minute

**Total**: ~10-15 minutes

---

## What's Next?

After installation:

1. ✅ Verify all tools work (see Verification section)
2. ✅ Test pre-commit hooks with a test commit
3. ✅ Read [Pre-commit Setup Guide](PRE_COMMIT_SETUP.md) for usage details
4. ✅ Make your first real commit (hooks run automatically!)
5. ✅ Review commit message template in setup guide

---

## Support

If you encounter issues not covered here:

1. Check [Pre-commit Setup Guide](PRE_COMMIT_SETUP.md) for detailed troubleshooting
2. Review tool documentation:
   - rustfmt: https://github.com/rust-lang/rustfmt
   - clippy: https://github.com/rust-lang/rust-clippy
   - cargo-tarpaulin: https://github.com/xd009642/tarpaulin
   - cargo-audit: https://github.com/rustsec/rustsec
   - cargo-deny: https://github.com/EmbarkStudios/cargo-deny
   - gitleaks: https://github.com/gitleaks/gitleaks
3. Open an issue on the project repository

---

**Last Updated**: October 2025
**Platform**: Debian 13 / Ubuntu / Linux
