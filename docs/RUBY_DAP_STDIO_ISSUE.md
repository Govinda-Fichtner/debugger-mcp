# Ruby DAP Stdio Issue - Root Cause Analysis

## Date: 2025-10-07

## Executive Summary

**CRITICAL FINDING**: rdbg does NOT support DAP protocol via stdio. It only supports DAP via socket (`--open` flag).

Our current implementation tries to use stdio (like debugpy), but this is fundamentally incompatible with rdbg's architecture.

## The Problem

### What We're Doing (WRONG)
```rust
// Current implementation:
spawn("rdbg", ["--stop-at-load", "program.rb"])
// Then send DAP messages via stdin/stdout
```

**Result**: rdbg runs in interactive console mode and tries to evaluate DAP JSON as Ruby code.

### Why It Doesn't Work

1. **rdbg defaults to interactive console mode** when stdin is connected
2. **DAP mode requires `--open` flag** which creates a TCP/UNIX socket
3. **No stdio DAP mode exists** in rdbg (unlike debugpy)

## Evidence

### Test 1: Sending DAP initialize via stdin
```bash
echo 'Content-Length: 314\n\n{...DAP JSON...}' | rdbg --stop-at-load program.rb
```

**Result**:
```
(rdbg) Content-Length: 314
eval error: syntax error, unexpected ':', expecting end-of-input
```

**Analysis**: rdbg treats the input as interactive console commands, not DAP protocol.

### Test 2: Checking rdbg source code
```ruby
# From /usr/lib/ruby/gems/3.3.0/gems/debug-1.11.0/lib/debug/server.rb
when /^Content-Length: (\d+)/
  require_relative 'server_dap'
  # ...switches to DAP mode
```

**Key Discovery**: This code is in the **socket server**, not the default stdin handler!

The DAP detection only happens when rdbg is in `--open` (socket server) mode, reading from a socket connection.

## Architecture Comparison

### debugpy (Python) - Adapter Server Pattern
```
┌────────────────────────┐
│ python -m              │  ← Adapter server (no program)
│ debugpy.adapter        │  ← Listens on stdin/stdout for DAP
└───────────┬────────────┘
            │ DAP: launch(program="app.py")
            ▼
┌────────────────────────┐
│ Spawns & debugs app.py │
└────────────────────────┘
```

**Key**: debugpy.adapter is a separate adapter server that accepts DAP via stdio.

### rdbg (Ruby) - Direct Debugger Pattern
```
┌────────────────────────┐
│ rdbg program.rb        │  ← Runs program directly
│ [Interactive Console]  │  ← stdin = console commands
└────────────────────────┘

OR

┌────────────────────────┐
│ rdbg --open program.rb │  ← Runs program + socket server
│ [Listens on socket]    │  ← DAP via socket connection
└────────────────────────┘
```

**Key**: No built-in stdio DAP adapter mode exists.

## Solutions

### Solution 1: Switch to Socket Mode (RECOMMENDED)

**Implementation**:
```rust
// 1. Find free port
let port = find_free_port();

// 2. Spawn rdbg with socket
spawn("rdbg", ["--open", "--port", &port.to_string(), program])

// 3. Wait for socket to be ready
wait_for_socket_ready(port, timeout=2s);

// 4. Connect to socket
let socket = TcpStream::connect(("127.0.0.1", port)).await?;

// 5. Use socket for DAP communication
let transport = DapTransport::new_from_socket(socket);
```

**Pros**:
- ✅ Native rdbg support
- ✅ Well-tested (used by VS Code, nvim-dap)
- ✅ No protocol translation needed

**Cons**:
- ❌ More complex (port allocation, socket management)
- ❌ Different from Python implementation
- ❌ Requires refactoring DAP client

### Solution 2: Create Stdio-to-Socket Bridge

