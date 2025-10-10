#!/bin/bash
# Debug Claude Code MCP connection issue in Docker

echo "========================================="
echo "Claude Code MCP Connection Debug"
echo "========================================="
echo ""

# Build the server
echo "📦 Building MCP server..."
cargo build --release 2>&1 | tail -n 3
echo "✅ Build complete"
echo ""

docker run --rm \
  -v "$(pwd)":/workspace \
  debugger-mcp:integration-tests \
  bash << 'EOF'
set -e

echo "📋 Inside Docker Container"
echo "════════════════════════════════════════"
echo ""

# Test 1: Can we run the binary directly?
echo "1️⃣  Testing binary execution..."
/workspace/target/release/debugger_mcp --version 2>&1 || echo "❌ Binary execution failed"
echo ""

# Test 2: Can we run with serve subcommand?
echo "2️⃣  Testing 'serve' subcommand..."
timeout 2 /workspace/target/release/debugger_mcp serve 2>&1 || echo "(timed out - expected)"
echo ""

# Test 3: Check file permissions
echo "3️⃣  Checking file permissions..."
ls -la /workspace/target/release/debugger_mcp
echo ""

# Test 4: Check if it's the right architecture
echo "4️⃣  Checking binary architecture..."
file /workspace/target/release/debugger_mcp
echo ""

# Test 5: Try to get ldd info
echo "5️⃣  Checking dependencies..."
ldd /workspace/target/release/debugger_mcp 2>&1 | head -n 10
echo ""

# Test 6: Register and check Claude logs
echo "6️⃣  Registering with Claude and checking logs..."
claude mcp add-json debugger-test-debug '{"command":"/workspace/target/release/debugger_mcp","args":["serve"]}'
echo "Registered. Now checking connection..."
echo ""

# Give it a moment
sleep 2

echo "7️⃣  Claude MCP list output..."
claude mcp list
echo ""

# Test 7: Check Claude's config
echo "8️⃣  Claude config location..."
echo "HOME=$HOME"
ls -la "$HOME/.config/claude" 2>&1 || echo "No .config/claude directory"
ls -la "$HOME/.claude" 2>&1 || echo "No .claude directory"
echo ""

# Test 8: Try our direct STDIO test
echo "9️⃣  Running direct STDIO test..."
echo "════════════════════════════════════════"
python3 /workspace/tests/test_mcp_stdio.py 2>&1 | grep -E "(SENDING|Found|✅|❌)"
echo "════════════════════════════════════════"
echo ""

# Cleanup
claude mcp remove debugger-test-debug 2>/dev/null || true

echo "========================================="
echo "Debug complete"
echo "========================================="
EOF
