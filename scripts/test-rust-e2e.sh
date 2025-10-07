#!/bin/bash
set -e

echo "🧪 Running Rust E2E tests with MCP server in Docker..."
echo ""

# Build the Docker image if needed
echo "📦 Building Docker image..."
docker build -f Dockerfile.rust -t mcp-debugger-rust:latest . > /dev/null 2>&1
echo "✅ Docker image built"
echo ""

# Start the MCP server in a container with test fixtures mounted
echo "🚀 Starting MCP server in Docker..."
CONTAINER_ID=$(docker run -d \
  --name debugger-mcp-test \
  -v "$(pwd)/tests/fixtures:/workspace/fizzbuzz-rust-test" \
  mcp-debugger-rust:latest \
  /usr/local/bin/debugger_mcp serve)

echo "✅ MCP server started (container: ${CONTAINER_ID:0:12})"
echo ""

# Give the server a moment to start
sleep 2

# Function to cleanup on exit
cleanup() {
  echo ""
  echo "🧹 Cleaning up..."
  docker stop debugger-mcp-test > /dev/null 2>&1 || true
  docker rm debugger-mcp-test > /dev/null 2>&1 || true
  echo "✅ Cleanup complete"
}
trap cleanup EXIT

# Send a simple test request to verify the server is running
echo "📡 Testing MCP server connectivity..."
echo '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | \
  docker exec -i debugger-mcp-test /usr/local/bin/debugger_mcp serve 2>&1 | \
  head -1 > /dev/null

if [ $? -eq 0 ]; then
  echo "✅ MCP server is responding"
else
  echo "❌ MCP server not responding"
  exit 1
fi

echo ""
echo "🧪 Running integration tests against MCP server..."
echo ""

# Run the Rust integration tests
# These will use the SessionManager to create sessions via the MCP interface
cargo test --test test_rust_integration -- --ignored --nocapture

echo ""
echo "✅ All E2E tests passed!"
echo ""
