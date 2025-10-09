#!/bin/bash
# Test MCP server STDIO communication directly
# This script sends JSON-RPC messages to the MCP server and displays responses

set -e

echo "========================================="
echo "MCP Server STDIO Communication Test"
echo "========================================="
echo ""

# Build the server first
echo "📦 Building MCP server..."
cargo build --release
echo "✅ Build complete"
echo ""

# Path to the binary
BINARY="./target/release/debugger_mcp"

if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at $BINARY"
    exit 1
fi

echo "🚀 Starting MCP server via STDIO..."
echo "Binary: $BINARY"
echo ""

# Create a temporary directory for test files
TEST_DIR=$(mktemp -d)
echo "📁 Test directory: $TEST_DIR"

# Create a simple Python test file
cat > "$TEST_DIR/test.py" << 'EOF'
def hello(name):
    print(f"Hello, {name}!")

if __name__ == "__main__":
    hello("World")
EOF

echo "✅ Created test.py"
echo ""

# Function to send JSON-RPC message and read response
send_message() {
    local message="$1"
    local description="$2"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📤 SENDING: $description"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "$message" | jq '.'
    echo ""

    # Send the message (newline-delimited JSON)
    echo "$message"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📥 WAITING FOR RESPONSE..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
}

# Start the MCP server process
{
    # 1. Initialize
    send_message '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}' "Initialize request"

    # Wait a bit for response
    sleep 1

    # 2. List available tools
    send_message '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' "List tools request"

    # Wait a bit for response
    sleep 1

    # 3. List available resources
    send_message '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}' "List resources request"

    # Wait a bit for response
    sleep 1

    # 4. Try to start a debugging session (this will fail since debugpy may not be available, but we'll see the response)
    send_message "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"debugger_start\",\"arguments\":{\"language\":\"python\",\"program\":\"$TEST_DIR/test.py\",\"args\":[],\"stopOnEntry\":true}}}" "Start debugging session"

    # Wait for response
    sleep 2

} | "$BINARY" serve 2>&1 | while IFS= read -r line; do
    echo "📥 RECEIVED: $line"

    # Try to pretty-print if it's JSON
    if echo "$line" | jq '.' > /dev/null 2>&1; then
        echo "$line" | jq '.'
    fi
    echo ""
done

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "========================================="
echo "Test complete!"
echo "========================================="
