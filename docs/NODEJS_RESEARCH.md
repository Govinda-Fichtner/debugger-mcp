# Node.js Debugging Research

**Date**: October 7, 2025
**Purpose**: Research Node.js debugging protocols and adapters for DAP MCP Server integration
**Branch**: `feature/nodejs-support`

---

## Executive Summary

Node.js debugging requires a **Debug Adapter Protocol (DAP) adapter** because Node.js's built-in inspector uses the **Chrome DevTools Protocol (CDP)**, not DAP directly.

**Recommended Solution**: Use Microsoft's **vscode-js-debug** as the DAP adapter.

---

## Protocol Landscape

### Node.js Built-in Inspector

**Command**: `node --inspect` or `node --inspect-brk`

- **Protocol**: Chrome DevTools Protocol (CDP)
- **Transport**: WebSocket (ws://127.0.0.1:9229/...)
- **Version**: Available since Node.js 6.3+ (stable since 8.0)
- **Replaced**: V8 Debugging Protocol (legacy, obsolete since Node 7.7)

#### Key Flags

| Flag | Behavior | Use Case |
|------|----------|----------|
| `--inspect` | Start execution immediately | Attach debugger while running |
| `--inspect-brk` | Wait at first line | stopOnEntry behavior (like Python's debugpy) |
| `--inspect-brk=<port>` | Wait at first line on specific port | Custom port configuration |

**Example**:
```bash
node --inspect-brk=9229 script.js
# Listens on ws://127.0.0.1:9229/...
# Waits for debugger to attach before execution
```

### Debug Adapter Protocol (DAP)

- **Our Implementation**: DAP client in `src/dap/client.rs`
- **Used by**: Python (debugpy), Ruby (rdbg socket mode), and our MCP server
- **Not used by**: Node.js inspector (uses CDP)

### The Gap: CDP ≠ DAP

**Problem**: Node.js speaks CDP, our MCP server speaks DAP.

**Solution**: Use a CDP-to-DAP adapter (translation layer).

---

## Available DAP Adapters for Node.js

### 1. vscode-js-debug (RECOMMENDED) ✅

**Repository**: https://github.com/microsoft/vscode-js-debug

#### Key Details

- **Maintainer**: Microsoft
- **Protocol**: Debug Adapter Protocol (DAP) ✅
- **Status**: Actively maintained, default debugger in VS Code
- **Supports**:
  - Node.js (all versions)
  - Chrome/Edge browsers
  - Child processes and worker threads
  - WebAssembly
  - Source maps (TypeScript support)
  - Performance profiling

#### Running as Standalone DAP Server

```bash
# Command
node /path/to/vscode-js-debug/out/src/dapDebugServer.js <port>

# Example
node ~/.vscode-js-debug/out/src/dapDebugServer.js 8123
```

#### Installation

**Option A: Download Release**
```bash
# Download from GitHub releases
wget https://github.com/microsoft/vscode-js-debug/releases/download/v1.XX.X/js-debug-dap-v1.XX.X.tar.gz
tar -xzf js-debug-dap-v1.XX.X.tar.gz -C ~/.vscode-js-debug
```

**Option B: Build from Source**
```bash
git clone https://github.com/microsoft/vscode-js-debug.git
cd vscode-js-debug
npm install --legacy-peer-deps
npx gulp vsDebugServerBundle
mv dist out
```

**Result**: `out/src/dapDebugServer.js` ready to use

#### Launch Configuration

From Emacs dap-mode (`dap-node.el`):
```elisp
(list :type "node"
      :cwd nil
      :request "launch"
      :program nil
      :name "Node::Run")
```

From nvim-dap:
```lua
require("dap").adapters["pwa-node"] = {
  type = "server",
  host = "localhost",
  port = "${port}",
  executable = {
    command = "node",
    args = {"/path/to/js-debug/out/src/dapDebugServer.js", "${port}"}
  }
}
```

#### Advantages

✅ **DAP Native** - No protocol translation needed
✅ **Actively Maintained** - Default in VS Code
✅ **Feature Rich** - Source maps, profiling, WebAssembly
✅ **Well Tested** - Used by millions of developers
✅ **Documented** - VS Code docs + community resources

#### Disadvantages

⚠️ **Size** - Larger than minimal adapters (~50MB)
⚠️ **Dependencies** - Requires Node.js to run adapter itself

---

### 2. node-debug2 (Legacy) ⚠️

**Repository**: https://github.com/microsoft/vscode-node-debug2

- **Status**: Deprecated, replaced by vscode-js-debug
- **Protocol**: DAP
- **Use Case**: Legacy projects only

**Recommendation**: Use vscode-js-debug instead.

---

### 3. Direct CDP Implementation ❌

**Approach**: Implement CDP-to-DAP translation ourselves.

**Complexity**:
- CDP has 50+ domains (Console, Debugger, Runtime, Profiler, etc.)
- Different message formats
- WebSocket transport
- State management differences

**Recommendation**: Not worth the effort when vscode-js-debug exists.

---

## Comparison with Python and Ruby

| Aspect | Python (debugpy) | Ruby (rdbg) | Node.js (vscode-js-debug) |
|--------|------------------|-------------|---------------------------|
| **Protocol** | DAP native | DAP native | DAP (translates CDP) |
| **Transport** | TCP socket | TCP socket | TCP (DAP server mode) |
| **stopOnEntry** | ✅ Native | ❌ Workaround needed | ✅ Native (--inspect-brk) |
| **Adapter Command** | `python -m debugpy` | `rdbg --open --port` | `node dapDebugServer.js` |
| **Port** | Dynamic | Dynamic | Dynamic |
| **Spawn Method** | Direct process | Direct process | Adapter server + Node.js |
| **Complexity** | Low | Medium (entry breakpoint) | Medium (adapter setup) |

---

## Implementation Strategy

### Option A: Use vscode-js-debug (RECOMMENDED)

**Approach**: Two-step process

1. **Spawn DAP Server**:
   ```bash
   node /path/to/vscode-js-debug/out/src/dapDebugServer.js <port>
   ```

2. **Connect via TCP**:
   - Our DAP client connects to `localhost:<port>`
   - Send DAP messages (same as Python/Ruby)

3. **Launch Configuration**:
   ```json
   {
     "type": "pwa-node",
     "request": "launch",
     "program": "/path/to/script.js",
     "stopOnEntry": true,
     "cwd": "/working/dir"
   }
   ```

**Advantages**:
- ✅ Reuses existing DAP client (`src/dap/client.rs`)
- ✅ TCP transport (already implemented)
- ✅ Native stopOnEntry support (no workaround)
- ✅ Well-tested adapter

**Implementation Complexity**: **Medium**
- Need to bundle/install vscode-js-debug
- Two-process management (adapter + Node.js)
- But protocol is familiar (DAP)

---

### Option B: Direct Node.js Inspector ❌

**Approach**: Connect to Node.js inspector directly

1. **Spawn Node.js**:
   ```bash
   node --inspect-brk=9229 script.js
   ```

2. **Connect via WebSocket**:
   - Parse WebSocket handshake
   - Implement CDP protocol
   - Translate to DAP internally

**Disadvantages**:
- ❌ Need WebSocket client (new dependency)
- ❌ Need CDP implementation (complex)
- ❌ Need CDP-to-DAP translation (complex)
- ❌ Reinventing the wheel

**Recommendation**: Don't do this when vscode-js-debug exists.

---

## Chosen Approach: vscode-js-debug

### Architecture

```
MCP Client (Claude)
    ↓ MCP Protocol
┌─────────────────────────────────────┐
│  DAP MCP Server (Rust)              │
│  ├─ src/adapters/nodejs.rs          │
│  └─ src/dap/client.rs               │
└────────────┬────────────────────────┘
             ↓ DAP over TCP
┌────────────┴────────────────────────┐
│  vscode-js-debug (DAP Server)       │
│  (node dapDebugServer.js <port>)    │
└────────────┬────────────────────────┘
             ↓ CDP over WebSocket
┌────────────┴────────────────────────┐
│  Node.js Inspector                  │
│  (node --inspect-brk script.js)     │
└─────────────────────────────────────┘
```

### Implementation Steps

#### Step 1: Adapter Configuration

**File**: `src/adapters/nodejs.rs`

```rust
pub struct NodeJsAdapter;

impl NodeJsAdapter {
    pub fn adapter_command() -> String {
        "node".to_string()
    }

    pub fn adapter_args(port: u16) -> Vec<String> {
        vec![
            "/path/to/vscode-js-debug/out/src/dapDebugServer.js".to_string(),
            port.to_string(),
        ]
    }

    pub fn launch_config(program: &str, args: &[String], stop_on_entry: bool) -> Value {
        json!({
            "type": "pwa-node",
            "request": "launch",
            "program": program,
            "args": args,
            "stopOnEntry": stop_on_entry,
        })
    }
}
```

#### Step 2: Spawn Pattern

```rust
// 1. Find free port for DAP server
let dap_port = socket_helper::find_free_port()?;

// 2. Spawn vscode-js-debug DAP server
let adapter_process = Command::new("node")
    .args(&[
        "/path/to/vscode-js-debug/out/src/dapDebugServer.js",
        &dap_port.to_string(),
    ])
    .spawn()?;

// 3. Connect to DAP server via TCP
let socket = socket_helper::connect_with_retry(dap_port, Duration::from_secs(2)).await?;

// 4. Use existing DAP client (same as Ruby)
// - initialize
// - launch (with Node.js program path)
// - configurationDone
// - debugging session continues...
```

#### Step 3: stopOnEntry Handling

**Expected Behavior**: Should work natively (like Python)!

```rust
// Launch config with stopOnEntry: true
let launch_args = json!({
    "type": "pwa-node",
    "request": "launch",
    "program": "/path/to/script.js",
    "stopOnEntry": true,  // Should work!
});

// Expected:
// 1. vscode-js-debug receives launch request
// 2. Spawns node --inspect-brk internally
// 3. Sends 'stopped' event at entry point
// 4. We can set breakpoints, continue, etc.
```

**Hypothesis**: No entry breakpoint workaround needed (unlike Ruby).

**Test**: Must verify this assumption!

---

## Key Differences from Ruby

### Ruby Challenges (for context)

1. ❌ **No stopOnEntry**: Needed entry breakpoint workaround
2. ❌ **Pause broken**: rdbg accepts pause but doesn't pause
3. ⚠️ **DAP sequence**: Had to set breakpoints before configurationDone

### Node.js Expectations

1. ✅ **stopOnEntry**: Should work natively via vscode-js-debug
2. ✅ **Pause**: Should work (vscode-js-debug is well-tested)
3. ✅ **DAP sequence**: vscode-js-debug handles correctly

**Risk**: These are assumptions that need testing!

---

## Testing Strategy

### Test 1: Adapter Installation
```bash
# Verify vscode-js-debug is available
test -f ~/.vscode-js-debug/out/src/dapDebugServer.js
echo $?  # Should be 0
```

### Test 2: DAP Server Startup
```bash
# Start DAP server manually
node ~/.vscode-js-debug/out/src/dapDebugServer.js 8123

# Verify it's listening
nc -zv localhost 8123  # Should connect
```

### Test 3: Protocol Verification (Manual)
```bash
# Terminal 1: Start DAP server
node ~/.vscode-js-debug/out/src/dapDebugServer.js 8123

# Terminal 2: Connect and send initialize
nc localhost 8123
Content-Length: 125

{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"test","adapterID":"pwa-node"}}

# Expected: DAP response with capabilities
```

### Test 4: Node.js Execution Test
```javascript
// test.js
console.log("Start");
debugger;  // Breakpoint
console.log("End");
```

```bash
# Launch via vscode-js-debug
# Should stop at debugger statement
```

### Test 5: stopOnEntry Validation
```rust
#[tokio::test]
async fn test_nodejs_stop_on_entry() {
    // Launch with stopOnEntry: true
    // Verify 'stopped' event received at entry
    // No entry breakpoint workaround should be needed
}
```

---

## Installation Requirements

### For Development

```bash
# Install vscode-js-debug
mkdir -p ~/.vscode-js-debug
cd ~/.vscode-js-debug
wget https://github.com/microsoft/vscode-js-debug/releases/latest/download/js-debug-dap-v1.XX.X.tar.gz
tar -xzf js-debug-dap-v1.XX.X.tar.gz
```

### For Docker

**Dockerfile.nodejs**:
```dockerfile
FROM rust:1.70-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM node:20-alpine
RUN apk add --no-cache bash

# Install vscode-js-debug
RUN mkdir -p /usr/local/lib/vscode-js-debug
COPY --from=vscode-js-debug-source /out /usr/local/lib/vscode-js-debug/

COPY --from=builder /build/target/release/debugger_mcp /usr/local/bin/

ENTRYPOINT ["debugger_mcp", "serve"]
```

---

## Open Questions & Assumptions

### Assumptions to Test

1. ✅ **vscode-js-debug uses DAP** - Confirmed from docs
2. ⏳ **stopOnEntry works natively** - Needs testing
3. ⏳ **Port configuration works** - Needs testing
4. ⏳ **Two-process spawn works** - Needs implementation
5. ⏳ **TCP connection stable** - Needs testing
6. ⏳ **DAP sequence same as Python** - Needs validation

### Questions

1. **Adapter Path**: Where to install vscode-js-debug in production?
   - Option A: Bundle in Docker image
   - Option B: Require user installation
   - Option C: Download on first use

2. **Port Management**: How to coordinate ports?
   - DAP server port (our choice)
   - Node.js inspector port (vscode-js-debug chooses?)

3. **Process Lifecycle**: Who manages Node.js process?
   - vscode-js-debug spawns Node.js internally
   - We only manage vscode-js-debug process

4. **Error Handling**: What if vscode-js-debug fails?
   - Fallback strategy?
   - Clear error messages

---

## Expected Implementation Complexity

### Easy (Same as Python) ✅

- DAP client reuse
- TCP transport (already implemented)
- Launch configuration (JSON structure)
- Breakpoint setting
- Expression evaluation

### Medium (New but straightforward) ⚠️

- Two-process management (DAP server + Node.js)
- vscode-js-debug installation/bundling
- Port coordination
- Path configuration

### Hard (Unknowns) ❓

- None expected (if stopOnEntry works natively)
- Fallback: Entry breakpoint pattern (already implemented for Ruby)

---

## Success Criteria

### Must Have ✅

- [ ] vscode-js-debug spawns successfully
- [ ] DAP connection established
- [ ] stopOnEntry works (stopped event at entry)
- [ ] Breakpoints work (set, verified, hit)
- [ ] Expression evaluation works
- [ ] Stack traces work
- [ ] Clean disconnect

### Nice to Have ⭐

- [ ] Source map support (TypeScript)
- [ ] Child process debugging
- [ ] Performance profiling
- [ ] WebAssembly debugging

---

## Timeline Estimate

| Task | Duration | Deliverable |
|------|----------|-------------|
| Research (this doc) | 2-3 hours | ✅ Complete |
| Test fixture | 30 min | `tests/fixtures/fizzbuzz.js` |
| Failing tests | 1-2 hours | `tests/test_nodejs.rs` |
| Adapter config | 1 hour | `src/adapters/nodejs.rs` |
| vscode-js-debug integration | 2-3 hours | Spawn + connect |
| Make tests pass | 2-4 hours | TDD cycles |
| End-to-end validation | 1-2 hours | Claude testing |
| Docker image | 1-2 hours | `Dockerfile.nodejs` |
| Documentation | 1-2 hours | Implementation guide |
| **Total** | **12-17 hours** | **Node.js support complete** |

---

## References

### Documentation

- **vscode-js-debug**: https://github.com/microsoft/vscode-js-debug
- **Node.js Inspector**: https://nodejs.org/en/learn/getting-started/debugging
- **Chrome DevTools Protocol**: https://chromedevtools.github.io/devtools-protocol/
- **DAP Specification**: https://microsoft.github.io/debug-adapter-protocol/

### Examples

- **nvim-dap Node.js config**: https://github.com/mxsdev/nvim-dap-vscode-js
- **Emacs dap-mode**: https://github.com/emacs-lsp/dap-mode/blob/master/dap-node.el
- **VS Code Node.js debugging**: https://code.visualstudio.com/docs/nodejs/nodejs-debugging

### Community Resources

- **Neovim Node.js debugging**: https://www.darricheng.com/posts/setting-up-nodejs-debugging-in-neovim/
- **JavaScript debugging frameworks**: https://theosteiner.de/debugging-javascript-frameworks-in-neovim
- **Stack Overflow discussions**: Multiple recent (2023-2024) Q&A threads

---

## Decision: Use vscode-js-debug

**Rationale**:
1. ✅ DAP native - Works with our existing client
2. ✅ Well maintained - Microsoft, millions of users
3. ✅ Feature complete - All debugging features
4. ✅ Proven - Default in VS Code
5. ✅ Documented - Good community support

**Next Steps**:
1. Install vscode-js-debug
2. Create test fixture (fizzbuzz.js)
3. Write failing integration tests
4. Implement adapter configuration
5. TDD implementation cycles
6. Validation and documentation

---

**Status**: ✅ Research Complete
**Recommendation**: Proceed with vscode-js-debug implementation
**Confidence**: High (90%)
