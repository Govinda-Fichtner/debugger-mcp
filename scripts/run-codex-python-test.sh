#!/bin/bash
# Simplified Codex Python Integration Test
# Runs Codex directly without Docker-in-Docker complexity

set -e

echo "🚀 Codex Python Integration Test (Simplified)"
echo "=============================================="
echo ""

# Determine workspace root (script location is /workspace/scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$WORKSPACE_ROOT"

echo "📁 Workspace: $WORKSPACE_ROOT"
echo ""

# 1. Check prerequisites
echo "📋 Step 1: Checking prerequisites..."
echo "──────────────────────────────────────"

if [ -z "$OPENAI_API_KEY" ]; then
    echo "❌ OPENAI_API_KEY not set"
    exit 1
fi
echo "✅ OPENAI_API_KEY is set (length: ${#OPENAI_API_KEY})"

if ! command -v codex &> /dev/null; then
    echo "❌ Codex CLI not found"
    exit 1
fi
CODEX_VERSION=$(codex --version 2>&1)
echo "✅ Codex CLI: $CODEX_VERSION"

if ! command -v python3 &> /dev/null; then
    echo "❌ python3 not found"
    exit 1
fi
PYTHON_VERSION=$(python3 --version 2>&1)
echo "✅ python3: $PYTHON_VERSION"

# Check for debugpy module
if ! python3 -c "import debugpy" &> /dev/null; then
    echo "⚠️  Warning: debugpy module not found (Codex may use its own)"
else
    echo "✅ debugpy module available"
fi

# Check for binary in release or debug directory
BINARY_PATH_RELEASE="$WORKSPACE_ROOT/target/release/debugger_mcp"
BINARY_PATH_DEBUG="$WORKSPACE_ROOT/target/debug/debugger_mcp"

if [ -f "$BINARY_PATH_RELEASE" ]; then
    BINARY_PATH="$BINARY_PATH_RELEASE"
    echo "✅ MCP server binary (release): $BINARY_PATH"
elif [ -f "$BINARY_PATH_DEBUG" ]; then
    BINARY_PATH="$BINARY_PATH_DEBUG"
    echo "✅ MCP server binary (debug): $BINARY_PATH"
else
    echo "❌ MCP server binary not found at either:"
    echo "   - $BINARY_PATH_RELEASE"
    echo "   - $BINARY_PATH_DEBUG"
    echo "   Build with: cargo build --release (or cargo build for debug)"
    exit 1
fi

echo ""

# 2. Verify Python fixture exists
echo "📝 Step 2: Verifying Python test fixture..."
echo "──────────────────────────────────────"

FIXTURE_SOURCE="$WORKSPACE_ROOT/tests/fixtures/fizzbuzz.py"

if [ ! -f "$FIXTURE_SOURCE" ]; then
    echo "❌ Python fixture not found: $FIXTURE_SOURCE"
    exit 1
fi

echo "✅ Fixture verified: $FIXTURE_SOURCE"
echo ""

# 3. Login to Codex
echo "🔑 Step 3: Logging in to Codex..."
echo "──────────────────────────────────────"

LOGIN_OUTPUT=$(echo "$OPENAI_API_KEY" | codex login --with-api-key 2>&1)
LOGIN_EXIT=$?

if [ $LOGIN_EXIT -ne 0 ]; then
    echo "❌ Codex login failed:"
    echo "$LOGIN_OUTPUT"
    exit 1
fi

echo "✅ Logged in successfully"
echo ""

# 4. Register MCP server (NO Docker-in-Docker!)
echo "🔧 Step 4: Registering MCP server with Codex..."
echo "──────────────────────────────────────"

MCP_SERVER_NAME="debugger-test-python-simplified"

# Remove any existing registration
codex mcp remove "$MCP_SERVER_NAME" 2>/dev/null || true

# Register pointing directly to local binary
REGISTER_OUTPUT=$(codex mcp add "$MCP_SERVER_NAME" -- "$BINARY_PATH" serve 2>&1)
REGISTER_EXIT=$?

if [ $REGISTER_EXIT -ne 0 ]; then
    echo "⚠️  MCP registration had issues (may be expected):"
    echo "$REGISTER_OUTPUT"
else
    echo "✅ MCP server registered as: $MCP_SERVER_NAME"
fi

# Give MCP server a moment to initialize
echo "⏳ Waiting 2 seconds for MCP server to initialize..."
sleep 2

echo ""

# 5. Create debugging prompt
echo "📝 Step 5: Creating debugging prompt..."
echo "──────────────────────────────────────"

PROMPT_FILE="$WORKSPACE_ROOT/debug_prompt.md"

cat > "$PROMPT_FILE" << EOF
# Python Debugging Test with Codex

