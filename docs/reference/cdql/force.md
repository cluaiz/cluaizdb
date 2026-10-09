# `force` Command Reference

The `force` parameter is an action directive used within the `/crispr` API subsystem. It instructs the cluaizdb engine to explicitly write a directed graph edge between two independent records, bypassing any background execution scripts or automated routing.

## Syntax Rules

```text
POST /crispr/force/[ ORIGIN_UUID ]
```

### JSON Body Parameters

| Field       | Type     | Description                                                                                             |
| :---------- | :------- | :------------------------------------------------------------------------------------------------------ |
| `target_id` | `string` | The UUID of the destination record.                                                                     |
| `weight`    | `float`  | A numerical value (typically between -1.0 and 1.0) defining the strength or priority of the connection. |

---

## Architecture: How it works under the hood

### 1. The Adjacency Matrix Append
Graph relationships in cluaizdb are not stored in a separate relational table. Instead, every record contains an embedded Adjacency Array. When `force` is called, the engine opens a write transaction, expands the Origin record's allocated block by a few bytes, and appends the `target_id` and `weight`.

### 2. Write-Ahead Log (WAL) Durability
Even though edge creation modifies an existing record, the entire modification is appended to the WAL to guarantee crash-resilience. The WAL ensures that if the server loses power exactly when the edge is formed, the relationship is fully restored upon reboot.

---

## Time Complexity

| Operation               | Complexity   | Notes                                                                                       |
| :---------------------- | :----------- | :------------------------------------------------------------------------------------------ |
| **Graph Pointer Write** | **O(log N)** | The system must query the B-Tree index to verify both UUIDs exist before securing the edge. |

---

## Examples

### 1. Standard Edge Forcing

```bash
curl -X POST http://localhost:7331/crispr/force/123e4567-e89b-12d3-a456-426614174000 \
-H "Content-Type: application/json" \
-d '{
  "target_id": "123e4567-e89b-12d3-a456-426614174001",
  "weight": 0.85
}'
```
