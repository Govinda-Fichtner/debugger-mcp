# Event-Driven DAP Client Architecture

## Design Goals

1. **Non-blocking requests**: Send requests without waiting for responses
2. **Event callbacks**: Register handlers for DAP events
3. **Response callbacks**: Handle responses asynchronously
4. **Proper DAP sequence**: Follow the spec exactly as nvim-dap does

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         DapClient                            │
├─────────────────────────────────────────────────────────────┤
│ Event Handlers:                                             │
│   - Map<String, Vec<EventCallback>>                         │
│                                                              │
│ Pending Requests:                                           │
│   - Map<i32, ResponseCallback>                              │
│                                                              │
│ Message Handler Task:                                       │
│   - Reads messages continuously                             │
│   - Dispatches events to registered callbacks               │
│   - Dispatches responses to request callbacks               │
└─────────────────────────────────────────────────────────────┘
```

## Core Types

```rust
// Callback for handling events
type EventCallback = Box<dyn Fn(Event) + Send + Sync>;

// Callback for handling responses
type ResponseCallback = Box<dyn FnOnce(Result<Response>) + Send>;

// Event handler registration
pub struct EventHandler {
    callbacks: Vec<EventCallback>,
}
```

## API Design

### Synchronous API (for simple cases)
```rust
// Existing API, implemented using callbacks internally
pub async fn initialize(&self, adapter_id: &str) -> Result<Capabilities>
pub async fn launch(&self, args: Value) -> Result<()>
```

### Async API (for complex flows)
```rust
// Send request with callback for response
pub async fn send_request_async<F>(&self, command: &str, args: Option<Value>, callback: F)
where F: FnOnce(Result<Response>) + Send + 'static

// Register event handler
pub fn on_event<F>(&self, event_name: &str, handler: F)
where F: Fn(Event) + Send + Sync + 'static

// One-time event handler (removed after first invocation)
pub fn once_event<F>(&self, event_name: &str, handler: F)
where F: FnOnce(Event) + Send + 'static
```

## Implementation Strategy

### Phase 1: Add Callback Infrastructure
- Add event callback storage
- Add response callback storage
- Update message_handler to invoke callbacks

### Phase 2: Implement Async Methods
- `send_request_async()` - send without blocking
- `on_event()` - register persistent event handler
- `once_event()` - register one-time event handler

### Phase 3: Refactor Synchronous Methods
- Keep existing `initialize()`, `launch()` etc.
- Implement them using the async/callback infrastructure
- Use channels to bridge async → sync

### Phase 4: Implement Proper DAP Sequence
```rust
// Proper initialization sequence
pub async fn initialize_and_launch(&self, adapter_id: &str, launch_args: Value) -> Result<()> {
    // 1. Send initialize request
    let caps = self.initialize(adapter_id).await?;

    // 2. Register initialized event handler BEFORE sending launch
    let (tx, rx) = oneshot::channel();
    self.once_event("initialized", move |_event| {
        // In the initialized handler, send configurationDone
        // This happens DURING launch processing
    });

    // 3. Send launch request (will trigger initialized event)
    self.send_request_async("launch", Some(launch_args), move |result| {
        // Launch response arrives AFTER configurationDone
        tx.send(result).ok();
    });

    // 4. Wait for launch to complete
    rx.await??;

    Ok(())
}
```

## Example Usage

```rust
// Client code
let client = DapClient::spawn("python", &["-m", "debugpy.adapter"]).await?;

// Set up event handlers
client.on_event("stopped", |event| {
    println!("Debugger stopped: {:?}", event);
});

client.on_event("output", |event| {
    println!("Output: {:?}", event);
});

// Initialize and launch
client.initialize_and_launch("debugpy", launch_args).await?;

// Session is now running, events are handled by callbacks
```

## Benefits

1. **Matches DAP spec exactly**: Events can arrive at any time
2. **No deadlocks**: No blocking waits that create circular dependencies
3. **Flexible**: Can handle any adapter's event timing
4. **Compatible**: Existing synchronous API still works

## Migration Path

1. Implement new infrastructure alongside existing code
2. Add new async methods
3. Refactor existing methods to use new infrastructure internally
4. Tests continue to work with minimal changes
5. Users can opt into async API when needed

## Testing Strategy

1. Test basic event registration and invocation
2. Test response callbacks
3. Test proper DAP sequence with debugpy
4. Test with Ruby adapter (different timing)
5. Integration tests verify full workflows
