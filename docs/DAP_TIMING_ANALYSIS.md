# DAP Protocol Timing Analysis

**Date**: 2025-10-06
**Test**: `scripts/test_dap_standalone.py`
**Adapter**: debugpy (Python Debug Adapter)

## Executive Summary

We ran a standalone Python test with precise timing measurements to understand the exact sequence and timing of DAP messages with debugpy. This provides baseline timing data to compare against the Rust implementation.

## Measured Timing Intervals

### Successful Run Metrics

```
✅ Initialize request → response:           2.49ms   (FAST - simple handshake)
✅ Launch request → 'initialized' event:   318.78ms  (SLOW - adapter spawns debuggee)
✅ 'initialized' → configurationDone:        0.05ms  (INSTANT - event handler)
✅ configurationDone → response:            43.17ms  (MODERATE - adapter acks)
✅ configurationDone → launch response:     43.21ms  (NEARLY SAME - launch waits for config)
✅ TOTAL: Launch request → response:       362.03ms  (OVERALL SEQUENCE)
```

## Critical Timing Insight

**The 'initialized' event arrives ~319ms after the launch request, but the launch response only arrives ~43ms after configurationDone!**

This proves:
1. The adapter sends 'initialized' event **during** launch processing (after ~319ms)
2. The adapter **blocks** the launch response until configurationDone is received
3. Once configurationDone is sent, the launch response arrives quickly (~43ms)

### Timeline Visualization

```
Time 0ms:     Client sends initialize request
              ↓
Time 2.5ms:   Adapter responds to initialize (FAST!)
              ↓
Time 605ms:   Client sends launch request
              |
              | (Adapter spawning debuggee process...)
              |
Time 924ms:   Adapter sends 'initialized' event (319ms after launch!)
              ↓ (< 0.1ms - event handler reacts)
Time 924ms:   Client sends configurationDone
              ↓
Time 967ms:   Adapter responds to configurationDone (43ms)
Time 967ms:   Adapter responds to launch (43ms - AT SAME TIME!)
```

## Complete Message Sequence with Timestamps

```
[   0.00ms] SPAWN        Starting debugpy adapter process
[   0.36ms] SPAWN        Adapter process ready
[  39.61ms] RECV_EVENT   'output' event (telemetry)
[  39.64ms] RECV_EVENT   'output' event (telemetry)
[  39.66ms] RECV_EVENT   'debugpySockets' event
[ 501.90ms] SEND_REQ     initialize request                       (seq 1)
[ 504.40ms] RECV_RESP    initialize response                      (seq 1)
[ 605.06ms] SEND_REQ     launch request                           (seq 2)
[ 606.40ms] RECV_EVENT   'debugpySockets' event
[ 606.85ms] RECV_EVENT   'debugpySockets' event
[ 790.77ms] RECV_EVENT   'debugpySockets' event
[ 923.83ms] RECV_EVENT   'initialized' event         ← CRITICAL!
[ 923.88ms] SEND_REQ     configurationDone request                (seq 3)
[ 967.05ms] RECV_RESP    configurationDone response               (seq 3)
[ 967.09ms] RECV_RESP    launch response                          (seq 2) ← Unblocked!
[ 967.12ms] RECV_EVENT   'process' event
[ 967.26ms] RECV_EVENT   'output' event (program output)
[ 967.31ms] RECV_EVENT   'output' event (program output)
[ 967.42ms] RECV_EVENT   'thread' event
[1135.14ms] RECV_EVENT   'thread' event
[1687.73ms] RECV_EVENT   'exited' event
[1688.06ms] RECV_EVENT   'terminated' event
[1688.11ms] RECV_EVENT   'debugpySockets' event
[2012.46ms] SEND_REQ     disconnect request                       (seq 4)
[2012.82ms] RECV_RESP    disconnect response                      (seq 4)
[2012.85ms] RECV_EVENT   'debugpySockets' event
```

## Implications for Rust Implementation

### 1. Event Handler Must Be Fast (< 1ms)

