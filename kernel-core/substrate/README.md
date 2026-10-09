# 🧱 kernel-core/substrate: The Zero-Opinion Storage Substrate
> **Engine:** CluaizDB Kernel v2 Physical Storage Engine  
> **Underlying Storage:** Memory-Mapped Transactional B+Tree (LMDB via Heed)  
> **Latency Target:** `< 5 µs` In-Process Point Recall

---

## 🎯 1. Purpose
`kernel-core/substrate` is the beating heart of CluaizDB Kernel v2. It is a **Zero-Opinion, Hardware-Native Key-Value Substrate** that manages 6 physical database tables. It stores raw byte payloads and maintains secondary composite index keys without assuming any knowledge of AI vectors, graphs, or business schemas.

---

## 🏛️ 2. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- Open, configure, and manage memory-mapped Heed/LMDB environments.
- Provide ACID multi-table read and write transactions.
- Perform $O(\log N)$ point lookups, composite prefix scans, and reciprocal graph edge traversals.
- Deliver zero-copy byte slice references (`&[u8]`) directly to callers.

### ❌ What this crate CANNOT do:
- Open TCP/HTTP/WebSocket sockets or touch Axum server logic.
- Execute WebAssembly code (WASM execution is strictly delegated to `runtimes/wasmtime-sandbox`).
- Perform heavy vector similarity search algorithms (delegated to `modules/index-engine`).
- Touch any UI or Presentation code.

---

## 📦 3. The 6 Physical Database Tables

| Table Name | Key Schema | Value Schema | Lookup Complexity | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| **`records`** | `(collection_id: u32, record_id: u128)` | `raw_bytes: &[u8]` | $O(\log N)$ | Primary raw entity storage. Zero serialization tax. |
| **`indexes`** | `(col_id: u32, term: &[u8], record_id: u128)` | `()` (empty) | $O(\log N)$ Range Seek | Composite non-overwriting B+Tree secondary index. |
| **`outbound_edges`**| `(src_id: u128, type_id: u32, dst_id: u128)` | `EdgeMetadata` (weight, timestamp) | $O(\log N)$ | Directed relationship graph traversal. |
| **`inbound_edges`** | `(dst_id: u128, type_id: u32, src_id: u128)` | `()` (empty) | $O(\log N)$ | Reverse relationship lookup (zero table scans). |
| **`genomes`** | `wasm_id: u64` | `wasm_bytecode: &[u8]` | $O(\log N)$ | Pre-compiled sandboxed WASM binaries. |
| **`stats`** | `record_id: u128` | `{ access_count, importance }` | $O(\log N)$ | Telemetry and decay metadata (batched in DRAM). |

---

## ⚡ 4. Exported Traits & APIs
- **`trait StorageSubstrate`**: The core abstract interface for getting, putting, deleting, and scanning records.
- **`struct SubstrateEnv`**: Thread-safe manager wrapping the physical Heed environment.
- **`struct ReadTransaction<'a>`**: Zero-copy read transaction providing microsecond recall.
- **`struct WriteTransaction<'a>`**: Atomic multi-table write transaction.

---

## 🛡️ 5. Performance & Safety Invariants
1. **Reads Never Write:** Read transactions NEVER mutate disk state or trigger write locks.
2. **Zero Deadlocks:** Transactions are strictly single-writer multi-reader (SWMR).
3. **Physical Durability:** Backed by explicit OS page-cache flushes and SSD barriers coordinated via `modules/group-commit`.
