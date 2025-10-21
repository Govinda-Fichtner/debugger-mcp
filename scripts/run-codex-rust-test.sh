#!/bin/bash
# Simplified Codex Rust Integration Test
# Runs Codex directly without Docker-in-Docker complexity

set -e

echo "🚀 Codex Rust Integration Test (Simplified)"
echo "============================================="
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

if ! command -v rustc &> /dev/null; then
    echo "❌ rustc not found"
    exit 1
fi
RUSTC_VERSION=$(rustc --version 2>&1)
echo "✅ rustc: $RUSTC_VERSION"

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

if ! command -v lldb &> /dev/null; then
    echo "❌ LLDB not found (required for Rust debugging)"
    exit 1
fi
echo "✅ LLDB available"

echo ""

# 2. Compile Rust fixture
echo "🔨 Step 2: Compiling Rust test fixture..."
echo "──────────────────────────────────────"

FIXTURE_SOURCE="$WORKSPACE_ROOT/tests/fixtures/fizzbuzz.rs"
OUTPUT_DIR="$WORKSPACE_ROOT/tests/fixtures/target"
FIZZBUZZ_BINARY="$OUTPUT_DIR/fizzbuzz"

mkdir -p "$OUTPUT_DIR"

# Remove old binary to ensure fresh compilation
if [ -f "$FIZZBUZZ_BINARY" ]; then
    rm -f "$FIZZBUZZ_BINARY"
    echo "🗑️  Removed cached binary"
fi

echo "Compiling: $FIXTURE_SOURCE"
echo "Output: $FIZZBUZZ_BINARY"

# Compile with debug symbols and no optimizations
rustc "$FIXTURE_SOURCE" \
    -g \
    -C opt-level=0 \
    -o "$FIZZBUZZ_BINARY"

if [ ! -f "$FIZZBUZZ_BINARY" ]; then
    echo "❌ Compilation failed"
    exit 1
fi

echo "✅ Compilation successful"

# Verify debug symbols (if readelf available)
if command -v readelf &> /dev/null; then
    if readelf -S "$FIZZBUZZ_BINARY" | grep -q ".debug_info"; then
        echo "✅ Debug symbols verified (.debug_info section present)"
    else
        echo "⚠️  Warning: Debug symbols may be missing"
    fi
fi

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

MCP_SERVER_NAME="debugger-rust"

# Check if server already exists (Docker container)
if codex mcp list 2>&1 | grep -q "^$MCP_SERVER_NAME"; then
    echo "✅ MCP server '$MCP_SERVER_NAME' already registered (using existing Docker container)"
else
    echo "⚠️  MCP server '$MCP_SERVER_NAME' not found, registering local binary..."
    # Register pointing directly to local binary
    REGISTER_OUTPUT=$(codex mcp add "$MCP_SERVER_NAME" -- "$BINARY_PATH" serve 2>&1)
    REGISTER_EXIT=$?

    if [ $REGISTER_EXIT -ne 0 ]; then
        echo "❌ MCP registration failed:"
        echo "$REGISTER_OUTPUT"
        exit 1
    else
        echo "✅ MCP server registered as: $MCP_SERVER_NAME"
    fi

    # Give MCP server a moment to initialize
    echo "⏳ Waiting 2 seconds for MCP server to initialize..."
    sleep 2
fi

echo ""

# 5. Create debugging prompt
echo "📝 Step 5: Creating debugging prompt..."
echo "──────────────────────────────────────"

PROMPT_FILE="$WORKSPACE_ROOT/debug_prompt.md"

cat > "$PROMPT_FILE" << EOF
# Rust Debugging Test with Codex - Enhanced Version

