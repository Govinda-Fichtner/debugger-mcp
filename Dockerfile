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

# Build release binary with static linking
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Create minimal runtime image
FROM alpine:3.21

# Install runtime dependencies
# Python and debugpy are needed for Python debugging support
RUN apk add --no-cache \
    python3 \
    py3-pip \
    && pip3 install --no-cache-dir debugpy \
    && rm -rf /root/.cache

# Create non-root user
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser

# Copy binary from builder
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/debugger_mcp /usr/local/bin/debugger_mcp

# Set ownership
RUN chown mcpuser:mcpuser /usr/local/bin/debugger_mcp

# Switch to non-root user
USER mcpuser

# Set working directory
WORKDIR /workspace

# Default command
CMD ["debugger_mcp", "serve"]

# Metadata
LABEL org.opencontainers.image.title="debugger-mcp"
LABEL org.opencontainers.image.description="DAP MCP Server - Debug Adapter Protocol for AI Agents"
LABEL org.opencontainers.image.source="https://github.com/Govinda-Fichtner/debugger-mcp"
LABEL org.opencontainers.image.version="0.1.0"