**Implementation**:
```rust
// 1. Spawn rdbg with socket
let port = find_free_port();
spawn("rdbg", ["--open", "--port", &port.to_string(), program]);

// 2. Create bridge process
let bridge = spawn_bridge_task(port, stdin, stdout);

// Bridge forwards:
// - stdin → socket (DAP requests)
// - socket → stdout (DAP responses)
```

**Pros**:
- ✅ Keeps stdio interface for MCP
- ✅ Less refactoring of DAP client

**Cons**:
- ❌ Additional complexity
- ❌ Another point of failure
- ❌ Performance overhead

### Solution 3: Use VSCode rdbg Extension Approach

VS Code's vscode-rdbg extension uses:
```
rdbg --open=vscode --port=0 (auto-allocate port)
```

Then reads the port from a JSON file created by rdbg.

**Implementation**:
```rust
// 1. Spawn with auto port
spawn("rdbg", ["--open=vscode", "--port=0", program]);

// 2. Read port from .vscode/rdbg_autoattach.json
let port = read_port_from_file(".vscode/rdbg_autoattach.json")?;

// 3. Connect to port
connect_to_socket(port);
```

**Pros**:
- ✅ Well-established pattern
- ✅ Handles port allocation automatically

**Cons**:
- ❌ Requires file system access
- ❌ Specific to VS Code integration

## Recommended Approach

**Use Solution 1: Socket Mode**

### Implementation Plan

1. **Create RubySocketAdapter** (new file)
   ```rust
   pub struct RubySocketAdapter {
       port: u16,
       process: Option<Child>,
   }

   impl RubySocketAdapter {
       pub async fn spawn(program: &str, args: &[String], stop_on_entry: bool) -> Result<(Self, TcpStream)> {
           // 1. Find free port
           let port = find_free_port()?;

           // 2. Build args
           let mut rdbg_args = vec![
               "--open".to_string(),
               "--port".to_string(),
               port.to_string(),
           ];

           if stop_on_entry {
               rdbg_args.push("--stop-at-load".to_string());
           } else {
               rdbg_args.push("--nonstop".to_string());
           }

           rdbg_args.push(program.to_string());
           rdbg_args.extend(args.iter().cloned());

           // 3. Spawn rdbg
           let child = Command::new("rdbg")
               .args(&rdbg_args)
               .spawn()?;

           // 4. Wait for socket (with timeout)
           let socket = timeout(Duration::from_secs(3), async {
               loop {
                   match TcpStream::connect(("127.0.0.1", port)).await {
                       Ok(s) => return Ok(s),
                       Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                   }
               }
           }).await??;

           Ok((Self { port, process: Some(child) }, socket))
       }
   }
   ```

2. **Update DapTransport** to support sockets
   ```rust
   pub enum DapTransport {
       Stdio { stdin: ChildStdin, stdout: BufReader<ChildStdout> },
       Socket { stream: TcpStream },
   }
   ```

3. **Update manager.rs** Ruby case
   ```rust
   "ruby" => {
       let (adapter, socket) = RubySocketAdapter::spawn(&program, &args, stop_on_entry).await?;
       let transport = DapTransport::Socket { stream: socket };
       // ... rest of initialization
   }
   ```

## Timeline to Fix

**Estimated**: 3-4 hours

1. **Hour 1**: Implement RubySocketAdapter
   - Port finding logic
   - Socket connection with retry/timeout
   - Process management

2. **Hour 2**: Update DapTransport
   - Add Socket variant
   - Implement read/write for socket
   - Update tests

3. **Hour 3**: Integration
   - Update manager.rs
   - Update session initialization
   - Handle socket lifecycle

4. **Hour 4**: Testing
   - Unit tests for socket adapter
   - Integration tests
   - End-to-end testing with Claude Code

## Testing Plan

