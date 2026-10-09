# `find` Command Reference

The `find` command is the primary read-operation instruction in the cluaizdbd Query Language (CDQL). It is designed to traverse the database engine's storage layers and retrieve records.

## Syntax Rules

```text
find [ TARGET ] [ CONDITIONAL_CLAUSE ] [ MODIFIERS ]
```

### 1. Targets

The `TARGET` dictates *how* the engine deserializes and treats the binary data as it pulls it from the LMDB memory map.

| Target | Description | Underlying Execution |
| :--- | :--- | :--- |
| `*` | Raw Byte Extraction | Bypasses JSON parsing entirely. The engine reads the raw `[u8]` slice from the memory map and pipes it directly to the socket. This is the absolute fastest way to dump data. |
| `json` | Structured Deserialization | The engine reads the binary MsgPack/BSON format and casts it into a structured JSON tree in RAM before filtering. Required if you need to use `where` clauses on specific keys. |
| `similar` | Hardware SIMD Vector Search | Re-routes the query to the High-Dimensional Vector subsystem. The engine uses CPU SIMD registers (AVX2/AVX-512) to compute mathematical distances between floating-point arrays. |

## Time Complexity (Big-O)

The time complexity of a `find` traversal depends heavily on the presence of B-Tree indices and the chosen target.

| Operation | Syntax | Complexity | Notes |
| :--- | :--- | :--- | :--- |
| **Global Scan** | `find *` | **O(N)** | Sequentially reads all records. Highly efficient due to sequential memory access, but scales linearly with dataset size. |
| **Filtered Scan** | `find json where...` | **O(N)** | Requires parsing fields. Use indexing (if configured) to reduce this to O(log N). |
| **Vector Search** | `find similar...` | **O(N * d)** | Where `d` is the vector dimensionality. Optimized via SIMD instructions. |

---

## Examples

### 1. Raw Binary Retrieval (Global Scan)

```text
find *
```

### 2. Structured JSON Retrieval

```text
find json
```

### 3. Vector Similarity Search

*(Note: Requires the `using` modifier to specify the distance algorithm)*

```text
find similar using cosine_distance
```
