# ⚡ modules/group-commit: Hardware-Native Batch Committer
> **Module:** CluaizDB Kernel v2 ACID Group Committer  
> **Underlying Guarantee:** Single Atomic LMDB Write Transaction + Physical `force_sync()` SSD Barrier  
> **Throughput Invariant:** ~121 µs / write (~8,200 durable writes/sec) under high concurrency

---

## 🎯 1. Purpose
`modules/group-commit` eliminates disk IO bottlenecks by coalescing concurrent write requests into unified atomic batches. Instead of forcing each client thread to issue a blocking physical `fsync` (which caps performance at ~100–175 writes/sec on typical SSDs), the Group Committer batches up to 128 operations together while preserving strict ACID durability: an acknowledgment is sent **only after** physical disk sync completes.

---

## 🏛️ 2. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- Accept asynchronous write operations through non-blocking MPSC channels.
- Drain up to `max_batch_size` items within a `max_latency` timeout window.
- Execute atomic multi-record mutations in `kernel-core/substrate`.
- Issue physical SSD sync barriers (`force_sync()`).
- Notify individual caller threads via zero-allocation oneshot channels.

### ❌ What this crate CANNOT do:
- Parse high-level query languages or JSON schemas.
- Open network sockets or run HTTP servers.
- Touch UI or presentation layers.

---

## 📦 3. Exported Types & APIs
- **`struct GroupCommitter`**: Thread-safe, clonable handle for submitting write operations.
- **`struct GroupCommitConfig`**: Tunable batch sizes, queue capacities, and latency deadlines.
- **`struct WriteOperation`**: Decoupled raw payload mutation `(col_id: u32, record_id: u128, payload: Vec<u8>)`.

---

## 🛡️ 4. Durability & Safety Invariants
1. **Zero Fake Acknowledgments:** Callers NEVER receive `Ok(())` while data is still in RAM page-cache.
2. **Crash Resilience:** If power cuts or process dies, all acknowledged writes are physically safe on disk.
3. **Decoupled from Models:** Zero dependence on monolithic legacy types (`UniversalNeuron`). Pure raw byte operations.
