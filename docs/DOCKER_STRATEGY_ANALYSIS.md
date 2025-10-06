# Docker Strategy Analysis: Multi-Language Debug Adapter

## The Question

As we add support for multiple languages (Python, Ruby, and potentially more), we face a decision:

**Option A**: Single "fat" Dockerfile with all language runtimes
**Option B**: Multiple language-specific Dockerfiles (one per language)
**Option C**: Hybrid approach

---

## Research Findings

### Industry Practice

**VS Code / Debug Adapters**:
- Debug adapters are typically installed as **separate extensions**
- Each language has its own debug adapter installation
- No single "multi-language debugger container" approach found

**Dev Containers (containers.dev)**:
- Emphasize **customizable, tailored** containers per project
- "Features" allow composing capabilities as needed
- Flexibility over one-size-fits-all

**Eclipse Theia / Cloud IDEs**:
- Multi-language support but relies on extension model
- Each language typically has separate tooling/runtime

**Common Pattern**:
- Language runtimes and debug adapters are **NOT bundled together**
- Users install what they need for their specific use case
- Separation of concerns: debugger vs language runtime

---

## Our Current Situation

### Current Dockerfile (Python only)
```dockerfile
FROM rust:1.83-alpine AS builder
# Build debugger_mcp binary
RUN cargo build --release

FROM alpine:latest
# Install Python + debugpy
RUN apk add --no-cache python3 py3-pip && \
    pip install --break-system-packages debugpy

COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/
# ...
```

**Size**: ~150-200 MB (estimate)

### If We Add Ruby (Option A - Single Dockerfile)
```dockerfile
FROM alpine:latest

# Install Python + debugpy
RUN apk add --no-cache python3 py3-pip && \
    pip install --break-system-packages debugpy

# Install Ruby + debug gem
RUN apk add --no-cache ruby ruby-dev ruby-bundler && \
    gem install debug

# Install Node.js + debug adapter (future)
# Install Go + delve (future)
# Install Java + jdtls (future)
# ...
```

**Size**: ~400-600 MB (estimate for 3-4 languages)
**Problem**: Users who only need Python get Ruby, Node, etc.

---

## Option Analysis

### Option A: Single "Fat" Dockerfile

**Pros**:
- ✅ Simple for users (one image for everything)
- ✅ Easy to maintain (one Dockerfile)
- ✅ Works for multi-language projects

**Cons**:
- ❌ Large image size (400-600+ MB)
- ❌ Users download unused runtimes
- ❌ Slower pulls and builds
- ❌ Security: larger attack surface
- ❌ Each language update requires full rebuild
- ❌ Alpine may not have all languages available

**Best For**:
- Users with multi-language projects
- Environments where bandwidth isn't a concern

---

### Option B: Multiple Language-Specific Dockerfiles ✅ IMPLEMENTED

**Structure**:
```
debugger_mcp/
├── Dockerfile.python      # Python + debugpy only
├── Dockerfile.ruby        # Ruby + debug gem only
└── Dockerfile.node        # Node.js + debug adapter only (future)
```

**Build**:
```bash
docker build -f Dockerfile.python -t mcp-debugger:python .
docker build -f Dockerfile.ruby -t mcp-debugger:ruby .
```

**Pros**:
- ✅ Smaller images (~100-120 MB each)
- ✅ Users pull only what they need
- ✅ Faster builds and pulls
- ✅ Better security (smaller attack surface)
- ✅ Independent language updates
- ✅ Clear separation of concerns

**Cons**:
- ❌ More Dockerfiles to maintain (but simpler per-file)
- ❌ Users must choose correct image (but clearer choice)
- ❌ More complex CI/CD (but more flexible)

**Best For**:
- Single-language projects (majority case) ✅
- Resource-constrained environments ✅
- Security-conscious deployments ✅
- **This is our implementation** ✅

---

### Option C: Hybrid / Best of Both Worlds

**Base Image + Language Layers**:

```dockerfile
# Dockerfile.base - Just the debugger binary
FROM rust:1.83-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser
COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/
USER mcpuser
WORKDIR /workspace
ENTRYPOINT ["debugger_mcp"]
```

```dockerfile
# Dockerfile.python - Extends base with Python
FROM mcp-debugger:base
USER root
RUN apk add --no-cache python3 py3-pip && \
    pip install --break-system-packages debugpy
USER mcpuser
```

```dockerfile
# Dockerfile.ruby - Extends base with Ruby
FROM mcp-debugger:base
USER root
RUN apk add --no-cache ruby ruby-dev ruby-bundler && \
    gem install debug
USER mcpuser
```

```dockerfile
# Dockerfile.multi - Extends base with all languages
FROM mcp-debugger:base
USER root
RUN apk add --no-cache python3 py3-pip ruby ruby-dev ruby-bundler && \
    pip install --break-system-packages debugpy && \
    gem install debug
USER mcpuser
```

**Pros**:
- ✅ Reuses base layer (DRY principle)
- ✅ Users choose language-specific or multi
- ✅ Smaller individual images
- ✅ Easy to add new languages
- ✅ Flexibility for users

**Cons**:
- ❌ Slightly more complex setup
- ❌ Requires multi-stage Docker builds
- ❌ More images to publish

