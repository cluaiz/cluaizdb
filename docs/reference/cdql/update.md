# `update` Command Reference

The `update` command in CDQL modifies existing records in place. The cluaizdb engine handles updates via Multi-Version Concurrency Control (MVCC) to ensure that reads are never blocked by concurrent writes.

## Syntax Rules

```text
update [ TARGET ] where [ CONDITION ] set [ FIELD ] = [ VALUE ]
```

### Modifiers
- `set` : Overwrites or adds a specific key-value pair to the JSON payload.
- `unset` : Removes a key from the JSON payload.

---

## Architecture: How it works under the hood

Updating a record in a zero-copy memory-mapped database involves delicate pointer arithmetic and disk reallocation.

### 1. Read-Copy-Update (RCU) Mechanics
Because the primary memory map is read-only for concurrent `find` queries, the `update` command does not immediately overwrite the raw bytes in the memory page if the new payload is larger than the old one. Instead, it allocates a new memory block, serializes the updated payload into it, and atomically swaps the internal B-Tree pointer. The old memory block is marked for garbage collection.

### 2. WAL Append
Every successful update is appended to the Write-Ahead Log (WAL) before the TCP response is sent to the client, guaranteeing durability.

### 3. Execution Affordance (WASM Hooks)
If the targeted record contains an `on_update` WASM execution script within its `dna` field, the script is invoked *before* the final commit. If the WASM script returns an error, the transaction is rolled back.

---

## Time Complexity

| Operation            | Complexity   | Notes                                                                        |
| :------------------- | :----------- | :--------------------------------------------------------------------------- |
| **Indexed Update**   | **O(log N)** | Extremely fast. B-Tree index lookup followed by a constant-time memory swap. |
| **Unindexed Update** | **O(N)**     | Requires a full scan to evaluate the `where` condition before updating.      |

---

## Examples

### 1. Basic Field Update

Modifying the `status` field for a specific user ID.

```json
{
  "cdql": "update json where user_id == 101 set status = 'verified'"
}
```

### 2. Field Deletion

Removing a deprecated field from a record.

```json
{
  "cdql": "update json where session_active == false unset temporary_token"
}
```
