# `$lt` Command Reference

The `$lt` (Less Than) operator, typically written as `<`, is the inverse of `$gt`. It applies a strict upper-bound limit to numerical or lexicographical queries.

## Syntax Rules

```text
where [ FIELD ] < [ VALUE ]
```
Alternatively:
```text
where [ FIELD ] $lt [ VALUE ]
```

---

## Architecture: How it works under the hood

The architectural execution of `$lt` mirrors that of `$gt`, utilizing byte-level evaluation and B-Tree range logic.

### 1. Reverse Range Traversal
When executing an indexed `$lt` query, the internal cursor operates in reverse. It seeks the specific boundary node in the B-Tree index, and then uses a backward iterator to stream records downward until it reaches the absolute minimum node. This allows the cluaizdb engine to return the largest qualifying numbers first (if desired) without executing an expensive `ORDER BY` sort operation in memory.

### 2. Lexicographical Boundaries
While primarily used for numbers, the `$lt` operator is also valid for strings. The engine uses a highly optimized `memcmp` (memory compare) instruction to evaluate UTF-8 byte arrays, allowing queries like `where name < 'M'` to execute near the physical limits of the CPU cache.

---

## Time Complexity

| Scenario | Complexity | Notes |
| :--- | :--- | :--- |
| **Indexed Traversal** | **O(log N + K)** | Logarithmic seek time followed by linear extraction of `K` results. |
| **Unindexed Traversal** | **O(N)** | Requires evaluation of every record in the dataset. |

---

## Examples

### 1. Integer Thresholding

```json
{
  "cdql": "find json where retry_count < 3"
}
```

### 2. Lexicographical Sorting Constraint

Finding records where the primary key string falls below a certain alphabetical boundary.

```json
{
  "cdql": "find json where category $lt 'Software'"
}
```