**IMPORTANT**: You have access to an MCP server called \`debugger-test-python-simplified\` that provides debugging tools.

**Step 1: First, list ALL available tools and resources from the \`debugger-test-python-simplified\` MCP server**
Use the list_mcp_tools MCP tool to show what debugging capabilities are available.

**Step 2: Then perform the debugging test:**
1. Use the debugger tools from \`debugger-test-python-simplified\` to start a debugging session for: $FIXTURE_SOURCE
2. Set a breakpoint at line 5
3. Continue execution until the breakpoint is hit
4. Get the stack trace
5. Evaluate a variable
6. Disconnect the debugging session

IMPORTANT: At the end of testing, **USE THE WRITE TOOL** to create a file named 'test-results.json' with this EXACT format:
\`\`\`json
{
  "test_run": {
    "language": "python",
    "timestamp": "<current ISO 8601 timestamp>",
    "overall_success": <true if all operations succeeded, false otherwise>,
    "ai_client": "codex"
  },
  "operations": {
    "session_started": <true/false>,
    "breakpoint_set": <true/false>,
    "breakpoint_verified": <true/false>,
    "execution_continued": <true/false>,
    "stopped_at_breakpoint": <true/false>,
    "stack_trace_retrieved": <true/false>,
    "variable_evaluated": <true/false>,
    "session_disconnected": <true/false>
  },
  "errors": [
    {
      "operation": "<operation name>",
      "message": "<error message>"
    }
  ]
}
\`\`\`

Set each boolean to true only if that specific operation completed successfully.
Add errors array entries for any failures encountered.

Also **USE THE WRITE TOOL** to create mcp_protocol_log.md documenting all interactions.

**CRITICAL**: After creating both files:
1. Use the Read tool to read back test-results.json
2. Display the full content to verify it was written correctly
3. Do NOT just claim you created the files - actually show the content!
EOF

echo "✅ Prompt created: $PROMPT_FILE"
echo ""

# 6. Run Codex
echo "🤖 Step 6: Running Codex debugging session..."
echo "──────────────────────────────────────"

PROMPT_CONTENT=$(cat "$PROMPT_FILE")

# Use timeout to prevent hanging (2 minutes max)
echo "Running with 2-minute timeout..."
CODEX_OUTPUT=$(timeout 120 codex exec \
    --json \
    --dangerously-bypass-approvals-and-sandbox \
    --output-last-message "$WORKSPACE_ROOT/codex-last-message.txt" \
    "$PROMPT_CONTENT" 2>&1 || echo "CODEX_EXIT_CODE=$?")

CODEX_EXIT_CODE=$?

echo "📊 Codex output:"
echo "────────────────────────────────────────────────────────"
echo "$CODEX_OUTPUT"
echo "────────────────────────────────────────────────────────"
echo ""

if [ $CODEX_EXIT_CODE -eq 124 ]; then
    echo "⚠️  Warning: Codex execution timed out after 120 seconds"
    echo "   This may indicate:"
    echo "   - MCP server communication issues"
    echo "   - Codex waiting for interactive input despite --json flag"
    echo "   - Debugger hanging"
    echo ""
elif [ $CODEX_EXIT_CODE -ne 0 ]; then
    echo "⚠️  Warning: Codex exited with code $CODEX_EXIT_CODE"
    echo ""
fi

# 7. Validate output files
echo "📊 Step 7: Validating results..."
echo "──────────────────────────────────────"

TEST_RESULTS="$WORKSPACE_ROOT/test-results.json"
PROTOCOL_LOG="$WORKSPACE_ROOT/mcp_protocol_log.md"

# Check if test-results.json exists and is valid
if [ -f "$TEST_RESULTS" ]; then
    FILE_SIZE=$(stat -f%z "$TEST_RESULTS" 2>/dev/null || stat -c%s "$TEST_RESULTS" 2>/dev/null || echo "0")

    if [ "$FILE_SIZE" -gt 0 ]; then
        # Try to validate JSON
        if command -v jq &> /dev/null; then
            if jq empty "$TEST_RESULTS" 2>/dev/null; then
                echo "✅ test-results.json: Valid JSON ($FILE_SIZE bytes)"
            else
                echo "⚠️  test-results.json: Invalid JSON"
            fi
        else
            echo "✅ test-results.json: Created ($FILE_SIZE bytes)"
        fi
    else
        echo "⚠️  test-results.json: Empty file"
    fi
else
    echo "⚠️  test-results.json: Not found, attempting extraction..."

    # Try to extract from Codex output
    if echo "$CODEX_OUTPUT" | grep -q "test_run"; then
        # Extract JSON block
        echo "$CODEX_OUTPUT" | sed -n '/```json/,/```/p' | sed '1d;$d' > "$TEST_RESULTS" 2>/dev/null || true

        if [ -f "$TEST_RESULTS" ] && [ -s "$TEST_RESULTS" ]; then
            echo "✅ test-results.json: Extracted from output"
        fi
    fi
fi

# Check protocol log
if [ -f "$PROTOCOL_LOG" ]; then
    FILE_SIZE=$(stat -f%z "$PROTOCOL_LOG" 2>/dev/null || stat -c%s "$PROTOCOL_LOG" 2>/dev/null || echo "0")
    echo "✅ mcp_protocol_log.md: Created ($FILE_SIZE bytes)"
else
    echo "⚠️  mcp_protocol_log.md: Not found"
fi

echo ""

# 8. Cleanup
echo "🧹 Step 8: Cleanup..."
echo "──────────────────────────────────────"

codex mcp remove "$MCP_SERVER_NAME" 2>/dev/null || true
rm -f "$PROMPT_FILE" || true

echo "✅ Cleanup complete"
echo ""

# 9. Final summary
echo "═══════════════════════════════════════"
echo "📊 Test Summary"
echo "═══════════════════════════════════════"

if [ -f "$TEST_RESULTS" ] && [ -s "$TEST_RESULTS" ]; then
    echo "✅ test-results.json ready for CI artifact collection"

    # Show summary if jq available
    if command -v jq &> /dev/null; then
        echo ""
        echo "Test run summary:"
        jq -r '.test_run // "No test_run data"' "$TEST_RESULTS" 2>/dev/null || true
    fi

    EXIT_CODE=0
else
    echo "❌ test-results.json missing or empty"
    EXIT_CODE=1
fi

if [ -f "$PROTOCOL_LOG" ]; then
    echo "✅ mcp_protocol_log.md ready"
else
    echo "⚠️  mcp_protocol_log.md missing"
fi

echo ""
echo "🎉 Codex Python integration test completed!"

exit $EXIT_CODE
