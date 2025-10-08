# Container Path Mapping - Critical Guide for Debugger MCP

**🔴 CRITICAL**: When the debugger MCP server runs in a Docker container, you MUST use **container paths**, not host paths!

## The Problem

The #1 issue when debugging in containers: **Path confusion**

```
❌ WRONG - This will fail:
debugger_start({
  "program": "/home/vagrant/projects/fizzbuzz-nodejs-test/fizzbuzz.js"
})

Error: Session terminates immediately, "File not found"

✅ CORRECT - This works:
debugger_start({
  "program": "/workspace/fizzbuzz-nodejs-test/fizzbuzz.js"
})

Success: Debugging session starts normally
```

## Why This Happens

### The Environment

```
┌─────────────────────────────────────────┐
│ Host Machine                            │
│                                         │
│ /home/vagrant/projects/                 │
│   ├── fizzbuzz-nodejs-test/             │
│   │   └── fizzbuzz.js  ← You see this  │
│   └── debugger_mcp/                     │
│                                         │
└──────────────┬──────────────────────────┘
               │ Docker Volume Mount
               │ -v /home/vagrant/projects:/workspace
               ▼
┌─────────────────────────────────────────┐
│ Container (debugger MCP server)         │
│                                         │
│ /workspace/                              │
│   ├── fizzbuzz-nodejs-test/             │
│   │   └── fizzbuzz.js  ← MCP sees this │
│   └── debugger_mcp/                     │
│                                         │
└─────────────────────────────────────────┘
```

**Key Point**: The MCP server runs INSIDE the container, so it only sees `/workspace/`, not `/home/vagrant/projects/`.

## How to Find the Correct Path

### Method 1: Check Container Mounts

```bash
# Find your container ID
docker ps | grep debugger

# Inspect the mounts
docker inspect <container-id> | grep -A 10 "Mounts"

# Output shows:
# "Source": "/home/vagrant/projects",
# "Destination": "/workspace"
```

### Method 2: Test Path Accessibility

```bash
# Verify the file exists in the container
docker exec <container-id> ls -la /workspace/fizzbuzz-nodejs-test/

# Should show:
# -rw-rw-r-- 1 mcpuser mcpuser 455 Oct 7 fizzbuzz.js
```

### Method 3: Common Patterns

**Standard debugger-nodejs container**:
```
Host Path:      /home/vagrant/projects/fizzbuzz-nodejs-test/fizzbuzz.js
Container Path: /workspace/fizzbuzz-nodejs-test/fizzbuzz.js
```

**Standard debugger-ruby container**:
```
Host Path:      /home/vagrant/projects/fizzbuzz-ruby-test/fizzbuzz.rb
Container Path: /workspace/fizzbuzz.rb
```

**Standard debugger-python container**:
```
Host Path:      /home/vagrant/projects/fizzbuzz-python-test/fizzbuzz.py
Container Path: /workspace/fizzbuzz.py
```

**Standard debugger-rust container**:
```
Host Path:      /home/vagrant/projects/fizzbuzz-rust-test/fizzbuzz.rs
Container Path: /workspace/fizzbuzz-rust-test/fizzbuzz.rs
```

## Translation Rules

### Rule 1: Replace Host Base with Container Base

```python
# Generic formula
container_path = host_path.replace("/home/vagrant/projects", "/workspace")

# Example
host_path = "/home/vagrant/projects/my-app/src/main.js"
container_path = "/workspace/my-app/src/main.js"
```

### Rule 2: Always Use Absolute Paths

```bash
✅ CORRECT: "/workspace/app/main.py"
❌ WRONG:   "app/main.py"
❌ WRONG:   "./app/main.py"
❌ WRONG:   "~/projects/app/main.py"
```

### Rule 3: Preserve Directory Structure

```
Host:      /home/vagrant/projects/myapp/src/utils/helper.js
Container: /workspace/myapp/src/utils/helper.js
           ↑         ↑
           |         └─ Same structure maintained
           └─ Base changed
```

## Language-Specific Examples

### Python

```json
{
  "language": "python",
  "program": "/workspace/my-python-app/main.py",
  "stopOnEntry": true
}
```

### Ruby

```json
{
  "language": "ruby",
  "program": "/workspace/fizzbuzz-ruby-test/fizzbuzz.rb",
  "stopOnEntry": true
}
```

### Node.js

```json
{
  "language": "nodejs",
  "program": "/workspace/fizzbuzz-nodejs-test/fizzbuzz.js",
  "stopOnEntry": true
}
```

### Rust

**Important**: For Rust, provide the **source file** path (`.rs`), not the binary path. The MCP server will compile it automatically.

```json
{
  "language": "rust",
  "program": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
  "stopOnEntry": true
}
```

**After compilation**, the binary will be at:
```
/workspace/fizzbuzz-rust-test/target/debug/fizzbuzz
```

But you don't need to specify this - the server handles it automatically.

## Breakpoint Paths

**IMPORTANT**: Breakpoint source paths must ALSO use container paths!

```json
// ❌ WRONG - Host path
debugger_set_breakpoint({
  "sessionId": "...",
  "sourcePath": "/home/vagrant/projects/fizzbuzz-nodejs-test/fizzbuzz.js",
  "line": 9
})

// ✅ CORRECT - Container path
debugger_set_breakpoint({
  "sessionId": "...",
  "sourcePath": "/workspace/fizzbuzz-nodejs-test/fizzbuzz.js",
  "line": 9
})
```

