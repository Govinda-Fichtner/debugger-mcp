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

# Stage 2: Create minimal runtime image
FROM alpine:3.21

# Install runtime dependencies
# Ruby and debug gem are needed for Ruby debugging support
RUN apk add --no-cache \
    ruby \
    ruby-dev \
    ruby-bundler \
    && gem install debug --no-document \
    && rm -rf /root/.gem/ruby/*/cache

# Create non-root user
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser

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
LABEL org.opencontainers.image.title="debugger-mcp-ruby"
LABEL org.opencontainers.image.description="DAP MCP Server - Ruby Debugging Support"
LABEL org.opencontainers.image.source="https://github.com/Govinda-Fichtner/debugger-mcp"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.variant="ruby"
