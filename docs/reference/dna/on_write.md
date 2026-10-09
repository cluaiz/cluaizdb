# `on_write` Event Hook

The `on_write` hook is a synchronous execution affordance triggered whenever a mutation (Insert or Update) occurs on a record possessing this specific DNA script. 

## Architectural Execution

When a client submits a payload to the cluaizdb Engine, the mutation does not immediately hit the Write-Ahead Log (WAL).

1. **Sandboxed Instantiation**: The engine detects the `on_write` hook and spins up a lightweight WASM instance (via `wasmtime` or `rhai`). 
2. **Memory Pointer Passing**: To avoid copying the payload, the engine passes a raw memory pointer (offset and length) to the WASM sandbox.
3. **Evaluation**: The script executes. If the script modifies the payload (e.g., auto-hashing a password), the engine writes the mutated buffer to the WAL.
4. **Commit or Rollback**: If the script calls `ctx.abort()`, the entire transaction is rolled back, preventing invalid data ingestion.

## Time Complexity & Performance

| Operation              | Complexity   | Overhead                                                        |
| :--------------------- | :----------- | :-------------------------------------------------------------- |
| **WASM Instantiation** | **O(1)**     | ~5-10 microseconds (mitigated via pre-compiled module caching). |
| **Execution**          | **Variable** | Depends entirely on the logic written by the developer.         |

## Example: Data Validation & Modification

```rust
// A Rhai or Rust WASM script injected into the DNA
fn on_write(ctx) {
    let mut payload = ctx.get_payload_json();

    // 1. Validation (Aborts transaction if age is invalid)
    if payload.age < 18 {
        ctx.abort("Age must be 18 or older.");
        return;
    }

    // 2. Mutation (Auto-computes a field before saving to disk)
    payload.created_epoch = ctx.time();
    
    // Write back the mutated payload to the engine memory
    ctx.set_payload_json(payload);
}
```