### Rust Breakpoint Paths

For Rust, use the **source file** path (`.rs`), not the compiled binary path:

```json
// ✅ CORRECT - Source file path
debugger_set_breakpoint({
  "sessionId": "...",
  "sourcePath": "/workspace/fizzbuzz-rust-test/fizzbuzz.rs",
  "line": 9
})

// ❌ WRONG - Binary path
debugger_set_breakpoint({
  "sessionId": "...",
  "sourcePath": "/workspace/fizzbuzz-rust-test/target/debug/fizzbuzz",
  "line": 9
})
```

## Error Symptoms

### Symptom 1: Immediate Termination

```
Session state goes directly to "Terminated" (< 1 second)
```

**Cause**: Program file not found in container
**Fix**: Check path translation

### Symptom 2: Cannot Set Breakpoints

```
Breakpoint set: {"verified": false, "message": "Source not found"}
```

**Cause**: Source path doesn't match container path
**Fix**: Use container path in `sourcePath`

### Symptom 3: Stack Trace Shows Wrong Files

```
Stack trace shows paths you don't recognize
```

**Cause**: Debugger found DIFFERENT file in container
**Fix**: Verify exact path in container

## Troubleshooting Workflow

### Step 1: Verify Container is Running

```bash
docker ps | grep debugger-nodejs

# Should show:
# CONTAINER ID   IMAGE                    STATUS
# abc123def456   mcp-debugger-nodejs:latest   Up 5 minutes
```

### Step 2: Check Volume Mount

```bash
docker inspect <container-id> | grep -A 5 '"Mounts"'

# Verify "Source" and "Destination" match your expectation
```

### Step 3: Test File Accessibility

```bash
# Try to list your file
docker exec <container-id> ls -la /workspace/yourapp/yourfile.js

# If "No such file or directory":
# 1. Check volume mount is correct
# 2. Restart container with correct mount
# 3. Verify file exists on host
```

### Step 4: Test Debugging with Absolute Path

```json
// Try with explicit container path
{
  "language": "nodejs",
  "program": "/workspace/fizzbuzz-nodejs-test/fizzbuzz.js"
}

// If this works, your previous path was wrong
```

## Common Mistakes

### Mistake 1: Using Host Path

```json
❌ {"program": "/home/vagrant/projects/app.js"}
✅ {"program": "/workspace/app.js"}
```

### Mistake 2: Missing Directory Structure

```json
// Host:      /home/vagrant/projects/myapp/src/index.js
❌ {"program": "/workspace/index.js"}
✅ {"program": "/workspace/myapp/src/index.js"}
```

### Mistake 3: Relative Paths

```json
❌ {"program": "./app.js"}
❌ {"program": "app.js"}
✅ {"program": "/workspace/app.js"}
```

### Mistake 4: Inconsistent Paths

```json
// Start with container path
debugger_start({"program": "/workspace/app.js"})

// But use host path for breakpoint (WRONG!)
❌ debugger_set_breakpoint({
  "sourcePath": "/home/vagrant/projects/app.js"
})

// Use container path for BOTH
✅ debugger_set_breakpoint({
  "sourcePath": "/workspace/app.js"
})
```

## Pre-Flight Checklist

Before starting a debugging session in a container:

- [ ] Know the container ID: `docker ps | grep debugger`
- [ ] Know the volume mount: `docker inspect <id> | grep Mounts`
- [ ] Translate host path → container path
- [ ] Verify file exists: `docker exec <id> ls -la <container-path>`
- [ ] Use container path in ALL debugger calls
- [ ] Use absolute paths (start with `/`)

## Quick Reference Card

```
┌────────────────────────────────────────────────────────────┐
│ Container Path Quick Reference                             │
├────────────────────────────────────────────────────────────┤
│                                                            │
│ 1. Find container mount:                                  │
│    docker inspect <id> | grep -A 5 Mounts                 │
│                                                            │
│ 2. Translate path:                                        │
│    /home/vagrant/projects/* → /workspace/*                │
│                                                            │
│ 3. Verify file:                                           │
│    docker exec <id> ls -la /workspace/...                 │
│                                                            │
│ 4. Use container path EVERYWHERE:                         │
│    - debugger_start: program                              │
│    - debugger_set_breakpoint: sourcePath                  │
│    - Always absolute (starts with /)                      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Advanced: Custom Volume Mounts

If you run the container with a custom mount:

```bash
# Custom mount example
docker run -v /my/custom/path:/data mcp-debugger-nodejs:latest

# Then use /data in debugger calls:
{
  "program": "/data/myapp/index.js"
}
```

**Rule**: Whatever follows `-v` after the `:` is your container base path.

```bash
-v /host/path:/container/path
              ^^^^^^^^^^^^^^^^
              Use this in debugger calls
```

## Summary

**The Golden Rule**:
> Always use paths as the MCP server (running in the container) sees them, not as you see them on your host.

**Quick Test**:
> Can you `docker exec <container-id> cat /your/path/to/file.js`?
> If yes → path is correct for debugging
> If no → path is wrong, session will fail

**Common Pattern**:
> Replace `/home/vagrant/projects` with `/workspace` and you're 90% there!

---

**Related Documentation**:
- See `DOCKER_SETUP.md` for container deployment
- See `TROUBLESHOOTING.md` for common debugging issues
- See `GETTING_STARTED.md` for basic usage (includes container notes)
