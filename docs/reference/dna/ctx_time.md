# `ctx.time` Engine API

The `ctx.time()` API provides sandboxed scripts with the current Unix Epoch timestamp (in milliseconds). It is essential for managing TTLs (Time-To-Live), auditing, and sequential ordering.

## Architectural Execution

### 1. Syscall Optimization via Atomic Clocks
In high-performance compute nodes, repeatedly asking the Operating System for the current time via syscalls (e.g., `clock_gettime()`) introduces severe kernel-space to user-space context switching overhead. 

To prevent this, the cluaizdb Engine runs a dedicated background thread that queries the OS clock once every millisecond. It stores this timestamp in a highly optimized `AtomicU64` memory address. When `ctx.time()` is called inside the WASM sandbox, it simply reads this atomic integer from RAM without ever triggering a kernel syscall.

### 2. Monotonic Guarantees
The atomic clock utilized by `ctx.time()` strictly adheres to a monotonic progression. It is immune to NTP (Network Time Protocol) drift or leap seconds, ensuring that a timestamp generated during an `on_write` hook is never logically "older" than a previous transaction.

## Time Complexity

| Operation       | Complexity | Notes                                                                |
| :-------------- | :--------- | :------------------------------------------------------------------- |
| **Atomic Read** | **O(1)**   | Executes in ~1-2 CPU nanoseconds due to cache-line hit optimization. |

## Example: Auto-Expiring Records (TTL)

```rust
fn on_write(ctx) {
    let mut payload = ctx.get_payload_json();
    
    // Automatically attach an expiration timestamp 24 hours into the future
    let current_epoch_ms = ctx.time();
    let one_day_ms = 86400 * 1000;
    
    payload.insert("expires_at", current_epoch_ms + one_day_ms);
    
    ctx.set_payload_json(payload);
}
```
