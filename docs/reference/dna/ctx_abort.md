# `ctx.abort` Engine API

The `ctx.abort()` instruction is a critical safety mechanism exposed to WASM and Rhai execution affordances. It commands the cluaizdb to instantly halt the current execution thread and rollback any pending memory mutations.

## Architectural Execution

### 1. The WAL Rollback Mechanism
Because cluaizdb uses a Write-Ahead Log (WAL), data integrity is paramount. If an `on_write` or `on_delete` script invokes `ctx.abort()`, the engine simply discards the uncommitted buffer sitting in RAM. The physical WAL append operation is bypassed entirely, meaning the disk never registers the invalid transaction.

### 2. Lock Release
In high-concurrency environments, writes may temporarily lock specific B-Tree nodes. Calling `ctx.abort()` immediately signals the Engine's Thread Manager to release these Mutex locks, preventing deadlocks or prolonged system stalls caused by faulty WASM scripts.

## Time Complexity

| Operation         | Complexity | Notes                                                                                                          |
| :---------------- | :--------- | :------------------------------------------------------------------------------------------------------------- |
| **Abort Trigger** | **O(1)**   | Executed in constant time. The memory buffer is dropped instantly without requiring an expensive disk `fsync`. |

## Example: Validating Transactions

```rust
fn on_write(ctx) {
    let payload = ctx.get_payload_json();
    
    // Reject any records missing a timestamp
    if !payload.contains_key("created_at") {
        ctx.abort("Transaction Rejected: Missing 'created_at' timestamp.");
        return;
    }
    
    // Reject excessive payload sizes to protect the shard
    if ctx.get_payload_bytes() > 1024 * 1024 { // 1 Megabyte
        ctx.abort("Transaction Rejected: Payload exceeds 1MB limit.");
        return;
    }
}
```
