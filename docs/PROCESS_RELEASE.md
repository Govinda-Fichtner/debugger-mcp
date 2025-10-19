# Release Process

This document describes how to create and publish releases for the debugger-mcp project.

## Overview

Releases are fully automated through GitHub Actions. When you push a version tag, the release workflow builds binaries for all supported platforms and creates a GitHub release.

## Release Workflow

### 1. Prepare Release

Before creating a release:

```bash
# Ensure you're on main branch with latest changes
git checkout main
git pull origin main

# Ensure all tests pass
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings

# Verify coverage meets threshold
cargo tarpaulin --lib --exclude-files 'tests/*' --fail-under 60

# Update version in Cargo.toml if needed
# Edit: version = "1.0.0"
```

### 2. Create Release Tag

```bash
# Create and push tag (triggers release workflow)
git tag v1.0.0
git push origin v1.0.0
```

**Tag format**: `v<major>.<minor>.<patch>` (e.g., `v1.0.0`, `v2.1.3`)

### 3. Automated Build Process

GitHub Actions automatically:

1. **Creates GitHub Release**
   - Release name: "Release v1.0.0"
   - Auto-generated release notes
   - Draft: No (published immediately)

2. **Builds Multi-Arch Binaries**
   - `x86_64-unknown-linux-gnu` (Linux x86_64)
   - `aarch64-unknown-linux-gnu` (Linux ARM64)

3. **Uploads Release Assets**
   - Compressed archives: `debugger-mcp-<target>.tar.gz`
   - Raw binaries: `debugger-mcp-<target>`

### 4. Verify Release

After the workflow completes (~5-10 minutes):

```bash
# Check release on GitHub
gh release view v1.0.0

# Download and test binary
wget https://github.com/Govinda-Fichtner/debugger-mcp/releases/download/v1.0.0/debugger-mcp-x86_64-unknown-linux-gnu
chmod +x debugger-mcp-x86_64-unknown-linux-gnu
./debugger-mcp-x86_64-unknown-linux-gnu --version
```

## Release Artifacts

Each release includes:

### Compressed Archives (.tar.gz)
- `debugger-mcp-x86_64-unknown-linux-gnu.tar.gz`
- `debugger-mcp-aarch64-unknown-linux-gnu.tar.gz`

**Use case**: Download, extract, and run
**Size**: Smaller (compressed)

### Raw Binaries
- `debugger-mcp-x86_64-unknown-linux-gnu`
- `debugger-mcp-aarch64-unknown-linux-gnu`

**Use case**: Direct download and execute
**Size**: Larger (uncompressed)

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (`v2.0.0`): Breaking changes, incompatible API changes
- **MINOR** (`v1.1.0`): New features, backwards compatible
- **PATCH** (`v1.0.1`): Bug fixes, backwards compatible

### Examples:

```bash
# Major release (breaking changes)
git tag v2.0.0
git push origin v2.0.0

# Minor release (new features)
git tag v1.1.0
git push origin v1.1.0

# Patch release (bug fixes)
git tag v1.0.1
git push origin v1.0.1
```

## Release Checklist

Before creating a release:

- [ ] All PRs merged to main
- [ ] CI passing on main branch
- [ ] Tests pass locally: `cargo test --all-targets`
- [ ] Linting clean: `cargo clippy --all-targets --all-features`
- [ ] Coverage above 60%: `cargo tarpaulin --lib`
- [ ] Version updated in `Cargo.toml` (if applicable)
- [ ] CHANGELOG updated with changes (if maintaining one)
- [ ] Tag follows semver format: `v<major>.<minor>.<patch>`

## Troubleshooting

### Release Workflow Failed

Check the GitHub Actions logs:

```bash
# View workflow runs
gh run list --workflow=release.yml

# View specific run logs
gh run view <run-id> --log
```

Common issues:
- **Compilation errors**: Check Rust code compiles on target platform
- **Permission errors**: Ensure `GITHUB_TOKEN` has release permissions
- **Tag format**: Must match `v*.*.*` pattern

### Delete Failed Release

If a release fails and you need to retry:

```bash
# Delete release
gh release delete v1.0.0 --yes

# Delete tag locally and remotely
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Fix issues, then retry
git tag v1.0.0
git push origin v1.0.0
```

### Test Release Locally

To test the release build without creating a tag:

```bash
# Build release binary
cargo build --release

# Test binary
./target/release/debugger_mcp --version
./target/release/debugger_mcp serve
```

## Post-Release

After a successful release:

1. **Verify Downloads**
   ```bash
   # Check release assets
   gh release view v1.0.0
   ```

2. **Announce Release**
   - Update README.md if needed
   - Notify users (if applicable)

3. **Monitor Issues**
   - Watch for bug reports on new release
   - Be ready to patch if critical issues found

## CI vs Release Workflows

### CI Workflow (`.github/workflows/ci.yml`)
- **Trigger**: Every PR to main
- **Purpose**: Validate code quality and tests
- **Artifacts**: Temporary (90 days), for testing PRs
- **Platforms**: Same as release (multi-arch)

### Release Workflow (`.github/workflows/release.yml`)
- **Trigger**: Push tag `v*.*.*`
- **Purpose**: Create public release for users
- **Artifacts**: Permanent, attached to GitHub release
- **Platforms**: x86_64, ARM64

## Example Release Session

Complete example from start to finish:

```bash
# 1. Prepare
git checkout main
git pull origin main
cargo test --all-targets
cargo clippy --all-targets --all-features

# 2. Update version (if needed)
# Edit Cargo.toml: version = "1.0.0"
git add Cargo.toml
git commit -m "chore: bump version to 1.0.0"
git push origin main

# 3. Create release
git tag v1.0.0
git push origin v1.0.0

# 4. Wait for workflow (~5-10 minutes)
# Monitor: https://github.com/Govinda-Fichtner/debugger-mcp/actions

# 5. Verify
gh release view v1.0.0

# 6. Test download
wget https://github.com/Govinda-Fichtner/debugger-mcp/releases/download/v1.0.0/debugger-mcp-x86_64-unknown-linux-gnu
chmod +x debugger-mcp-x86_64-unknown-linux-gnu
./debugger-mcp-x86_64-unknown-linux-gnu --version

# 7. Success! 🎉
```

## Security Considerations

- **Binaries are built in GitHub Actions**: No local build artifacts uploaded
- **Reproducible builds**: Anyone can verify by checking out the tag and building
- **Checksums**: Consider adding SHA256 checksums in future releases
- **Signing**: Consider GPG signing releases in the future

## Future Enhancements

Potential improvements to the release process:

- [ ] Automated CHANGELOG generation from commits
- [ ] Binary checksums (SHA256) in release notes
- [ ] GPG signing of releases
- [ ] Docker image publishing to Docker Hub
- [ ] Homebrew formula auto-update
- [ ] Cargo crate publishing (crates.io)

---

**Last Updated**: 2025-10-08
**Workflow Files**: `.github/workflows/release.yml`, `.github/workflows/ci.yml`
