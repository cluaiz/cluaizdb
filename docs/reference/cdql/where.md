# `where` Command Reference

The `where` clause is the primary filtering operator in the cluaizdbd Query Language (CDQL). It allows conditional evaluation of records during a traversal, limiting the output to payloads that satisfy the given conditions.

## Syntax Rules

```text
<COMMAND> <TARGET> where <FIELD> <OPERATOR> <VALUE>
```

### Supported Commands
- `find`
- `update`
- `delete`

### Supported Operators
- `==` : Strict equality
- `!=` : Strict inequality
- `>`  : Greater than (alternatively `$gt`)
- `<`  : Less than (alternatively `$lt`)
- `>=` : Greater than or equal to
- `<=` : Less than or equal to

---

## Architecture: How it works under the hood

The cluaizdb engine handles the `where` clause differently depending on the configured runtime schema and indexing.

### 1. Byte-Level Early Rejection
If the query targets a structured field without indexing, the engine attempts to scan the raw byte-stream for the specific key sequence (e.g., `"status":"active"`) directly within the LMDB memory map. If the bytes do not match, the record is rejected instantly, preventing the CPU cost of deserializing the entire JSON payload into the heap.

### 2. WASM Predicate Evaluation
If complex logic is required, the `where` clause can be compiled into a micro-WASM execution affordance. The cluaizdb engine passes a memory pointer to the WASM runtime, which evaluates the condition and returns a boolean `true`/`false`. This allows Turing-complete filtering without IPC (Inter-Process Communication) overhead.

---

## Time Complexity

| Scenario                    | Complexity   | Notes                                                                                                            |
| :-------------------------- | :----------- | :--------------------------------------------------------------------------------------------------------------- |
| **Unindexed Filter**        | **O(N)**     | Requires a sequential scan of all records in the shard. Highly optimized via Zero-Copy, but still linear.        |
| **Indexed Filter (B-Tree)** | **O(log N)** | If the field is indexed, the engine traverses the secondary B-Tree index to locate the memory offsets instantly. |

---

## Examples

### 1. Basic Equality Filter

```json
{
  "cdql": "find json where role == 'admin'"
}
```

### 2. Chained Mutations (Update)

The `where` clause is critical for targeted mutations, ensuring that only specific records are updated without affecting the entire shard.

```json
{
  "cdql": "update json where session_id == '12345' set status = 'expired'"
}
```

### 3. Deletion Filter

Permanently removes records matching the condition from the physical disk and memory map.

```json
{
  "cdql": "delete where ttl < 1690000000"
}
```