---

## Alternative: No Language Runtimes in Container

### Option D: Binary-Only Container + User-Provided Runtimes

**Minimal Dockerfile**:
```dockerfile
FROM alpine:latest
COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/
# NO language runtimes!
ENTRYPOINT ["debugger_mcp"]
```

**Size**: ~20-30 MB

**User Responsibility**:
```bash
# User installs runtimes on host
pip install debugpy
gem install debug

# Mount debugger binary OR use it natively
docker run -v /usr/bin/python3:/usr/bin/python3 \
           -v /usr/local/lib/python3:/usr/local/lib/python3 \
           mcp-debugger
```

**Pros**:
- ✅ Smallest image (20-30 MB)
- ✅ No language runtime management
- ✅ Users control versions
- ✅ Follows "do one thing well" philosophy

**Cons**:
- ❌ Complex volume mounts
- ❌ Portability issues
- ❌ Harder for users to set up
- ❌ Defeats purpose of containerization

**Verdict**: ❌ Not recommended - too complex for users

---

## Recommendation

### Primary Approach: **Option B (Multiple Dockerfiles)**

**Rationale**:
1. **Aligns with Industry Practice**: Debug adapters are typically language-specific
2. **User-Friendly**: Clear choice (python, ruby, or multi)
3. **Efficient**: Users don't download unused runtimes
4. **Maintainable**: Each Dockerfile is simple and focused
5. **Scalable**: Easy to add new languages without bloating existing images

### Implemented Structure

```
debugger_mcp/
├── Dockerfile.python            # Python-only (~120 MB)
├── Dockerfile.ruby              # Ruby-only (~100 MB)
├── docker-compose.yml           # Examples for each variant
└── docs/
    └── DOCKER_DEPLOYMENT.md     # Guide for choosing image
```

**Note**: No multi-language Dockerfile - users choose the language-specific image they need.

### Docker Tags

```bash
# Language-specific images (choose based on your project)
docker pull ghcr.io/you/debugger-mcp:python
docker pull ghcr.io/you/debugger-mcp:ruby

# Version-specific
docker pull ghcr.io/you/debugger-mcp:python-v0.2.0
docker pull ghcr.io/you/debugger-mcp:ruby-v0.2.0
```

### Build Process

```bash
# Build language-specific variants
docker build -f Dockerfile.python -t mcp-debugger:python .
docker build -f Dockerfile.ruby -t mcp-debugger:ruby .

# Tag and push
docker tag mcp-debugger:python ghcr.io/you/debugger-mcp:python
docker tag mcp-debugger:ruby ghcr.io/you/debugger-mcp:ruby
```

### CI/CD Updates

```yaml
# .github/workflows/docker.yml
strategy:
  matrix:
    variant: [python, ruby]

steps:
  - name: Build ${{ matrix.variant }}
    run: |
      docker build -f Dockerfile.${{ matrix.variant }} \
                   -t mcp-debugger:${{ matrix.variant }} .
```

---

## Implementation Path

### ✅ Implemented: Language-Specific Dockerfiles

- Created `Dockerfile.python` (Python-only, ~120 MB)
- Created `Dockerfile.ruby` (Ruby-only, ~100 MB)
- **No multi-language Dockerfile** - users choose based on project needs
- Updated docs to reflect language-specific approach
- **Commits**:
  - "feat: Add Ruby language support and Docker variants"
  - "refactor: Remove multi-lang Dockerfile"

### Future: Optimize & Publish

- Set up CI/CD for language-specific image builds
- Publish to container registry
- Add docker-compose examples
- **Commit**: "ci: Add language-specific Docker builds"

---

## User Documentation Impact

### Getting Started

**For Python Projects**:
```bash
docker run -v $(pwd):/workspace ghcr.io/you/debugger-mcp:python
```

**For Ruby Projects**:
```bash
docker run -v $(pwd):/workspace ghcr.io/you/debugger-mcp:ruby
```

### Choosing an Image

| Use Case | Recommended Image | Size |
|----------|------------------|------|
| Python project | `:python` | ~120 MB |
| Ruby project | `:ruby` | ~100 MB |

---

## Decision Summary

### ✅ Implemented: Language-Specific Only
- `Dockerfile.python` - Python debugging only (~120 MB)
- `Dockerfile.ruby` - Ruby debugging only (~100 MB)
- **No multi-language image** - users choose what they need
- Follows industry best practices (debug adapters are language-specific)

### Future Languages
- `:node` - Node.js debugging (planned)
- `:go` - Go debugging (planned)
- `:rust` - Rust debugging (planned)

---

## Conclusion

**Implemented Strategy**: Language-specific Dockerfiles only ✅

**Rationale**:
1. Industry norm is language-specific debug adapters
2. Smaller images benefit most users (no unused runtimes)
3. Clear separation of concerns
4. Easier maintenance long-term
5. Users choose what they need (Python or Ruby)
6. No "kitchen sink" approach - focused and efficient

**This aligns with**:
- ✅ Container best practices (small, focused images)
- ✅ Security best practices (minimal attack surface)
- ✅ User experience (fast downloads, clear choices)
- ✅ Development workflow (easier testing per language)
- ✅ Industry standards (VS Code, nvim-dap use language-specific adapters)
