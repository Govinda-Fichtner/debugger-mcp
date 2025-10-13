#!/bin/bash
# Test MCP server registration with Claude Code CLI locally in Docker
# This reproduces the CI environment without needing an API key

set -e

echo "========================================="
echo "Claude Code MCP Registration Test (Local)"
echo "========================================="
echo ""

# Build the server
echo "📦 Building MCP server..."
cargo build --release
echo "✅ Build complete"
echo ""

BINARY="./target/release/debugger_mcp"

# Test inside Docker (same as CI)
echo "🐳 Starting Docker container (same as CI)..."
echo ""

docker run --rm \
  -v "$(pwd)":/workspace \
  debugger-mcp:integration-tests \
  bash -c '
set -e

echo "📋 Inside Docker Container"
echo "═══════════════════════════════════════"
echo ""

# Check Claude CLI
echo "1️⃣  Checking Claude CLI..."
claude --version
echo "✅ Claude CLI available"
echo ""

# Register MCP server
echo "2️⃣  Registering MCP server..."
claude mcp add-json debugger-test-local "{\"command\":\"/workspace/target/release/debugger_mcp\",\"args\":[\"serve\"]}"
echo "✅ MCP server registered"
echo ""

# List MCP servers
echo "3️⃣  Listing MCP servers..."
echo "═══════════════════════════════════════"
claude mcp list
echo "═══════════════════════════════════════"
echo ""

# Check if connected
echo "4️⃣  Checking connection status..."
if claude mcp list | grep -q "✓ Connected"; then
    echo "✅ MCP server shows as Connected"
else
    echo "❌ MCP server NOT connected"
    exit 1
fi
echo ""

# Try to query tools using our STDIO test
echo "5️⃣  Testing direct STDIO communication..."
echo "═══════════════════════════════════════"
python3 /workspace/tests/test_mcp_stdio.py 2>&1 | grep -A 5 "Found.*tools"
echo "═══════════════════════════════════════"
echo ""

# Cleanup
echo "6️⃣  Cleaning up..."
claude mcp remove debugger-test-local 2>/dev/null || true
echo "✅ Cleanup complete"
echo ""

echo "========================================="
echo "✅ All checks passed!"
echo "========================================="
'
