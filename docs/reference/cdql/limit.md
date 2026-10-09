# `limit` Command Reference

The `limit` modifier truncates the result set returned by the engine. It is a critical safeguard against memory exhaustion and network congestion when querying large shards.

## Syntax Rules

```text
<COMMAND> <TARGET> [ FILTERS ] limit <INTEGER>
```

---

## Architecture: How it works under the hood

The cluaizdb engine handles limits at the iterator level, avoiding unnecessary data loading.

### 1. Lazy Evaluation
When a query includes a `limit` (e.g., `limit 10`), the internal storage iterator does not scan the entire database and then slice the array. Instead, it processes records one by one. As soon as the 10th record matching the `where` filter is found, the engine immediately aborts the traversal loop.

### 2. Network Backpressure
By aborting the traversal early, the engine prevents backpressure on the internal TCP socket buffer. Without a limit, a `find *` query on a 10GB database would attempt to stream 10GB of data, potentially causing OS-level socket exhaustion.

---

## Time Complexity

| Scenario                     | Complexity             | Notes                                                                                                                                                      |
| :--------------------------- | :--------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Limit with Vector Search** | **O(N * d + L log L)** | AI Vector searches must evaluate all distances before sorting to find the Top-K (`L`). Thus, `limit` does not prevent a full scan for similarity searches. |
| **Limit with Filters**       | **O(K)**               | Where `K` is the number of records scanned before finding `L` matches. In the best case, it halts almost instantly.                                        |

---

## Examples

### 1. Hard Capping a Global Scan

```json
{
  "cdql": "find * limit 100"
}
```

### 2. Top-K Vector Results

Returning the 3 most semantically similar records.

```json
{
  "cdql": "find similar using cosine_distance limit 3",
  "query_vector": [0.1, 0.9, -0.4, 0.2]
}
```
