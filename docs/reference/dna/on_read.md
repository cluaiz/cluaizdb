# `on_read` Event Hook

The `on_read` execution affordance is invoked during a `find` query, immediately after the engine locates the record but *before* the payload is serialized over the network socket to the client.

## Architectural Execution

### 1. Zero-Copy Interception
During a massive sequential scan (`find * limit 100000`), invoking a WASM hook on every single record could introduce catastrophic latency. Cluaizdb solves this via **JIT Compilation and Memory Aliasing**. The `on_read` script is compiled once per query batch. As the engine streams over the LMDB memory map, the WASM runtime merely shifts its internal pointer window to look at the next record.

### 2. Ephemeral Mutation
Unlike `on_write`, any modifications made to the payload inside the `on_read` hook are **strictly ephemeral**. The engine allocates a tiny scratchpad buffer for the modified output. The physical disk and primary memory map are never altered.

## Use Cases

### Dynamic Redaction (Security)
Redacting PII (Personally Identifiable Information) or sensitive fields based on the query context before the data leaves the database engine.

### On-the-Fly Computation
Calculating ages from birthdates or summing internal arrays without permanently storing the computed values on disk.

## Example: Secure Redaction

```rust
fn on_read(ctx) {
    let mut payload = ctx.get_payload_json();
    
    // Check if the executing user (from context) lacks admin privileges
    if ctx.get_auth_role() != "admin" {
        // Redact the credit card number instantly
        payload.remove("credit_card_num");
        payload.insert("credit_card_num", "****-****-****-****");
    }
    
    ctx.set_ephemeral_json(payload);
}
```
