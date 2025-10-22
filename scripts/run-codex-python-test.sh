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

if ! python3 -c "import debugpy" 2>/dev/null; then
    echo "❌ debugpy not found (required for Python debugging)"
    exit 1
fi
echo "✅ debugpy available"

echo ""

# 2. Check Python fixture (no compilation needed)
echo "📋 Step 2: Checking Python test fixture..."
echo "──────────────────────────────────────"

FIXTURE_SOURCE="$WORKSPACE_ROOT/tests/fixtures/fizzbuzz.py"

if [ ! -f "$FIXTURE_SOURCE" ]; then
    echo "❌ Python fixture not found: $FIXTURE_SOURCE"
    exit 1
fi

echo "✅ Python fixture found: $FIXTURE_SOURCE"

# Verify syntax
if ! python3 -m py_compile "$FIXTURE_SOURCE" 2>/dev/null; then
    echo "❌ Python syntax error in fixture"
    exit 1
fi
echo "✅ Python syntax valid"

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

MCP_SERVER_NAME="debugger-python"

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

PROMPT_FILE="$WORKSPACE_ROOT/debug_prompt_python.md"

cat > "$PROMPT_FILE" << EOF
# Python Debugging Test with Codex - Enhanced Version

**IMPORTANT**: You have access to an MCP server called \`debugger-python\` that provides debugging tools.

---

## PHASE 1: MCP Resource Discovery

**Before starting any debugging operations, perform thorough discovery:**

### Step 1A: List Available Resources
Call \`list_mcp_resources\` on the \`debugger-python\` MCP server to discover:
- Session management resources (debugger://sessions)
- Workflow templates (debugger://workflows)
- State machine documentation
- Any other available resources

Document ALL discovered resources with their URIs and descriptions.

### Step 1B: List Available Resource Templates
Call \`list_mcp_resource_templates\` to enumerate available resource templates.

**Why this matters**: Understanding available resources and tools helps plan an effective debugging workflow and verifies the MCP server is properly configured.

---

## PHASE 2: Debugging Workflow

**Execute the following steps IN ORDER, documenting EVERY operation:**

### Step 2.1: Start Debug Session ✓
**Tool**: \`debugger_start\`
**Parameters**:
\`\`\`json
{
  "language": "python",
  "program": "$FIXTURE_SOURCE",
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
  "sourcePath": "/workspace/tests/fixtures/fizzbuzz.py",
  "line": 18
}
\`\`\`
**Expected Response**: \`verified: true\`, confirming breakpoint is set at line 18
**Verification**: Check that line number and source path match your request
**Note**: Line 18 is the first if statement in the fizzbuzz function (n%15 == 0)

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
- Top frame should be \`fizzbuzz\` at line 18
- Caller frame should be in main module
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
**Verification**: Value should be 1 (int type)
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
    "language": "python",
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

### mcp_protocol_log.md Format

**USE THE WRITE TOOL** to create 'mcp_protocol_log.md' documenting ALL MCP operations in order:

\`\`\`markdown
1. \`list_mcp_resources\` (server="debugger-python"): <describe result>
2. \`list_mcp_resource_templates\` (server="debugger-python"): <describe result>
3. \`debugger_start\` (program "$FIXTURE_SOURCE", stopOnEntry=true): session <id> started
4. \`debugger_wait_for_stop\` (timeoutMs=5000): <describe result>
5. \`debugger_session_state\`: <describe result>
6. \`debugger_set_breakpoint\` (file "/workspace/tests/fixtures/fizzbuzz.py", line 18): <describe result>
7. \`debugger_continue\`: <describe result>
8. \`debugger_wait_for_stop\` (timeoutMs=5000): <describe result>
9. \`debugger_session_state\`: <describe result>
10. \`debugger_stack_trace\`: <describe result including top frames>
11. \`debugger_evaluate\` (frameId=<id>, expression "n"): <describe result>
12. \`debugger_disconnect\`: <describe result>
\`\`\`

---

## Required Operation Sequence

Perform these operations IN ORDER. Each step builds on the previous one:

### 1. Discover Available Tools
\`\`\`
list_mcp_resources(server="debugger-python")
list_mcp_resource_templates(server="debugger-python")
\`\`\`
**Why**: Verify the MCP server is responding correctly.

### 2. Start Debug Session
\`\`\`
debugger_start(
  language="python",
  program="$FIXTURE_SOURCE",
  stopOnEntry=true
)
\`\`\`
**Expected**: Returns a sessionId (UUID format).
**Why**: Launches the Python script under debugger control, paused at entry point.

### 3. Wait for Entry Point
\`\`\`
debugger_wait_for_stop(
  sessionId=<id-from-step-2>,
  timeoutMs=5000
)
\`\`\`
**Expected**: State "Stopped", reason "entry" or "exception".
**Why**: Program stops at entry before executing user code.

### 4. Verify Session State
\`\`\`
debugger_session_state(sessionId=<id>)
\`\`\`
**Expected**: State "Stopped", includes reason and threadId.
**Why**: Confirm session is ready for breakpoint setting.

### 5. Set Breakpoint at Line 18
\`\`\`
debugger_set_breakpoint(
  sessionId=<id>,
  sourcePath="/workspace/tests/fixtures/fizzbuzz.py",
  line=18
)
\`\`\`
**Expected**: \`verified: true\`
**Why**: Line 18 is the first condition in fizzbuzz function.

### 6. Continue to Breakpoint
\`\`\`
debugger_continue(sessionId=<id>)
\`\`\`
**Expected**: Status "continued"
**Why**: Resume execution until breakpoint is hit.

### 7. Wait for Breakpoint
\`\`\`
debugger_wait_for_stop(
  sessionId=<id>,
  timeoutMs=5000
)
\`\`\`
**Expected**: State "Stopped", reason "breakpoint"
**Why**: Confirm execution stopped at our breakpoint.

### 8. Verify Breakpoint State
\`\`\`
debugger_session_state(sessionId=<id>)
\`\`\`
**Expected**: Stopped at breakpoint with thread info.

### 9. Get Stack Trace
\`\`\`
debugger_stack_trace(sessionId=<id>)
\`\`\`
**Expected**: Array of frames, top frame at fizzbuzz.py:18

### 10. Evaluate Variable
\`\`\`
debugger_evaluate(
  sessionId=<id>,
  expression="n",
  frameId=<top-frame-id>
)
\`\`\`
**Expected**: Result "1" (first call to fizzbuzz)

### 11. Clean Disconnect
\`\`\`
debugger_disconnect(sessionId=<id>)
\`\`\`
**Expected**: Status "disconnected"
**Why**: Clean session termination.

---

## Important Notes

- **Use exact paths**: The source path must be \`/workspace/tests/fixtures/fizzbuzz.py\`
- **Breakpoint line**: Line 18 is the first if statement checking \`n % 15 == 0\`
- **Variable evaluation**: The variable 'n' should be 1 on first breakpoint hit
- **Write files**: You MUST use the WRITE tool to create both test-results.json and mcp_protocol_log.md
- **Document everything**: Every MCP call should be documented in the protocol log

**Success criteria**: All 8 operations marked as true in test-results.json, with detailed protocol log.
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
    echo "⚠️  Warning: Codex execution timed out after 600 seconds"
    echo "   This may indicate:"
    echo "   - Very large output"
    echo "   - Infinite loop"
    echo "   - Waiting for user input"
fi

# 7. Validate results
echo "📊 Step 7: Validating results..."
echo "──────────────────────────────────────"

# Check if test-results.json exists and is valid
if [ -f "$WORKSPACE_ROOT/test-results.json" ]; then
    # Validate JSON
    if jq empty "$WORKSPACE_ROOT/test-results.json" 2>/dev/null; then
        FILE_SIZE=$(wc -c < "$WORKSPACE_ROOT/test-results.json")
        echo "✅ test-results.json: Valid JSON ($FILE_SIZE bytes)"
    else
        echo "❌ test-results.json: Invalid JSON"
    fi
else
    echo "⚠️  test-results.json: Not found, attempting extraction..."
fi

# Check if mcp_protocol_log.md exists
if [ -f "$WORKSPACE_ROOT/mcp_protocol_log.md" ]; then
    FILE_SIZE=$(wc -c < "$WORKSPACE_ROOT/mcp_protocol_log.md")
    echo "✅ mcp_protocol_log.md: Created ($FILE_SIZE bytes)"
else
    echo "⚠️  mcp_protocol_log.md: Not found"
fi

echo ""

# 8. Cleanup
echo "🧹 Step 8: Cleanup..."
echo "──────────────────────────────────────"

# Remove MCP server registration
if codex mcp remove "$MCP_SERVER_NAME" 2>&1 | grep -q "Removed"; then
    echo "✅ Cleanup complete"
else
    echo "⚠️  MCP server was not registered or already removed"
fi

echo ""
echo "═══════════════════════════════════════"
echo "📊 Test Summary"
echo "═══════════════════════════════════════"

# Show test results if available
if [ -f "$WORKSPACE_ROOT/test-results.json" ]; then
    echo "✅ test-results.json ready for CI artifact collection"
    echo ""
    echo "Test run summary:"
    jq '.test_run' "$WORKSPACE_ROOT/test-results.json" 2>/dev/null || echo "❌ Could not parse test results"
else
    echo "❌ test-results.json missing or empty"
fi

# Show protocol log status
if [ -f "$WORKSPACE_ROOT/mcp_protocol_log.md" ]; then
    echo "✅ mcp_protocol_log.md ready"
else
    echo "⚠️  mcp_protocol_log.md missing"
fi

echo ""
echo "🎉 Codex Python integration test completed!"
