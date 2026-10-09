# 🏛️ CluaizDB Kernel v2 — Production Architecture Tree Blueprint
> **Audited & Approved by Claude Opus 5.5 on Vertex AI**  
> **Status:** Gold Standard Architecture Target (`Khichdi-Zero` & `Ironclad DRY`)  
> **Root Workspace:** `c:\Users\Aryan\my\Cluaiz-workspace\Cluaiz-Technologies\cluaizdb`

---

## 🎯 1. Architectural Philosophy & Hard Invariants

CluaizDB Kernel v2 is engineered as a **Hardware-Native, Zero-Opinion Programmable Database Substrate**. It eliminates the legacy v1 debt (interpreted Rhai scripts, uncompiled JSON schemas, and monolithic `UniversalNeuron` bindings) and replaces it with strict domain separation.

### The 5 Ironclad Engineering Laws:
1. **Modular Sovereignty (Khichdi-Zero):** The core storage substrate NEVER touches network sockets, Axum HTTP handlers, WebSockets, or GUI frontends. Substrate is a pure, embeddable, single-writer multi-reader engine.
2. **Absolute DRY (Don't Repeat Yourself):** Shared traits (`StorageSubstrate`, `TransactionContext`, `VectorStore`) live in `api/` or `kernel-core/substrate`. No duplicate indexing or serialization logic is permitted.
3. **No Hardcoded Logic:** Banned from writing static paths or arbitrary fixed constants. Dynamic hardware probing and dependency injection are mandatory.
4. **Mandatory Documentation:** **Every single directory** in this workspace MUST contain its own `README.md` defining its responsibility, API boundaries, and invariants.
5. **Physical Durability & Recall Standards:**
   - In-Process Point Read: **`< 5 µs`** (Hardware DRAM/SSD cached page-cache).
   - Group Commit Durable Write: **`~121 µs / write`** (~8,200 durable writes/sec) backed by explicit `force_sync()` SSD barrier.

---

## 🌳 2. Master Workspace Directory Layout (`new-tree.md`)

```text
cluaizdb/
├── 📁 .github/                           # Continuous Integration & Delivery
│   ├── 📁 workflows/
│   │   └── ⚙️ ci.yml                     # Linux/Windows/macOS build, clippy, and case-sensitivity tests
│   └── 📝 README.md
│
├── 📁 api/                               # Protocol layer definitions & shared specifications
│   ├── 📁 mcp-spec/                      # Native Model Context Protocol (MCP) Server interfaces
│   │   ├── 📁 src/
│   │   │   ├── 🦀 lib.rs                 # Schema definitions (cluaiz_query_vector, cluaiz_inspect_nodes)
│   │   │   └── 🦀 handler.rs             # JSON-RPC Stdio dispatcher for Claude / LLM runtimes
│   │   ├── ⚙️ Cargo.toml
│   │   └── 📝 README.md
│   │
│   └── 📁 wasm-abi/                      # Host-Guest memory exchange definitions (Zero-Copy)
│       ├── 📁 src/
│       │   ├── 🦀 lib.rs                 # Linear memory layout, guest pointers & staging buffer specs
│       │   └── 🦀 types.rs               # RecordId (u128), SliceView, HookType definitions
│       ├── ⚙️ Cargo.toml
│       └── 📝 README.md
│
├── 📁 apps/                              # Presentation & client interfaces (NEVER touches storage internals)
│   ├── 📁 cli/                           # Developer administration CLI binary
│   │   ├── 📁 src/
│   │   │   ├── 🦀 main.rs                # Entry point
│   │   │   ├── 📁 commands/              # Subcommands: ingest, query, bench, inspect, backup
│   │   │   │   ├── 🦀 mod.rs
│   │   │   │   └── 📝 README.md
│   │   │   └── 📁 printer/               # Terminal formatting & ASCII tables
│   │   │       ├── 🦀 mod.rs
│   │   │       └── 📝 README.md
│   │   ├── ⚙️ Cargo.toml
│   │   └── 📝 README.md
│   └── 📁 server-http/                   # Network edge server (Axum REST & WebSockets)
│       ├── 📁 src/
│       │   ├── 🦀 main.rs                # Tokio async server runtime
│       │   ├── 📁 routes/                # Ingest, Query, Telemetry, and Health routes
│       │   │   ├── 🦀 mod.rs
│       │   │   └── 📝 README.md
│       │   └── 📁 state/                 # Shared AppState wrapping Engine Substrate
│       │       ├── 🦀 mod.rs
│       │       └── 📝 README.md
│       ├── ⚙️ Cargo.toml
│       └── 📝 README.md
│
├── 📁 kernel-core/                       # Core execution zone (Zero networking, pure compute & storage)
│   ├── 📁 ffi/                           # Dynamic C-linkage library (`cdylib` / `cluaizd.dll` / `cluaizd.so`)
│   │   ├── 📁 src/
│   │   │   └── 🦀 lib.rs                 # In-process FFI bridge (< 5 µs recall for Python / C++ / AI agents)
│   │   ├── ⚡ cluaizd.h                  # Auto-generated C header
│   │   ├── ⚙️ Cargo.toml
│   │   └── 📝 README.md
│   │
│   └── 📁 substrate/                     # Zero-Opinion B+Tree Substrate (6 Physical Tables)
│       ├── 📁 src/
│       │   ├── 🦀 lib.rs                 # `StorageSubstrate` trait & environment coordinator
│       │   ├── 🦀 db.rs                  # Physical LMDB/Heed environment initialization
│       │   ├── 🦀 schema.rs              # 6 Tables: records, indexes, outbound, inbound, genomes, stats
│       │   ├── 🦀 tx.rs                  # Atomic read/write transactions & staging buffers
│       │   ├── 🦀 index_layer.rs         # Non-overwriting composite index: `<col>:<term>:<id> -> ()`
│       │   └── 🦀 dual_graph.rs          # Bi-directional edges: outbound (`src|t|dst`) & inbound (`dst|t|src`)
│       ├── ⚙️ Cargo.toml
│       └── 📝 README.md
│
├── 📁 modules/                           # Standalone pure crates (Decoupled, reusable building blocks)
│   ├── 📁 group-commit/                  # Physical SSD fsync coordinator & batch committer
│   │   ├── 📁 src/
│   │   │   ├── 🦀 lib.rs                 # `GroupCommitter` channel pipeline & wait-free queue
│   │   │   └── 🦀 force_sync.rs          # Hardware-level disk barrier validation
│   │   ├── ⚙️ Cargo.toml
│   │   └── 📝 README.md
│   │
│   ├── 📁 index-engine/                  # Secondary indexing & vector algorithms
│   │   ├── 📁 mvhsnw-vector/             # Mutex-hardened HNSW vector index (Point reads < 4.24 µs)
│   │   │   ├── 📁 src/
│   │   │   │   ├── 🦀 lib.rs
│   │   │   │   └── 🦀 graph.rs           # Mutation-locked graph layers
│   │   │   ├── ⚙️ Cargo.toml
│   │   │   └── 📝 README.md
│   │   └── 📝 README.md
│   │
│   └── 📁 raft-engine/                   # Clustered consensus & state machine replication
│       ├── 📁 src/
│       │   └── 🦀 lib.rs                 # Raft log replication engine
│       ├── ⚙️ Cargo.toml
│       └── 📝 README.md
│
├── 📁 runtimes/                          # Sandboxed guest execution platforms
│   └── 📁 wasmtime-sandbox/              # Wasmtime runtime with fuel metering & memory sandboxing
│       ├── 📁 src/
│       │   ├── 🦀 lib.rs                 # Module cache, fuel limiter (50k instructions), instance pool
│       │   └── 🦀 staging.rs             # Host linear memory staging buffer (Zero LMDB RwTxn deadlocks)
│       ├── ⚙️ Cargo.toml
│       └── 📝 README.md
│
├── 📁 templates/                         # Guest Genomes written in safe Rust targeting WebAssembly
│   ├── 📁 guest/
│   │   ├── 📁 genome-vector/             # Vector space genome: cosine/l2 distance hooks & normalization
│   │   │   ├── 📁 src/
│   │   │   │   └── 🦀 lib.rs
│   │   │   ├── ⚙️ Cargo.toml             # Compiles to `wasm32-unknown-unknown`
│   │   │   └── 📝 README.md
│   │   │
│   │   ├── 📁 genome-graph/              # Semantic graph genome: decay weights & relation validation
│   │   │   ├── 📁 src/
│   │   │   │   └── 🦀 lib.rs
│   │   │   ├── ⚙️ Cargo.toml
│   │   │   └── 📝 README.md
│   │   │
│   │   └── 📁 genome-kv/                 # Zero-copy associative key-value schema validation
│   │       ├── 📁 src/
│   │       │   └── 🦀 lib.rs
│   │       ├── ⚙️ Cargo.toml
│   │       └── 📝 README.md
│   └── 📝 README.md
│
├── 📁 tests/                             # Workspace integration, chaos & durability suites
│   ├── 📁 api/                           # End-to-end REST & MCP protocol integration tests
│   ├── 📁 chaos/                         # Sudden crash / power-cut simulation (ACID durability assertions)
│   ├── 📁 benchmarks/                    # Micro-benchmarks for latency verification (4.24 µs point read)
│   └── 📝 README.md
│
├── ⚙️ Cargo.toml                         # Virtual workspace manifest linking all clean crates
└── 📝 README.md                          # Master project documentation & system architecture
```

---

## 🔄 3. Migration Truth Table (V1 Legacy vs. Kernel V2)

| Legacy Location (v1) | Target Kernel v2 Mapping | Action | Architectural Justification |
| :--- | :--- | :--- | :--- |
| `genomes/*.json` (30 uncompiled files) | `templates/guest/` | ❌ **PURGED** | Interpreting JSON strings on write paths was catastrophic for throughput. Replaced with compiled Rust WASM Genomes. |
| `active_dnas/` | `templates/guest/` | ❌ **PURGED** | Redundant legacy stubs. Superseded by typed WASM binaries cached in the Substrate's `genomes` table. |
| `components/cluaizdb-types/neuron.rs` | `kernel-core/substrate/` | ❌ **PURGED** | Monolithic `UniversalNeuron` eradicated. Replaced by raw byte entities (`record_id: u128 -> raw_bytes`). |
| `crates/storage/engine-lmdb/` | `kernel-core/substrate/` | 🔄 **MOVED & UPGRADED** | Refactored into clean 6-table schema (`records`, `indexes`, `outbound_edges`, `inbound_edges`, `genomes`, `stats`). |
| `crates/storage/engine-lmdb/src/group_commit.rs` | `modules/group-commit/` | 🔄 **MOVED** | Extracted from engine into a standalone, trait-driven module with explicit `force_sync()` SSD barrier. |
| `crates/storage/index-mvhsnw/` | `modules/index-engine/mvhsnw-vector/` | 🔄 **MOVED & SECURED** | Decoupled into pure algorithm crate. Mutation-locked with `parking_lot::Mutex<()>` and bounds checks. |
| `crates/server/` | `apps/server-http/` | 🔄 **MOVED** | Stripped of all direct storage access. Interacts with substrate strictly via async handles. |
| `apps/cli/` | `apps/cli/` | 🔄 **KEPT & RESTRUCTURED** | Modularized subcommands (`ingest`, `query`, `bench`) with clean separation from engine code. |
| `crates/consensus/raft-engine/` | `modules/raft-engine/` | 🔄 **MOVED** | Standardized as an isolated module for multi-node clustering. |
| *None (New)* | `api/mcp-spec/` | ✨ **NEW** | Native Model Context Protocol (MCP) server for zero-latency tool-calling by Claude on Vertex AI / Claude Desktop. |
| *None (New)* | `runtimes/wasmtime-sandbox/` | ✨ **NEW** | Deterministic Wasmtime engine with 50,000 instruction fuel limit and linear memory staging buffer. |

---

## 🛡️ 4. Directory Documentation Rule (Mandatory README Policy)

To eliminate ambiguity and prevent codebase rot, **every folder must contain a `README.md`** following this exact format:

```markdown
# [Module / Directory Name]

## 🎯 Purpose
Single-sentence explanation of what this module does.

## 🏛️ Domain Boundaries (What it CAN and CANNOT do)
- CAN: Process raw byte slices, execute fuel-metered WASM, etc.
- CANNOT: Open network sockets, mutate parent database transactions, etc.

## 📦 Exported Traits & APIs
- List of core traits and structs exposed to the workspace.

## ⚡ Performance Invariants
- Expected latency, memory consumption limits, and thread-safety requirements.
```

---

## 🚀 5. Immediate Execution Roadmap

1. **Step 1: Save `new-tree.md`** (Completed).
2. **Step 2: Workspace Manifest Realignment** (`Cargo.toml` update to include modular crate paths).
3. **Step 3: Core Substrate Scaffolding** (`kernel-core/substrate/src/lib.rs` and `db.rs` with the 6 physical LMDB tables).
4. **Step 4: Continuous Review Checkpoint** (Submit each module to Claude Opus 5.5 via `review.py` for automated verification).
