# MCP Notifications and Event Subscriptions

## Your Questions

### Q1: Does MCP have a subscription mechanism via STDIO?
**Answer: Yes, but it's limited.**

### Q2: Can AI agent MCP clients subscribe to event types and get notified?
**Answer: Partially - MCP supports notifications, but not subscriptions in the traditional sense.**

## How MCP Notifications Work

### MCP Protocol Specification

MCP supports **server-to-client notifications** but **NOT client-initiated subscriptions**.

**What MCP Has:**
- Servers can send **notifications** to clients at any time
- Notifications are JSON-RPC messages without an `id` field
- Format: `{"jsonrpc": "2.0", "method": "notifications/...", "params": {...}}`

**What MCP Does NOT Have:**
- No `subscribe` method
- No way for clients to register interest in specific event types
- No filtering mechanism

### MCP Notification Types (from spec)

1. **notifications/progress**
   - Long-running operation updates
   - Example: File processing, downloads

2. **notifications/message**
   - Important messages to display to users
   - Levels: debug, info, warning, error

3. **notifications/resources/updated** (optional)
   - Server notifies client when resources change
   - Client can re-fetch updated resources

4. **notifications/resources/list_changed** (optional)
   - Resource list has changed
   - Client should re-fetch the list

5. **notifications/tools/list_changed** (optional)
   - Available tools have changed
   - Client should re-fetch tools list

## Could We Use This for Debugging Events?

### The Challenge

For debugging, we'd want notifications like:
- `notifications/debugger/stopped` - Program hit breakpoint
- `notifications/debugger/continued` - Program resumed
- `notifications/debugger/terminated` - Program exited

**But there are problems:**

1. **No Subscription Mechanism**
   - Client can't say "I want stopped events only"
   - Server would need to send ALL events to ALL clients
   - Wasteful and noisy

2. **Client Implementation Burden**
   - Client (Claude Code) needs to handle notifications
   - Not all MCP clients support notifications well
   - Claude Code would need updates to process debug notifications

3. **Timing Issues**
   - Notifications are fire-and-forget
   - No acknowledgment from client
   - If client misses a notification, it's gone

## What We Implemented Instead: debugger_wait_for_stop

Instead of notifications/subscriptions, we implemented **blocking wait**:

```rust
async fn debugger_wait_for_stop(&self, arguments: Value) -> Result<Value> {
    // Polls internal state until stopped or timeout
    loop {
        let state = session.get_state().await;

        if let DebugState::Stopped { thread_id, reason } = state {
            return Ok(json!({
                "state": "Stopped",
                "threadId": thread_id,
                "reason": reason
            }));
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
```

**Why This Works Better:**

1. ✅ **Simple**: Client just calls a tool and waits
2. ✅ **Reliable**: No missed events
3. ✅ **Efficient**: 50ms polling internally (client doesn't poll)
4. ✅ **Compatible**: Works with any MCP client
5. ✅ **Clear**: Returns explicit state info

## Theoretical MCP Notification Implementation

If we WERE to use MCP notifications (not recommended), it would look like:

### Server Side (debugger_mcp)

```rust
// In src/mcp/mod.rs
pub struct McpServer {
    transport: Arc<Mutex<McpTransport>>,
    // ... existing fields
}

impl McpServer {
    // Send notification when debugger stops
    pub async fn notify_stopped(&self, session_id: &str, thread_id: i32, reason: &str) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/debugger/stopped",
            "params": {
                "sessionId": session_id,
                "threadId": thread_id,
                "reason": reason
            }
        });

        self.transport.lock().await.write_message(&notification)?;
        Ok(())
    }
}

// In src/debug/session.rs - when registering 'stopped' event handler
client.on_event("stopped", move |event| {
    // ... existing code to update state ...

    // NEW: Send MCP notification
    let mcp_server = mcp_server_ref.clone();
    tokio::spawn(async move {
        mcp_server.notify_stopped(&session_id, thread_id, &reason).await;
    });
}).await;
```

### Client Side (Claude Code / AI Agent)

The client would need to:

1. **Process incoming notifications** while waiting for responses
2. **Handle notifications asynchronously** (separate from request/response)
3. **Store notification state** for later queries

```typescript
// Hypothetical Claude Code implementation
class McpClient {
    private debuggerStoppedListeners: Map<string, (event) => void> = new Map();

    // Register listener for stopped events
    onDebuggerStopped(sessionId: string, callback: (event) => void) {
        this.debuggerStoppedListeners.set(sessionId, callback);
    }

    // Process incoming messages
    private async processMessage(message: any) {
        if (message.method === "notifications/debugger/stopped") {
            const listener = this.debuggerStoppedListeners.get(message.params.sessionId);
            if (listener) {
                listener(message.params);
            }
        }
    }
}
```

### Problems with This Approach

1. **Complexity**: Both server and client need notification plumbing
2. **Client Support**: Not all MCP clients handle notifications well
3. **Ordering**: Notifications might arrive before tool response
4. **Lost Events**: If client is busy, notifications could be missed
5. **Debugging**: Harder to debug than simple blocking calls

## Our Solution is Better

`debugger_wait_for_stop` solves all the user's pain points without the complexity:

**User's Original Workflow** (bad):
```javascript
debugger_continue()
sleep(0.2)  // Arbitrary!
debugger_session_state()  // Might not be stopped yet
```

**With Notifications** (complex):
```javascript
// Setup listener (once)
mcp.onDebuggerStopped(sessionId, (event) => {
    console.log("Stopped at", event.reason);
    // Now what? How do I synchronize with my workflow?
});

debugger_continue()
// Wait for notification... somehow?
// Sleep? Poll a flag? This gets messy.
```

**With wait_for_stop** (simple and efficient):
```javascript
debugger_continue()
const result = debugger_wait_for_stop({sessionId, timeoutMs: 5000})
// Returns immediately when stopped, with all info!
// result = {"state": "Stopped", "threadId": 1, "reason": "breakpoint"}
```

## Comparison

| Feature | Notifications | wait_for_stop |
|---------|--------------|---------------|
| Client Complexity | High | Low |
| Server Complexity | Medium | Low |
| Event Loss Risk | Yes | No |
| Timing Control | Hard | Easy |
| Client Support | Limited | Universal |
| Debugging | Hard | Easy |
| User Experience | Complex | Simple |

## Recommendation

**Keep the current implementation** (`debugger_wait_for_stop`).

**Only consider notifications if:**
1. MCP spec adds formal subscription mechanism
2. Claude Code adds robust notification support
3. Multiple concurrent debugging sessions need coordination
4. Real-time UI updates are required (not applicable for CLI AI agent)

## Summary

**Your Questions:**
1. Does MCP have subscriptions? → No formal mechanism
2. Is it applicable to our problems? → Not really, `wait_for_stop` is better

**Our Solution:**
- Internally poll state every 50ms (fast enough)
- Client makes one blocking call
- Returns immediately when state changes
- Simple, reliable, efficient

This addresses the user's feedback about polling being inefficient, without the complexity and fragility of a notification system.
