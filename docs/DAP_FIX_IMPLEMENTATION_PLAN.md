# DAP Protocol Fix - Implementation Plan

## Problem Statement
Our debugger times out during `launch` because we don't follow the correct DAP sequence. We send `launch` and wait for a response, but the adapter won't respond until we send `configurationDone`.

## Solution: Approach 1 (Simplest)

**Send operations in the correct order**: `initialize` → wait for `initialized` event → `configurationDone` → `launch`

## Implementation Steps

### Step 1: Add Event Notification to DapClient

**File**: `src/dap/client.rs`

Add a mechanism to wait for specific events:

```rust
use tokio::sync::Notify;
use std::collections::HashMap;

pub struct DapClient {
    // ... existing fields ...
    event_notifiers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
}

impl DapClient {
    // Add method to wait for an event
    pub async fn wait_for_event(&self, event_name: &str, timeout: Duration) -> Result<()> {
        let notify = {
            let mut notifiers = self.event_notifiers.write().await;
            let notify = Arc::new(Notify::new());
            notifiers.insert(event_name.to_string(), notify.clone());
            notify
        };

        tokio::select! {
            _ = notify.notified() => Ok(()),
            _ = tokio::time::sleep(timeout) => {
                Err(Error::Dap(format!("Timeout waiting for {} event", event_name)))
            }
        }
    }
}
```

Update `message_handler` to notify waiters:

```rust
Message::Event(event) => {
    debug!("Received event: {}", event.event);

    // Notify anyone waiting for this event
    let notifiers = self.event_notifiers.read().await;
    if let Some(notify) = notifiers.get(&event.event) {
        notify.notify_one();
    }
}
```

### Step 2: Update DebugSession::initialize

**File**: `src/debug/session.rs`

Make `initialize` wait for the `initialized` event:

```rust
pub async fn initialize(&self, adapter_id: &str) -> Result<()> {
    {
        let mut state = self.state.write().await;
        state.set_state(DebugState::Initializing);
    }

    let client = self.client.read().await;

    // Send initialize request and get response
    client.initialize(adapter_id).await?;

    // Wait for 'initialized' event (with 5 second timeout)
    client.wait_for_event("initialized", Duration::from_secs(5)).await?;

    {
        let mut state = self.state.write().await;
        state.set_state(DebugState::Initialized);
    }

    Ok(())
}
```

### Step 3: Update DebugSession::launch

**File**: `src/debug/session.rs`

Send `configurationDone` BEFORE sending `launch`:

```rust
pub async fn launch(&self, launch_args: serde_json::Value) -> Result<()> {
    {
        let mut state = self.state.write().await;
        state.set_state(DebugState::Launching);
    }

    let client = self.client.read().await;

    // IMPORTANT: Send configurationDone FIRST
    // The adapter will not respond to launch until this is done
    client.configuration_done().await?;

    // NOW send launch - response will come back immediately
    client.launch(launch_args).await?;

    {
        let mut state = self.state.write().await;
        state.set_state(DebugState::Running);
    }

    Ok(())
}
```

### Step 4: Cleanup

**File**: `src/debug/manager.rs`

No changes needed! The sequence is already correct:
```rust
session.initialize(adapter_id).await?;  // Now waits for 'initialized' event
session.launch(launch_args).await?;     // Now sends configurationDone first
```

## Alternative: Simpler Approach (No Event Waiting)

If we don't want to add event waiting infrastructure, we can use a simpler approach:

**Just send `configurationDone` before `launch`** without waiting for `initialized` event.

Most adapters send the `initialized` event immediately after the `initialize` response, so in practice, this should work:

```rust
// In DebugSession::initialize
pub async fn initialize(&self, adapter_id: &str) -> Result<()> {
    let client = self.client.read().await;
    client.initialize(adapter_id).await?;

    // Send configurationDone immediately
    // (The adapter has likely already sent 'initialized' event)
    client.configuration_done().await?;

    Ok(())
}

// In DebugSession::launch
pub async fn launch(&self, launch_args: serde_json::Value) -> Result<()> {
    let client = self.client.read().await;

    // Just send launch - configurationDone was already sent
    client.launch(launch_args).await?;

    Ok(())
}
```

## Recommendation

**Start with the simpler approach** (no event waiting) because:
1. Less code change
2. Easier to test
3. Most adapters send `initialized` event immediately
4. If needed, we can add event waiting later

## Testing

After implementation:
```bash
# Test with FizzBuzz integration test
cargo test --test integration_test test_fizzbuzz_debugging_integration -- --ignored --nocapture

# Expected: Should complete without timeout
# Should see breakpoint hit and stack trace
```

## Files to Modify

**Minimal approach (recommended first)**:
1. `src/debug/session.rs` - Add `configuration_done()` call to `initialize()`
2. Remove `configuration_done()` call from `launch()` (we already added it)
3. Test!

**Full approach (if needed)**:
1. `src/dap/client.rs` - Add event waiting mechanism
2. `src/debug/session.rs` - Use event waiting in initialize
3. Test!
