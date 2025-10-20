# Cross-Platform Build Strategy

## Platform Targets Overview

### Common Rust Targets

| OS | Architecture | Rust Target | Notes |
|----|--------------|-------------|-------|
| **Linux** | x86_64 | `x86_64-unknown-linux-gnu` | Most common |
| **Linux** | ARM64 | `aarch64-unknown-linux-gnu` | Raspberry Pi, ARM servers |
| **macOS** | x86_64 | `x86_64-apple-darwin` | Intel Macs |
| **macOS** | ARM64 | `aarch64-apple-darwin` | **Apple Silicon (M1/M2/M3)** |
| **Windows** | x86_64 | `x86_64-pc-windows-msvc` | Standard Windows |
| **Windows** | ARM64 | `aarch64-pc-windows-msvc` | Windows on ARM |

### Key Insights

1. **macOS ARM ≠ Linux ARM**: Different targets!
   - macOS Apple Silicon: `aarch64-apple-darwin`
   - Linux ARM: `aarch64-unknown-linux-gnu`

2. **Cross-compilation is complex**: Requires proper toolchains, linkers, and sysroots

3. **Native builds are simpler**: Build on the target platform

## Build Strategies

### Strategy 1: Native Builds (RECOMMENDED)

Build on the actual target platform using GitHub Actions runners.

**Advantages:**
- ✅ No cross-compilation complexity
- ✅ Reliable and well-tested
- ✅ GitHub provides free runners for common platforms
- ✅ Works for all dependencies (including C/C++ native code)

**Disadvantages:**
- ❌ Requires separate jobs for each platform
- ❌ Slower overall (sequential builds)

### Strategy 2: Cross-Compilation

Cross-compile from one platform to another.

**Advantages:**
- ✅ Can build all targets from one machine
- ✅ Faster if running in parallel

**Disadvantages:**
- ❌ Complex linker and toolchain configuration
- ❌ May fail with native dependencies (like `ring`, `openssl`)
- ❌ Requires maintaining cross-compilation setup

### Strategy 3: Hybrid Approach

Use native builds for complex targets, cross-compilation for simple ones.

## Recommended CI Configuration

### Matrix Strategy: Native Builds

```yaml
build:
  name: Build (${{ matrix.os }}, ${{ matrix.arch }})
  runs-on: ${{ matrix.runner }}

  strategy:
    matrix:
      include:
        # Linux
        - os: linux
          arch: x86_64
          target: x86_64-unknown-linux-gnu
          runner: ubuntu-latest

        # macOS Intel
        - os: macos
          arch: x86_64
          target: x86_64-apple-darwin
          runner: macos-13  # Intel runner

        # macOS Apple Silicon
        - os: macos
          arch: arm64
          target: aarch64-apple-darwin
          runner: macos-latest  # ARM64 runner (M1)

        # Windows
        - os: windows
          arch: x86_64
          target: x86_64-pc-windows-msvc
          runner: windows-latest

  steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}

    - name: Build
      run: cargo build --release --target ${{ matrix.target }}

    - name: Upload artifact
      uses: actions/upload-artifact@v4
      with:
        name: debugger-mcp-${{ matrix.os }}-${{ matrix.arch }}
        path: target/${{ matrix.target }}/release/debugger_mcp${{ matrix.os == 'windows' && '.exe' || '' }}
```

### Why This Works

1. **Native compilation**: Each platform builds for itself
2. **No linker issues**: Uses platform's native toolchain
3. **All dependencies work**: Native C/C++ dependencies compile correctly
4. **GitHub Actions support**: Free runners for all these platforms

## Current Issue: Cross-Compilation Without Linker Config

The current `.github/workflows/ci.yml` tries to cross-compile:

```yaml
- target: aarch64-unknown-linux-gnu
  os: ubuntu-latest  # x86_64 host
```

This **compiles** ARM64 code but tries to **link** with x86_64 linker. Missing:

```toml
# .cargo/config.toml (required for cross-compilation)
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

## Option A: Fix Cross-Compilation (Complex)

Add proper linker configuration:

1. **Create `.cargo/config.toml`**:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

2. **Update CI workflow**:
```yaml
- name: Install cross-compilation tools
  run: |
    sudo apt-get update
    sudo apt-get install -y gcc-aarch64-linux-gnu
```

3. **Handle native dependencies**: `ring` crate needs:
```yaml
env:
  CC_aarch64_unknown_linux_gnu: aarch64-linux-gnu-gcc
  AR_aarch64_unknown_linux_gnu: aarch64-linux-gnu-ar
```

## Option B: Use Native Builds (Simple, RECOMMENDED)

Remove cross-compilation, use native runners:

```yaml
strategy:
  matrix:
    include:
      - target: x86_64-unknown-linux-gnu
        runner: ubuntu-latest

      - target: aarch64-apple-darwin
        runner: macos-latest  # Native ARM64 (M1)
```

## Option C: Use `cross` Tool (Middle Ground)

Use Docker-based cross-compilation:

```yaml
- name: Install cross
  run: cargo install cross

- name: Build with cross
  run: cross build --release --target aarch64-unknown-linux-gnu
```

## Recommendations

### For This Project

**Use native builds (Option B)** because:

1. ✅ Most reliable
2. ✅ Simplest to maintain
3. ✅ Works with all dependencies
4. ✅ GitHub provides free runners

### Build Matrix

```yaml
# Minimal: Cover main use cases
- ubuntu-latest → x86_64-unknown-linux-gnu
- macos-latest → aarch64-apple-darwin (Apple Silicon)
- windows-latest → x86_64-pc-windows-msvc

# Optional: Additional targets
- macos-13 → x86_64-apple-darwin (Intel Mac)
- ubuntu-latest with cross → aarch64-unknown-linux-gnu (Linux ARM)
```

### Release Artifacts

For releases, provide:
- `debugger-mcp-linux-x86_64` - Most users
- `debugger-mcp-macos-arm64` - Apple Silicon users
- `debugger-mcp-macos-x86_64` - Intel Mac users (optional)
- `debugger-mcp-windows-x86_64.exe` - Windows users

## Testing Locally

### Test different targets:

```bash
# Install target
rustup target add aarch64-apple-darwin

# Build for target (on macOS with Apple Silicon)
cargo build --target aarch64-apple-darwin

# Check binary
file target/aarch64-apple-darwin/debug/debugger_mcp
# Output: Mach-O 64-bit executable arm64
```

## Summary

| Strategy | Complexity | Reliability | Speed |
|----------|------------|-------------|-------|
| Native builds | Low | High | Medium |
| Cross-compilation | High | Medium | Fast |
| `cross` tool | Medium | High | Medium |

**Recommendation**: Use native builds with GitHub Actions matrix.
