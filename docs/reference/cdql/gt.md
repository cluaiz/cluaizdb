# `$gt` Command Reference

The `$gt` (Greater Than) operator, often written simply as `>`, is a logical comparison directive used within the `where` clause. It filters numerical or lexicographical records, enforcing a strict boundary constraint.

## Syntax Rules

```text
where [ FIELD ] > [ VALUE ]
```
Alternatively:
```text
where [ FIELD ] $gt [ VALUE ]
```

---

## Architecture: How it works under the hood

Evaluating a `$gt` condition requires the engine to ensure type-safety at runtime without heavily penalizing query speed.

### 1. Zero-Allocation Type Coercion
When the cluaizdb engine reads a value from the storage layer (e.g., an integer field like `age: 25`), it does not instantiate a full JSON Object on the heap. Instead, the parser reads the raw byte-stream. When it encounters the `$gt` operator in the query tree, it performs a zero-allocation cast, comparing the raw parsed integer directly against the query parameter.

### 2. B-Tree Range Scans
If the field specified in the `$gt` clause is indexed, the engine leverages the B-Tree index to perform an optimized Range Scan. Instead of checking every record, the iterator seeks the exact node where the boundary condition is met and streams all subsequent records sequentially until the end of the index or the `limit` is reached.

---

## Time Complexity

| Scenario               | Complexity       | Notes                                                                                      |
| :--------------------- | :--------------- | :----------------------------------------------------------------------------------------- |
| **Indexed Range Scan** | **O(log N + K)** | Seeks the boundary in logarithmic time, then streams `K` matching records. Highly optimal. |
| **Unindexed Filter**   | **O(N)**         | Engine must load and cast the field for every record in the shard.                         |

---

## Examples

### 1. Standard Numerical Filter

Retrieving all user accounts older than 18.

```json
{
  "cdql": "find json where age > 18"
}
```

### 2. Timestamp Filtering

Fetching records created after a specific Unix Epoch timestamp.

```json
{
  "cdql": "find json where created_at $gt 1700000000"
}
```
