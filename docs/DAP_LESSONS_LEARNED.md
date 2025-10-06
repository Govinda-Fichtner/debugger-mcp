# DAP Protocol Implementation - Lessons Learned

## Summary

After extensive testing and research, we discovered that the Debug Adapter Protocol (DAP) has complex asynchronous event flows that don't map cleanly to synchronous APIs.

## Root Cause of Timeout Issue

**debugpy's Behavior**:
1. `initialize` request → `initialize` response (NO `initialized` event yet)
2. `launch` request is sent
3. **DURING `launch` processing**, adapter sends `initialized` event
4. Adapter waits for `configurationDone` request
5. **ONLY THEN** does adapter send `launch` response

**Our Problem**:
- We send `launch()` and block waiting for the response
- But the response won't come until we send `configurationDone`
- But we can't send `configurationDone` until we receive `initialized` event
- The `initialized` event only comes during `launch` processing
- **Classic deadlock!**

## What We Implemented

### Option B: Event Notification System ✅

We successfully implemented:
1. Event notification infrastructure using `tokio::sync::Notify`
2. `wait_for_event()` method in DapClient
3. Event notifiers in message_handler

### What's Still Needed

The proper solution requires **event-driven callbacks**, not synchronous waiting:

```rust
// Pseudo-code for proper solution
client.on_event("initialized", || {
    // Set breakpoints here if needed
    client.send_configurationDone();
});

client.send_initialize();
client.send_launch(); // Don't wait for response yet
// Responses and events are handled asynchronously
```

## Insights from nvim-dap

nvim-dap (https://github.com/mfussenegger/nvim-dap) handles this correctly:

1. Uses Lua coroutines for async operations
2. Registers event handlers BEFORE sending requests
3. The `initialized` event handler sends `configurationDone`
4. No synchronous blocking on responses

Key code from nvim-dap:
```lua
function Session:event_initialized()
  local function on_done()
    if self.capabilities.supportsConfigurationDoneRequest then
      self:request('configurationDone', nil, function(err1, _)
        self.initialized = true
      end)
    end
  end
  -- Set breakpoints, then call on_done
  self:set_breakpoints(bps, on_done)
end
```

## Recommended Solutions

### Short-term Fix (Hacky but Works)

Send requests without waiting for all responses:

```rust
// In DapClient, add a method to send without waiting
pub async fn send_request_nowait(&self, command: &str, args: Option<Value>) -> Result<()> {
    let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);
    let request = Request { seq, command: command.to_string(), arguments: args };

    let mut transport = self.transport.write().await;
    transport.write_message(&Message::Request(request)).await?;
    Ok(())
}

// Then in session.launch():
client.send_request_nowait("launch", Some(launch_args)).await?;
tokio::time::sleep(Duration::from_millis(200)).await;
client.wait_for_event("initialized", Duration::from_secs(5)).await?;
client.configuration_done().await?;
// Launch response will have been processed by message_handler
```

### Long-term Fix (Proper Architecture)

Implement full event-driven architecture:

1. **Event Callbacks**:
```rust
pub struct DapClient {
    event_handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Event) + Send + Sync>>>>,
}

impl DapClient {
    pub async fn on_event<F>(&self, event_name: &str, handler: F)
    where F: Fn(Event) + Send + Sync + 'static
    {
        let mut handlers = self.event_handlers.write().await;
        handlers.insert(event_name.to_string(), Box::new(handler));
    }
}
```

2. **Async Request Handling**:
```rust
// Launch returns immediately, response handled via callback
pub async fn launch_async<F>(&self, args: Value, on_complete: F) -> Result<()>
where F: FnOnce(Result<Response>) + Send + 'static
{
    // Register callback, send request, return immediately
}
```

3. **State Machine**:
```rust
enum SessionState {
    Created,
    Initializing,
    WaitingForInitialized,
    Configured,
    Launching,
    Running,
    Stopped,
    Terminated,
}
```

## Alternative: Use Different DAP Library

Consider using existing Rust DAP implementations:
- https://github.com/vanilla-technologies/dap-rs
- Or study how VS Code handles this in TypeScript

## Testing with Multiple Adapters

To validate our implementation works across different DAP adapters (not just debugpy-specific hacks), we should test with:

1. **Python** - debugpy (current)
2. **Ruby** - rdbg (different behavior)
3. **Go** - delve
4. **Rust** - rust-analyzer / lldb

Each adapter may have slightly different timing and event ordering.

## Current Status

- ✅ Event notification infrastructure implemented
- ✅ Basic DAP sequence understanding documented
- ❌ Deadlock issue with debugpy not resolved
- ❌ Need callback-based event handling
- ❌ Integration test still times out

## Next Steps

1. Implement `send_request_nowait()` method
2. Test short-term fix
3. If that works, consider refactoring to full event-driven architecture
4. Add Ruby adapter support to validate solution works across adapters
5. Document the final working sequence

## References

- DAP Spec: https://microsoft.github.io/debug-adapter-protocol
- nvim-dap: https://github.com/mfussenegger/nvim-dap
- Our research: `/docs/DAP_PROTOCOL_SEQUENCE.md`
- Implementation plan: `/docs/DAP_FIX_IMPLEMENTATION_PLAN.md`
