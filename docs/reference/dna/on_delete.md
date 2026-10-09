# `on_delete` Event Hook

The `on_delete` hook is triggered precisely before the database engine inserts a tombstone marker for a targeted record. 

## Architectural Execution

### 1. Blocking Cascades
The execution of `on_delete` is inherently blocking. If the DNA script attached to the record requires querying other records, the engine pauses the primary deletion transaction to ensure ACID compliance across the cascade.

### 2. Rollback Capability
Like `on_write`, the `on_delete` script retains the ability to invoke `ctx.abort()`. If abort is called, the tombstone is not written to the B-Tree, and the deletion is canceled.

## Time Complexity & Performance

| Operation                   | Complexity | Notes                                                                       |
| :-------------------------- | :--------- | :-------------------------------------------------------------------------- |
| **Simple Deletion Hook**    | **O(1)**   | Constant time execution overhead.                                           |
| **Cascading Graph Pruning** | **O(E)**   | Where `E` is the number of edges being forcefully deleted via script logic. |

## Example: Cascading Deletions

In relational databases, this requires defining foreign keys and `ON DELETE CASCADE` schemas. In cluaizdb, the record itself dictates its cascade logic.

```rust
fn on_delete(ctx) {
    let payload = ctx.get_payload_json();
    
    // If a user is deleted, we must also delete their session tokens
    if let Some(session_id) = payload.get("active_session_id") {
        
        // Use the Engine API to issue a secondary deletion query
        // This runs inside the current database transaction
        ctx.execute_cdql(format!("delete where id == '{}'", session_id));
    }
    
    // Allow the tombstone insertion to proceed
}
```
