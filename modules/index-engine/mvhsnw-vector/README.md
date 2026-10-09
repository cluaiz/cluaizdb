# 📐 modules/index-engine/mvhsnw-vector: Hardened HNSW Vector Index
> **Algorithm:** Hierarchical Navigable Small World (HNSW) Approximate Nearest Neighbor (ANN)  
> **Concurrency Invariant:** Mutex-Locked Mutations (`parking_lot::Mutex<()>`) + Fine-Grained `RwLock` Node Read Traversal  
> **Decoupled Key:** `u128` (UUIDv7 / RecordId)

---

## 🎯 1. Purpose
`mvhsnw-vector` provides lightning-fast Approximate Nearest Neighbor (ANN) search over floating-point dense vectors (e.g. 384-d, 768-d, 1536-d embeddings). It enables AI agents to query episodic memory within microseconds.

---

## 🏛️ 2. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- Insert dense float vectors associated with abstract `u128` record IDs.
- Execute top-$k$ $K$-Nearest Neighbor (KNN) searches using Cosine or Euclidean metrics.
- Enforce strict thread-safety during concurrent ingestion and read traversals.

### ❌ What this crate CANNOT do:
- Depend on `cluaizdb-types` or `UniversalNeuron`.
- Execute file IO or write to LMDB (physical persistence is managed by host coordinates).

---

## ⚡ 3. Performance & Concurrency Invariants
- **Recall Target:** Sub-millisecond ANN search (< 50 µs for $k=10$ on 10,000 vectors).
- **Race Condition Prevention:** Top-level structural changes are gated by a dedicated mutation lock, while node-level neighbor lists are guarded by fine-grained `parking_lot::RwLock` instances.
