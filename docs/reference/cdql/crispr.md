# `crispr` API Reference

In the cluaizdb architecture, graph relations and dynamic execution logic are not defined via static tables. Instead, they are mutated through the `/crispr` API route. This endpoint allows developers to manually construct or sever direct memory pointers between records.

## Syntax & Routing

```text
POST /crispr/[ ACTION ]/[ ORIGIN_UUID ]
```

### Supported Actions
- `force` : Creates a directed, weighted edge from the origin to a target.
- `sever` : Destroys an existing edge between the origin and a target.

---

## Architecture: How it works under the hood

The `crispr` API bypasses standard REST payloads to interact directly with the database's Graph Subsystem.

### 1. Pointer Mapping
When a `/crispr/force` command is issued, the engine does not duplicate the target data. Instead, it computes the memory offset of the Target UUID within the LMDB environment and writes a 64-bit integer pointer into the Origin record's adjacency list. 

### 2. Lock-Free Traversal
Because the edges are stored as direct byte-offsets, graph traversal queries (e.g., `GET /graph/{uuid}/traverse`) do not require B-Tree lookups for subsequent hops. The engine simply dereferences the pointer and jumps instantly to the next memory page, yielding sub-microsecond traversal speeds.

---

## Time Complexity

| Operation                 | Complexity   | Notes                                                                                            |
| :------------------------ | :----------- | :----------------------------------------------------------------------------------------------- |
| **Edge Creation (Force)** | **O(log N)** | Requires B-Tree lookups to validate both the Origin and Target UUIDs before writing the pointer. |
| **Edge Traversal**        | **O(1)**     | Traversal per hop is constant time due to direct pointer dereferencing.                          |

---

## Examples

### 1. Constructing a Graph Edge

Creating a directed relationship from User A to User B.

```bash
curl -X POST http://localhost:7331/crispr/force/USER-A-UUID \
-H "Content-Type: application/json" \
-d '{
  "target_id": "USER-B-UUID",
  "weight": 1.0
}'
```