**IMPORTANT**: You have access to an MCP server called \`debugger-rust\` that provides debugging tools.

---

## PHASE 1: MCP Resource Discovery

**Before starting any debugging operations, perform thorough discovery:**

### Step 1A: List Available Resources
Call \`list_mcp_resources\` on the \`debugger-rust\` MCP server to discover:
- Session management resources (debugger://sessions)
- Workflow templates (debugger://workflows)
- State machine documentation
- Any other available resources

Document ALL discovered resources with their URIs and descriptions.

### Step 1B: List Available Tools
Call \`list_mcp_tools\` to enumerate all debugging capabilities:
- Session management tools (debugger_start, debugger_disconnect, etc.)
- Execution control tools (debugger_continue, debugger_step_*, etc.)
- Inspection tools (debugger_stack_trace, debugger_evaluate, etc.)
- State query tools (debugger_session_state, debugger_wait_for_stop, etc.)

Document each tool name and its purpose.

**Why this matters**: Understanding available resources and tools helps plan an effective debugging workflow and verifies the MCP server is properly configured.

---

## PHASE 2: Debugging Workflow

**Execute the following steps IN ORDER, documenting EVERY operation:**

### Step 2.1: Start Debug Session ✓
**Tool**: \`debugger_start\`
**Parameters**:
\`\`\`json
{
  "language": "rust",
  "program": "$FIZZBUZZ_BINARY",
  "stopOnEntry": true
}
\`\`\`
**Expected Response**: Session ID and status "started"
**Verification**: Confirm you received a valid session ID (UUID format)

### Step 2.2: Wait for Entry Point + Verify State ✓
**Tool**: \`debugger_wait_for_stop\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id-from-step-2.1>",
  "timeoutMs": 5000
}
\`\`\`
**Expected Response**: State "Stopped" with reason "entry" or "exception"

**THEN IMMEDIATELY call** \`debugger_session_state\`:
\`\`\`json
{
  "sessionId": "<session-id>"
}
\`\`\`
**Why**: Verify the session is in a stopped state before setting breakpoints
**Document**: Current state, stop reason, and thread ID

### Step 2.3: Set Breakpoint ✓
**Tool**: \`debugger_set_breakpoint\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>",
  "sourcePath": "/workspace/tests/fixtures/fizzbuzz.rs",
  "line": 5
}
\`\`\`
**Expected Response**: \`verified: true\`, confirming breakpoint is set at line 5
**Verification**: Check that line number and source path match your request
**Note**: Line 5 is the first if statement in the fizzbuzz function

### Step 2.4: Continue Execution ✓
**Tool**: \`debugger_continue\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>"
}
\`\`\`
**Expected Response**: \`status: "continued"\`
**Verification**: Session should transition from Stopped → Running state

### Step 2.5: Wait for Breakpoint Hit + Verify State ✓
**Tool**: \`debugger_wait_for_stop\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>",
  "timeoutMs": 5000
}
\`\`\`
**Expected Response**: State "Stopped" with reason "breakpoint"

**THEN IMMEDIATELY call** \`debugger_session_state\`:
\`\`\`json
{
  "sessionId": "<session-id>"
}
\`\`\`
**Why**: Confirm we stopped at the breakpoint, not due to an error
**Document**: Stop reason, thread ID, and any additional details

### Step 2.6: Retrieve Stack Trace ✓
**Tool**: \`debugger_stack_trace\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>"
}
\`\`\`
**Expected Response**: Array of stack frames with at least 2 frames
**Verification**:
- Top frame should be \`fizzbuzz::fizzbuzz\` at line 5
- Caller frame should be \`fizzbuzz::main\` at line 18
**Document**: How many frames total? What are the top 3 frames?

### Step 2.7: Evaluate Variable ✓
**Tool**: \`debugger_evaluate\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>",
  "expression": "n",
  "frameId": <frame-id-from-stack-trace>
}
\`\`\`
**Expected Response**: \`result: "1"\` (first iteration of fizzbuzz loop)
**Verification**: Value should be 1 (i32 type)
**Context**: Variable 'n' is the parameter to the fizzbuzz function

### Step 2.8: Disconnect Session ✓
**Tool**: \`debugger_disconnect\`
**Parameters**:
\`\`\`json
{
  "sessionId": "<session-id>"
}
\`\`\`
**Expected Response**: \`status: "disconnected"\`
**Verification**: Clean termination without errors

---

## PHASE 3: Documentation Requirements

### test-results.json Format

**USE THE WRITE TOOL** to create 'test-results.json' with this EXACT format:
\`\`\`json
{
  "test_run": {
    "language": "rust",
    "timestamp": "<current ISO 8601 timestamp>",
    "overall_success": <true if ALL operations succeeded, false if ANY failed>,
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

**Set each boolean to true ONLY if that specific operation completed successfully.**
**Add errors array entries for ANY failures encountered (include operation name and error message).**

### STRATEGY 3: SPECIFIC OPERATION SEQUENCE WITH CONTEXT

**Your Task**: Debug the Rust fizzbuzz program at \`$FIZZBUZZ_BINARY\` using these MCP tools.

**MCP Server**: \`debugger-rust\`
**Source File**: \`/workspace/tests/fixtures/fizzbuzz.rs\`

---

## Required Operation Sequence

Perform these operations IN ORDER. Each step builds on the previous one:

### 1. Discover Available Tools
\`\`\`
list_mcp_resources(server="debugger-rust")
list_mcp_tools
\`\`\`
**Why**: Verify the MCP server is responding correctly.

### 2. Start Debug Session
\`\`\`
debugger_start(
  language="rust",
  program="$FIZZBUZZ_BINARY",
  stopOnEntry=true
)
\`\`\`
**Expected**: Returns a sessionId (UUID format).
**Why**: Launches the binary under debugger control, paused at entry point.

### 3. Wait for Entry Point
\`\`\`
debugger_wait_for_stop(
  sessionId=<id-from-step-2>,
  timeoutMs=5000
)
\`\`\`
**Expected**: State "Stopped", reason "entry" or "exception".
**Why**: Program stops at entry before executing user code.

### 4. Verify Session State (NEW - you're not doing this currently)
\`\`\`
debugger_session_state(sessionId=<id>)
\`\`\`
**Expected**: Confirms state is "Stopped".
**Why**: State verification before setting breakpoints ensures debugging workflow correctness.

### 5. Set Breakpoint
\`\`\`
debugger_set_breakpoint(
  sessionId=<id>,
  sourcePath="/workspace/tests/fixtures/fizzbuzz.rs",
  line=5
)
\`\`\`
**Expected**: \`verified: true\`.
**Why**: Line 5 is the first conditional in fizzbuzz function (\`n % 15 == 0\`).

### 6. Continue Execution
\`\`\`
debugger_continue(sessionId=<id>)
\`\`\`
**Expected**: Status "continued".
**Why**: Resume execution until breakpoint or program exit.

### 7. Wait for Breakpoint Hit
\`\`\`
debugger_wait_for_stop(
  sessionId=<id>,
  timeoutMs=5000
)
\`\`\`
**Expected**: State "Stopped", reason "breakpoint".
**Why**: Program stops at line 5 when fizzbuzz(1) is called.

### 8. Verify State After Breakpoint (NEW)
\`\`\`
debugger_session_state(sessionId=<id>)
\`\`\`
**Expected**: State "Stopped", thread ID available.
**Why**: Confirms we're at the breakpoint before inspection.

### 9. Get Stack Trace
\`\`\`
debugger_stack_trace(sessionId=<id>)
\`\`\`
**Expected**: Top frame at fizzbuzz.rs:5 in function \`fizzbuzz::fizzbuzz\`.
**Why**: Verify call stack structure.

### 10. Evaluate Variable
\`\`\`
debugger_evaluate(
  sessionId=<id>,
  frameId=<from-stack-trace>,
  expression="n"
)
\`\`\`
**Expected**: Result "1" (first call to fizzbuzz).
**Why**: Verify variable inspection works.

### 11. Disconnect
\`\`\`
debugger_disconnect(sessionId=<id>)
\`\`\`
**Expected**: Status "disconnected".
**Why**: Clean session termination.

---

## Success Criteria

1. ✅ All 11 operations complete successfully
2. ✅ Steps 4 and 8 (debugger_session_state) are included - **you're currently skipping these**
3. ✅ Steps 3 and 7 use \`timeoutMs: 5000\` parameter - **currently missing**
4. ✅ Breakpoint verified=true
5. ✅ Variable evaluation returns "1"

---

## Output Files

Create two files (your concise format is fine):

1. **test-results.json**: JSON with test_run and operations status
2. **mcp_protocol_log.md**: Numbered list of operations (your current format works perfectly)
EOF

echo "✅ Prompt created: $PROMPT_FILE"
echo ""

# 6. Run Codex
echo "🤖 Step 6: Running Codex debugging session..."
echo "──────────────────────────────────────"

# Clean up old artifacts BEFORE running Codex to ensure fresh validation
echo "🧹 Removing stale artifacts from previous runs..."
rm -f "$WORKSPACE_ROOT/test-results.json"
rm -f "$WORKSPACE_ROOT/mcp_protocol_log.md"
rm -f "$WORKSPACE_ROOT/codex-last-message.txt"
echo "✅ Old artifacts removed"
echo ""

PROMPT_CONTENT=$(cat "$PROMPT_FILE")

# Run Codex (with generous 10-minute timeout)
echo "Running with default Codex model (no config parameters), 10-minute timeout and debug logging..."
CODEX_OUTPUT=$(CODEX_LOG_LEVEL=debug timeout 600 codex exec \
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
echo "🎉 Codex Rust integration test completed!"

exit $EXIT_CODE
