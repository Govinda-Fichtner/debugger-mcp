# Codex Docker Execution Experiment Results

**Date:** 2025-10-21
**Objective:** Determine if Codex can be executed directly in Docker without Rust test wrapper

## Key Finding: ✅ Codex CAN execute directly in Docker!

### Experiment Setup

Created a simple shell script (`scripts/test-codex-direct.sh`) that runs Codex directly:
```bash
codex exec --dangerously-bypass-approvals-and-sandbox "$PROMPT"
```

Executed inside Docker container:
```bash
docker run --rm \
  -v /workspace \
  -e OPENAI_API_KEY="..." \
  debugger-mcp:integration-tests \
  ./scripts/test-codex-direct.sh
```

### Results

**✅ SUCCESS: No TTY errors!**

Codex executed and showed:
```
OpenAI Codex v0.47.0 (research preview)
--------
workdir: /workspace
model: gpt-5-codex
provider: openai
approval: never
sandbox: danger-full-access
reasoning effort: none
reasoning summaries: auto
session id: 019a0556-5980-7250-85d7-d3905fd9276c
--------
user
Who won the football world championship in 2024?
Re-connecting... 1/5
...
ERROR: exceeded retry limit, last status: 401 Unauthorized
```

**Key observations:**
1. ✅ **No "stdout is not a terminal" error**
2. ✅ **Codex started successfully**
3. ✅ **Read the prompt correctly**
4. ❌ **401 Unauthorized** - API key issue (not TTY issue)

## Why It Works Now vs Previous Experiments

### Previous failures:
```bash
# From OUTSIDE container
docker run ... codex exec "prompt"
# ERROR: stdout is not a terminal
```

### Current success:
```bash
# Shell script INSIDE container
docker run ... ./script.sh
# Script calls: codex exec "prompt"
# SUCCESS: No TTY error
```

**Hypothesis:** When Codex is executed from within the container's shell environment (via script), it detects a valid shell context and doesn't require explicit TTY allocation.

## Implications for Test Architecture

### Current Architecture (Complex):
```
GitHub Actions
  → Docker run
    → cargo test
      → Rust test code
        → Spawns nested Docker (Docker-in-Docker)
          → Runs Codex
            → Uses fallback extraction due to TTY issues
```

### Potential Simplified Architecture:
```
GitHub Actions
  → Docker run
    → Shell script
      → Runs Codex directly
        → MCP server interaction
          → Captures output directly
```

## Recommendations

1. **API Key Issue:** The 401 error suggests the API key may be invalid/expired. Need to verify credentials before further testing.

2. **Simplified Test Approach:** Consider rewriting integration tests as shell scripts instead of Rust tests:
   - Simpler to understand
   - Easier to debug
   - No Docker-in-Docker complexity
   - Direct output capture

3. **Keep Rust Tests for Unit Testing:** Reserve Rust `cargo test` for:
   - MCP server unit tests
   - DAP protocol tests
   - Core functionality tests

4. **Use Shell Scripts for Integration Tests:** For AI client integration:
   - Simpler orchestration
   - Direct interaction with Codex/Claude Code
   - Standard shell tooling (grep, jq, etc.)

## Login Requirement Experiment

### Question: Is `codex login --with-api-key` necessary or is OPENAI_API_KEY env var sufficient?

**Test 1: Without login (OPENAI_API_KEY only)**
```
Result: 401 Unauthorized
ERROR: exceeded retry limit, last status: 401 Unauthorized
```

**Test 2: With login (`echo $KEY | codex login --with-api-key`)**
```
Result: ✅ SUCCESS
Successfully logged in
codex
There was no FIFA World Cup (football world championship) held in 2024...
tokens used: 404
```

### Conclusion: ✅ Login step IS REQUIRED

- OPENAI_API_KEY environment variable alone is **NOT sufficient**
- Must run `codex login --with-api-key` before `codex exec`
- The login step authenticates and stores credentials
- After login, Codex works perfectly in Docker!

### Authentication Flow

```bash
# Required workflow:
echo "$OPENAI_API_KEY" | codex login --with-api-key
codex exec --dangerously-bypass-approvals-and-sandbox "prompt"
# ✅ Works!

# Insufficient workflow:
export OPENAI_API_KEY="sk-..."
codex exec --dangerously-bypass-approvals-and-sandbox "prompt"
# ❌ 401 Unauthorized
```

## Next Steps

1. ✅ Confirmed Codex can run directly in Docker (when called from inside)
2. ✅ Verified API key validity (works after login)
3. ✅ Confirmed full execution works (Codex answered question successfully)
4. ✅ Confirmed login step is required
5. ⏭️ Prototype shell-based integration test with login step
6. ⏭️ Compare complexity vs current Rust test approach
7. ⏭️ Decision: Keep Rust wrapper or migrate to shell scripts

## Files Created

- `scripts/test-codex-direct.sh` - Direct Codex execution test
- `docs/CODEX_DOCKER_EXPERIMENT_RESULTS.md` - This document
