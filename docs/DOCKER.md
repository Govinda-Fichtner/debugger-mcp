# Docker Deployment Guide

This guide covers how to build and run the debugger-mcp server using Docker.

## Overview

The Docker image is built using a multi-stage build process:
1. **Builder stage**: Compiles the Rust binary with static linking (musl)
2. **Runtime stage**: Creates a minimal Alpine-based image (~50MB)

The runtime image includes:
- The compiled `debugger_mcp` binary
- Python 3 + debugpy (for Python debugging support)
- Non-root user for security

## Quick Start

### Build the Image

```bash
docker build -t debugger-mcp:latest .
```

### Run the Server

```bash
docker run -i debugger-mcp:latest
```

The server will start listening on STDIO for MCP protocol messages.

## Using Docker Compose

### Basic Usage

```bash
# Build and start the server
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the server
docker-compose down
```

### Debug Mode

Run with verbose logging:

```bash
docker-compose --profile debug up debugger-mcp-debug
```

## Mounting Workspaces

To debug programs in your local directory:

```bash
docker run -i -v $(pwd)/your-project:/workspace debugger-mcp:latest
```

Then use `/workspace/your-file.py` as the program path when calling `debugger_start`.

## Image Details

### Multi-Stage Build

**Stage 1: Builder (rust:1.83-alpine)**
- Installs musl-dev for static linking
- Compiles release binary with target `x86_64-unknown-linux-musl`
- Results in a fully static binary

**Stage 2: Runtime (alpine:3.21)**
- Minimal base image (~5MB)
- Python 3 + debugpy (~45MB)
- Non-root user (mcpuser:1000)
- Total image size: ~50MB

### Security Features

- Non-root user (`mcpuser`) by default
- No shell in the final image
- Minimal attack surface (Alpine base)
- Static binary (no dynamic dependencies)

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

Set in docker-compose.yml or via `-e`:

```bash
docker run -i -e RUST_LOG=debug debugger-mcp:latest
```

## Integration with MCP Clients

### Claude Desktop

Configure Claude Desktop to use the Docker container:

```json
{
  "mcpServers": {
    "debugger": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-v",
        "${workspaceFolder}:/workspace",
        "debugger-mcp:latest"
      ]
    }
  }
}
```

### Cline (VS Code Extension)

Add to your MCP settings:

```json
{
  "mcpServers": {
    "debugger": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-v",
        "${workspaceFolder}:/workspace",
        "debugger-mcp:latest"
      ]
    }
  }
}
```

## Advanced Usage

### Custom Build Arguments

Build with specific Rust version:

```bash
docker build \
  --build-arg RUST_VERSION=1.83 \
  -t debugger-mcp:latest .
```

### Multi-Platform Builds

Build for multiple architectures:

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t debugger-mcp:latest \
  --push .
```

### Production Deployment

For production use, consider:

```bash
# Build with version tag
docker build -t debugger-mcp:0.1.0 .

# Tag for registry
docker tag debugger-mcp:0.1.0 your-registry.com/debugger-mcp:0.1.0

# Push to registry
docker push your-registry.com/debugger-mcp:0.1.0
```

## Troubleshooting

### Build Fails with musl Errors

Ensure you're using a recent Rust version:
```bash
docker build --no-cache -t debugger-mcp:latest .
```

### Python debugpy Not Found

The image includes Python + debugpy. If missing:
```bash
docker run -it debugger-mcp:latest sh
# Inside container:
python3 -m debugpy --version
```

### Permission Denied

Ensure volumes are readable by UID 1000 (mcpuser):
```bash
chmod -R 755 ./workspace
```

### STDIO Communication Issues

Docker requires `-i` (interactive) flag for STDIO:
```bash
# Correct
docker run -i debugger-mcp:latest

# Wrong (won't work)
docker run debugger-mcp:latest
```

## Size Optimization

Current image size: ~50MB

Further optimization possible:
- Remove Python if only using other languages: ~5MB
- Use `scratch` base (requires all dependencies static): ~2MB
- Use UPX compression on binary: ~1MB

## Development

### Build and Test Locally

```bash
# Build
docker build -t debugger-mcp:dev .

# Test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  docker run -i debugger-mcp:dev

# Expected: JSON response with server info
```

### Debug Build

Create a debug image for troubleshooting:

```bash
# Add to Dockerfile (after USER mcpuser)
USER root
RUN apk add --no-cache bash vim
USER mcpuser

# Build
docker build -t debugger-mcp:debug .

# Interactive shell
docker run -it debugger-mcp:debug sh
```

## Best Practices

1. **Always use `-i` flag** for STDIO communication
2. **Mount workspaces read-only** when possible: `-v $(pwd):/workspace:ro`
3. **Use specific version tags** instead of `latest` in production
4. **Set resource limits** in production:
   ```bash
   docker run -i \
     --memory=512m \
     --cpus=1 \
     debugger-mcp:latest
   ```

## FAQ

**Q: Can I use this with other languages besides Python?**
A: Yes, but you'll need to add their debug adapters to the Dockerfile. For example, add Ruby + rdbg.

**Q: Why Alpine instead of scratch?**
A: Python requires glibc/musl and other dependencies. Alpine provides these while remaining minimal.

**Q: Can I run multiple instances?**
A: Yes, each container is isolated. Use different container names.

**Q: How do I update?**
A: Rebuild the image: `docker build -t debugger-mcp:latest . && docker-compose up -d`

## Resources

- [Dockerfile](../Dockerfile)
- [docker-compose.yml](../docker-compose.yml)
- [Multi-stage builds](https://docs.docker.com/build/building/multi-stage/)
- [Alpine Linux](https://alpinelinux.org/)

---

**Image**: debugger-mcp:latest
**Base**: Alpine Linux 3.21
**Size**: ~50MB
**Security**: Non-root user, minimal surface
