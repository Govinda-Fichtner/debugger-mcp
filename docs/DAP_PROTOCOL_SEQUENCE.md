# Debug Adapter Protocol (DAP) - Correct Sequence

## Official Resources
- https://microsoft.github.io/debug-adapter-protocol
- https://github.com/microsoft/debug-adapter-protocol

## The Correct Initialization & Launch Sequence

Based on the official DAP specification, here is the **correct** sequence:

### 1. Initialize Phase
```
Client → Adapter: initialize request
Adapter → Client: initialize response (with capabilities)
Adapter → Client: initialized EVENT
```

**Key Point**: The `initialized` event signals that the adapter is **ready to receive configuration**.

### 2. Configuration Phase
```
Client → Adapter: setBreakpoints request(s) [optional, zero or more]
Client → Adapter: setFunctionBreakpoints request [optional]
Client → Adapter: setExceptionBreakpoints request [optional]
Client → Adapter: configurationDone request
Adapter → Client: configurationDone response
```

**Key Point**: `configurationDone` marks the **end of configuration**.

### 3. Launch/Attach Phase
```
Client → Adapter: launch (or attach) request
[Adapter may send events during launch]
Adapter → Client: launch response (AFTER configurationDone completes)
```

**Critical Discovery**: The `launch` response is **not sent until after `configurationDone`** completes!

## What We Were Doing Wrong

### Our Incorrect Sequence
```rust
// In SessionManager::create_session
client.spawn()           // OK
session.initialize()     // OK - sends initialize, gets response
session.launch()         // ❌ WRONG - sends launch, waits for response
                        // This BLOCKS because adapter is waiting for configurationDone!
```

### The Problem
1. We send `launch` request
2. Adapter sends `initialized` EVENT (not a response!)
3. Adapter waits for `configurationDone` before sending `launch` RESPONSE
4. Our code is blocked waiting for `launch` response
5. **Deadlock** - we never send `configurationDone`, adapter never responds to `launch`

## The Correct Flow

### Option A: Synchronous API with Hidden Event Handling
```rust
// In SessionManager::create_session
session.initialize()           // Sends initialize, gets response
                              // Internally handles 'initialized' event
session.configure_and_launch() // Sends configurationDone, then launch
                              // Waits for both responses
```

### Option B: Event-Driven API
```rust
// In SessionManager::create_session
session.initialize()     // Sends initialize, gets response

// Handle 'initialized' event
session.on_initialized(|| {
    // Set breakpoints here if needed
    session.configuration_done()
    session.launch()
})
```

### Option C: Explicit Sequence
```rust
// In SessionManager::create_session
session.initialize()           // Sends initialize, gets response
wait_for_initialized_event()   // Wait for 'initialized' event
session.configuration_done()   // Send configurationDone, get response
session.launch()              // Send launch, get response
```

## Recommended Approach

**Option C (Explicit Sequence)** is the clearest and follows the DAP spec most directly.

### Implementation Requirements

1. **Event Handling**: Our `message_handler` already processes events, but we need a way to **wait for** or **be notified of** specific events.

2. **Event Channel**: Add a mechanism to notify callers when specific events arrive:
   ```rust
   // When message_handler receives an 'initialized' event
   if event.event == "initialized" {
       // Notify waiting code
       initialized_notify.notify_one();
   }
   ```

3. **Refactor DapClient**:
   ```rust
   pub async fn initialize_and_wait(&self) -> Result<Capabilities> {
       // Send initialize request
       let caps = self.send_request("initialize", ...).await?;

       // Wait for 'initialized' event
       self.wait_for_event("initialized").await?;

       Ok(caps)
   }
   ```

4. **Update DebugSession::launch**:
   ```rust
   pub async fn launch(&self, launch_args: Value) -> Result<()> {
       let client = self.client.read().await;

       // Send configurationDone FIRST
       client.configuration_done().await?;

       // NOW send launch (response will come back)
       client.launch(launch_args).await?;

       Ok(())
   }
   ```

## Key Insights from DAP Spec

1. **`initialized` is an EVENT, not a response**
   - It signals readiness for configuration
   - Must be handled asynchronously

2. **`configurationDone` gates the `launch` response**
   - The adapter will NOT respond to `launch` until `configurationDone` is sent
   - This is by design to allow breakpoint configuration before execution starts

3. **Event-driven protocol**
   - DAP is fundamentally asynchronous
   - Events can arrive at any time
   - Must handle events separately from request-response pairs

## Testing the Fix

After implementing the fix, test with:
```bash
cargo test --test integration_test test_fizzbuzz_debugging_integration -- --ignored --nocapture
```

Expected behavior:
- No timeout on `debugger_start`
- Breakpoints are hit
- Stack trace is retrieved
- Expression evaluation works

## References

From DAP Spec:
> "Once the debug adapter is ready to receive configuration from the client, it sends an initialized event to the client. The client sends zero or more configuration-related requests before sending a configurationDone request. **After the response to configurationDone is sent, the debug adapter may respond to the launch or attach request**, and then the debug session has started."

This clearly states that `launch` response comes **after** `configurationDone` response.
