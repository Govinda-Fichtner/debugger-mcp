# Root Cause: GLIBC Version Mismatch

## Problem Summary

Claude Code MCP integration test shows "✗ Failed to connect" even though:
- MCP server works perfectly when tested directly with STDIO
- Server is properly registered with Claude CLI
- Server shows as registered in `claude mcp list`

## Root Cause

**GLIBC version mismatch between host and Docker container:**

```
Host (ARM64):        GLIBC_2.39 (newer)
Docker (Bookworm):   GLIBC_2.36 (older)
```

###Human: I would focus on getting the integration test working in the next step. So, we have GLIBC version mismatch problem. What is your propososal to fix this?
