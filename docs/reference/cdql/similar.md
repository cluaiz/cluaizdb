# `similar_to` Command Reference

The `similar_to` keyword switches the Cluaizdb query executor from standard exact-match logic (JSON/Text) to High-Dimensional Vector Similarity Search. It is designed specifically for AI workflows, RAG (Retrieval-Augmented Generation), and semantic search.

## Syntax Rules

```text
-> similar_to(vector: [<float>, ...], metric: "<ALGORITHM>")
```

### Supported Algorithms
- `"cosine"` : Best for normalized vectors (e.g., OpenAI embeddings).
- `"l2"` : Best for spatial data and non-normalized vectors (Euclidean distance).
- `"dot"` : Best for recommendation systems.

---

## Architecture: How it works under the hood

In Phase 5, Cluaizdb upgraded from flat O(N) scans to a **Multi-Version Hierarchical Navigable Small World (MV-HNSW)** index.

### 1. The MV-HNSW Graph
Instead of scanning every record, `similar_to` navigates a multi-layered proximity graph in RAM. The search drops through sparse top layers to quickly zoom in on the target region, then traverses the dense bottom layer to find the exact nearest neighbors.

### 2. SIMD Acceleration
Distance metrics (Cosine, L2, Dot) between nodes during the graph traversal are computed using hardware-level SIMD (Single Instruction, Multiple Data) intrinsics for extreme microsecond throughput.

---

## Time Complexity

| Operation                  | Complexity             | Notes                                                                                                                                                                          |
| :------------------------- | :--------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **MV-HNSW Search** | **O(log N)** | The search is sub-linear. It executes in `<1ms` regardless of whether the shard contains 10,000 or 10,000,000 embeddings. |

---

## Examples

### 1. Semantic Search using Cosine Distance

This is the standard approach when using embeddings from LLM providers like OpenAI.

```text
find Product(category: "shoes")
  -> similar_to(vector: [0.015, -0.022, 0.991, ...], metric: "cosine")
  -> limit 5
```