### Unit Tests
```rust
#[tokio::test]
async fn test_ruby_socket_adapter_spawn() {
    let (adapter, socket) = RubySocketAdapter::spawn(
        "/workspace/fizzbuzz.rb",
        &[],
        true
    ).await.unwrap();

    assert!(socket.peer_addr().is_ok());
}

#[tokio::test]
async fn test_ruby_socket_dap_initialize() {
    let (adapter, socket) = RubySocketAdapter::spawn(...).await.unwrap();
    let mut transport = DapTransport::Socket { stream: socket };

    let init_request = Request::Initialize {...};
    transport.write_message(&Message::Request(init_request)).await.unwrap();

    let response = transport.read_message().await.unwrap();
    assert!(matches!(response, Message::Response(_)));
}
```

### Integration Test
```rust
#[tokio::test]
#[ignore] // Requires rdbg
async fn test_ruby_debugging_with_socket() {
    // Start session
    let session = start_ruby_session(...).await.unwrap();

    // Set breakpoint
    set_breakpoint(session, line=9).await.unwrap();

    // Continue
    continue_execution(session).await.unwrap();

    // Wait for breakpoint (with timeout!)
    let stopped = wait_for_stop(session, timeout=5000).await.unwrap();
    assert_eq!(stopped.reason, "breakpoint");

    // Evaluate variable
    let result = evaluate(session, "n", frame_id).await.unwrap();
    assert!(result.is_number());
}
```

## Timeout Implementation (CRITICAL)

While fixing the socket issue, also add timeouts to prevent hangs:

```rust
pub struct DapClient {
    // ... existing fields
    default_timeout: Duration, // = 5 seconds
}

impl DapClient {
    pub async fn request_with_timeout(&self, request: Request, timeout: Duration) -> Result<Response> {
        tokio::time::timeout(timeout, self.request(request))
            .await
            .map_err(|_| Error::Timeout(format!("Request timed out after {:?}", timeout)))?
    }

    pub async fn disconnect(&self) -> Result<()> {
        // Always use timeout for disconnect
        let timeout = Duration::from_secs(5);
        match tokio::time::timeout(timeout, self.send_disconnect()).await {
            Ok(result) => result,
            Err(_) => {
                // Timeout - force cleanup
                warn!("Disconnect timed out, forcing cleanup");
                self.force_cleanup();
                Ok(())
            }
        }
    }
}
```

## User Experience Improvements

1. **Better Error Messages**:
   ```rust
   Err(Error::InitializeFailed(format!(
       "Ruby debugger failed to initialize within 3 seconds.\n\
        Possible causes:\n\
        - rdbg process failed to start\n\
        - Port {} may be blocked\n\
        - Program exited immediately\n\
        \n\
        Check logs with: docker logs <container-id>\n\
        Current state: {:?}",
       port, session.state()
   )))
   ```

2. **Timeouts on All Operations**:
   - `initialize`: 3 seconds
   - `disconnect`: 5 seconds
   - `wait_for_stop`: 5 seconds (already implemented)
   - Generic DAP requests: 10 seconds

3. **Process Health Monitoring**:
   ```rust
   tokio::spawn(async move {
       let exit_status = child.wait().await;
       if !session.is_terminated() {
           session.set_error(format!("rdbg exited unexpectedly: {:?}", exit_status));
       }
   });
   ```

## Summary

**Root Cause**: rdbg doesn't support DAP via stdio, only via socket.

**Solution**: Switch Ruby adapter to use socket mode (`--open --port`).

**Timeline**: 3-4 hours implementation + testing.

**Benefits**:
- ✅ Ruby debugging will actually work
- ✅ Native rdbg support (well-tested)
- ✅ Opportunity to add timeouts and error handling
- ✅ Better user experience overall

**Next Steps**:
1. Implement RubySocketAdapter
2. Update DapTransport for socket support
3. Add comprehensive timeouts
4. Test end-to-end
5. Update documentation

---

**Status**: Investigation Complete - Ready for Implementation

**Priority**: CRITICAL - Ruby debugging currently 100% non-functional

**Related Files**:
- `src/adapters/ruby.rs` (needs major refactor)
- `src/dap/transport.rs` (add socket support)
- `src/dap/client.rs` (add timeout wrappers)
- `src/debug/manager.rs` (update Ruby case)