The Python standalone test shows the event handler latency is **0.05ms** - essentially instant. This means:
- Event handler should NOT block
- Sending configurationDone should be fire-and-forget or async
- Cannot wait for locks in the event handler

### 2. Launch Request Blocks for ~362ms Total

The total time from sending launch to receiving the launch response is **362ms**. This is long enough that:
- Any code waiting for the launch response will block for a long time
- If the event handler cannot send configurationDone, the launch response will NEVER arrive
- This creates a deadlock if event handler and launch sender share locks

### 3. Message Handler Must Run Independently

The timing shows that:
- Events arrive while launch request is pending (319ms later)
- Event handler must process events without blocking on pending requests
- The message reader thread must not share locks with the request sender

## Why Our Rust Implementation Hangs

Based on these timing measurements, the Rust implementation hangs because:

### Problem: Lock Contention

```rust
// CURRENT RUST CODE (BROKEN):

// Step 1: Send launch request
self.send_request_async("launch", Some(launch_args), callback).await?;
// This acquires transport.write(), sends launch, releases lock
// Then WAITS for launch response via oneshot channel

// Step 2: Message handler receives 'initialized' event (319ms later)
// Event handler tries to call:
callback(event);  // This calls our registered closure

// Step 3: Our event handler closure tries to send configurationDone
self.configuration_done().await?;
// This tries to acquire transport.write()

// Step 4: DEADLOCK
// The launch request is waiting for a response
// But the adapter won't send the response until it receives configurationDone
// But configurationDone can't be sent because... [lock issue]
```

### The Real Issue

Looking at the code in `src/dap/client.rs`:
- `send_request_async` acquires `transport.write()`, sends message, then RELEASES lock ✅
- Message handler runs in separate task, acquires `transport.write()` to read ✅
- Event handler tries to call `configuration_done()` ✅
- But `configuration_done()` calls `send_request()` which waits for response ❌

**The actual problem**: Not lock contention, but the event handler is calling a **blocking** method (`configuration_done` → `send_request`) from within a closure that's invoked synchronously!

## The Fix

Based on timing analysis, we need:

```rust
// Event handler should spawn async task, NOT call blocking method directly
self.on_event("initialized", move |_event| {
    let self_clone = self_ref.clone();
    tokio::spawn(async move {
        // This runs in SEPARATE task, doesn't block event handler
        self_clone.configuration_done().await.ok();
    });
}).await;
```

Or even better:

```rust
// Use oneshot channel to signal, send from main context
let (tx, rx) = oneshot::channel();

self.on_event("initialized", move |_| {
    tx.send(()).ok();  // Just signal, don't send message
}).await;

// After launch request
self.send_request_nowait("launch", Some(launch_args)).await?;

// Wait for initialized signal
rx.await.ok();

// NOW send configurationDone from main context
self.configuration_done().await?;
```

## Comparison: Python (0.05ms) vs Rust (HANGS)

| Metric | Python Standalone | Rust Expected | Rust Actual |
|--------|------------------|---------------|-------------|
| Initialize latency | 2.49ms | ~5-10ms | ✅ Works |
| Launch → initialized | 318.78ms | ~300-350ms | ⏳ Never arrives |
| Event handler latency | 0.05ms | ~0.1ms | ∞ HANGS |
| configurationDone sent | 0.05ms after event | ~0.1ms | ❌ Never sent |
| Launch response | 43.21ms after config | ~40-50ms | ⏳ Never arrives |

The Rust implementation never completes because the event handler never successfully sends configurationDone.

## Conclusion

The timing analysis proves:
1. ✅ Our understanding of the DAP sequence is correct
2. ✅ The 'initialized' event timing is as expected (~300ms after launch)
3. ✅ Event-driven architecture is required
4. ❌ Our Rust event handler implementation blocks instead of being async
5. ❌ The event handler needs to either signal or spawn, not call blocking methods

**Next step**: Refactor the Rust event handler to use signaling instead of direct method calls.
