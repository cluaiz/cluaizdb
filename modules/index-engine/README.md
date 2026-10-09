# 🔍 modules/index-engine: Secondary Indexing & Vector Search
> **Layer:** Secondary Abstract Algorithm Engines  
> **Sub-Modules:**  
> - `mvhsnw-vector/`: Mutex-hardened Hierarchical Navigable Small World (HNSW) ANN search

---

## 🎯 Purpose
`modules/index-engine` houses vector and multidimensional indexing structures decoupled from physical storage layouts. It operates over in-memory or memory-mapped candidate arrays, delivering sub-millisecond approximate nearest neighbor (ANN) recall for AI embeddings.

---

## 🏛️ Domain Boundaries
- **CAN:** Calculate Euclidean (L2) and Cosine distances, maintain graph adjacency lists, execute multi-layer greedy searches.
- **CANNOT:** Perform direct disk transactions, open network connections, or depend on monolithic record types.
