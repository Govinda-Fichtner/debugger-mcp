# Multi-stage build for lean production image
# Stage 1: Build the Rust binary
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Create app directory
WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary with static linking for native architecture
# Supports both x86_64 and aarch64 (ARM64)
RUN cargo build --release

# Stage 2: Create Go debugging runtime image
FROM alpine:3.21

# Install base dependencies
RUN apk add --no-cache \
    wget \
    git \
    gcc \
    musl-dev \
    && rm -rf /var/cache/apk/*

# Install Go (official binary for consistent version across architectures)
# Using Go 1.23.4 - latest stable release with excellent debugging support
RUN cd /tmp && \
    ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ]; then \
        GO_ARCH="amd64"; \
    elif [ "$ARCH" = "aarch64" ]; then \
        GO_ARCH="arm64"; \
    else \
        echo "Unsupported architecture: $ARCH" && exit 1; \
    fi && \
    wget -q https://go.dev/dl/go1.23.4.linux-${GO_ARCH}.tar.gz && \
    tar -C /usr/local -xzf go1.23.4.linux-${GO_ARCH}.tar.gz && \
    rm go1.23.4.linux-${GO_ARCH}.tar.gz

# Set Go environment variables
ENV PATH="/usr/local/go/bin:${PATH}"
ENV GOPATH="/go"
ENV PATH="${GOPATH}/bin:${PATH}"

# Install Delve (Go debugger with full DAP support)
# Using v1.23.1 - latest stable with mature DAP implementation
# This version is tested to work correctly with the MCP server
RUN go install github.com/go-delve/delve/cmd/dlv@v1.23.1 && \
    # Verify Delve installation
    dlv version && \
    # Clean up build cache
    rm -rf /root/.cache /tmp/* && \
    # Remove build tools no longer needed
    apk del wget gcc musl-dev && \
    rm -rf /var/cache/apk/*

# Verify installation and print versions
RUN echo "=== Go Debugger Installation ===" && \
    echo "Go version: $(go version)" && \
    echo "Delve version: $(dlv version)" && \
    echo "Delve location: $(which dlv)" && \
    echo "✅ Go 1.23.4 and Delve v1.23.1 installed successfully"

# Create non-root user
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser && \
    # Create GOPATH directory for mcpuser
    mkdir -p /go/bin && \
    chown -R mcpuser:mcpuser /go

# Copy binary from builder (native architecture)
COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/debugger_mcp

# Set ownership
RUN chown mcpuser:mcpuser /usr/local/bin/debugger_mcp

# Switch to non-root user
USER mcpuser

# Set working directory
WORKDIR /workspace

# Default command
CMD ["debugger_mcp", "serve"]

# Metadata
LABEL org.opencontainers.image.title="debugger-mcp-go"
LABEL org.opencontainers.image.description="DAP MCP Server - Go Debugging Support"
LABEL org.opencontainers.image.source="https://github.com/Govinda-Fichtner/debugger-mcp"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.variant="go"
