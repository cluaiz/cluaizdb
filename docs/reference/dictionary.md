# 📖 Cluaizdb Master Technical Dictionary (A-Z)

Welcome to the official **Cluaizdb Master Glossary and Systems Dictionary**. This document outlines every key technical term, architectural component, database paradigm keyword, and mathematical concept utilized across Cluaizdb's codebase, query engines, and documentation.

---

## 🅰️ A

### Apoptosis
*   **Definition**: The programmatic process of self-destruction triggered on a database node (`UniversalNeuron`).
*   **Derivation**: Derived from the biological term *apoptosis* (programmed cell death).
*   **Systems Implementation**: Triggered by returning `{"delete_neuron": true}` inside an `on_lifecycle` hook evaluated by the Dreamer compactor daemon. The engine issues an immediate tombstone mutation and physical LMDB deletion to prevent memory bloat.

### AVX2 / AVX-512 (Advanced Vector Extensions)
*   **Definition**: Intel/AMD hardware-level SIMD instruction set extensions designed for executing high-dimensional parallel mathematical operations.
*   **Systems Implementation**: Cluaizdb's core SIMD vector engine binds these hardware registers to perform fast Euclidean and Cosine distance operations on the `[f32; 16]` neuron spatial footprint.

---

## 🅱️ B

### B-Tree (Balanced Tree)
*   **Definition**: A self-balancing search tree data structure that maintains sorted data and allows searches, sequential access, insertions, and deletions in logarithmic time.
*   **Systems Implementation**: The foundational data structure used by LMDB to map physical page offsets to memory addresses, facilitating $O(\log N)$ lookup performance.

### BM25 (Best Matching 25)
*   **Definition**: A probabilistic ranking function used by search engines to estimate the relevance of documents to a given search query.
*   **Systems Implementation**: Triggered when using the `search` operator in CDQL on a `text` payload type. The engine computes term-frequency/inverse-document-frequency values to score and rank relevant database nodes.

### BSL 1.1 (Business Source License)
*   **Definition**: A source-available licensing agreement that transitions to open-source (e.g., GPL/Apache) after a specified time duration.
*   **Systems Implementation**: Governs Cluaizdb's commercial usage restrictions, allowing free deployment for edge robotics, research, and non-managed deployments.

---

## 🅲 C

### CDQL (Cluaiz Database Query Language)
*   **Definition**: A pipeline-based, multi-model query language designed to execute key-value, graph, document, and vector operations sequentially in a single execution pipeline.
*   **Systems Implementation**: Evaluated by the CDQL Planner (`crates/server/src/routes/query.rs`). The parser resolves queries like `find User -> traverse -> similar` into optimal hardware routing tasks.

### C-FFI (C Foreign Function Interface)
*   **Definition**: A foreign language boundary allowing programs written in C/C++, Python, or Go to call Rust binary functions directly without IPC overhead.
*   **Systems Implementation**: Implemented inside `crates/cluaizdb-ffi`. Bypasses TCP stack latency to deliver 0ms database ingestion speeds directly inside in-process memory runtimes.

### Cognitive Tissue
*   **Definition**: The primary database sharding partition (`cognitive_tissue.mdb`) in Cluaizdb.
*   **Systems Implementation**: Stores the core graph relationships, long-term index pointers, vector spatial structures, and heavy document payloads. Designed for highly structured query access.

### CRDT (Conflict-Free Replicated Data Type)
*   **Definition**: A mathematical data structure that can be replicated across multiple nodes in a network, where replicas can be updated independently and concurrently without coordination, guaranteeing eventual consistency.
*   **Systems Implementation**: Covered in Cluaizdb paradigm guides [22-crdt.md](../cluaizdb-types/22-crdt.md) and [28-crdt-collaborative.md](../cluaizdb-types/28-crdt-collaborative.md) to manage concurrent edge updates.

### CRISPR (Cluaiz Relational Indexing & Surgical Pointer Routing)
*   **Definition**: An administrative system API enabling direct manual mutations of raw B-Tree keys and physical pointer arrays.
*   **Derivation**: Inspired by the biological gene-editing system *CRISPR-Cas9*.
*   **Systems Implementation**: Exposed via the `/crispr` API endpoint. Bypasses the DNA engine hooks to programmatically force-inject or sever network graph edges between neurons.

---

## 🅳 D

### DNA (Dynamic Neuron Affordance / Genomes)
*   **Definition**: The runtime ruleset containing validation, reinforcement, and lifecycle logic attached directly to individual database records.
*   **Systems Implementation**: Stored inside `UniversalNeuron.dna`. Executed at the storage boundary using WASM, Rhai, JSON, or CDQL scripts to dynamically control database ingestion, read verification, and garbage collection.

### Dreamer Engine
*   **Definition**: The asynchronous background compactor daemon thread in Cluaizdb.
*   **Systems Implementation**: Scans the physical database files, monitors RAM pressure, executes `on_lifecycle` DNA hooks, transitions records between memory tiers, and applies ZSTD Level 9 compression to cold blocks.

### Durability
*   **Definition**: The transactional guarantee that committed data will survive system power loss or operating system failures.
*   **Systems Implementation**: Configurable in DNA scripts. Supports `sync_write: "strict"` (blocking `fsync` system calls) and `sync_write: "lite"` (asynchronous WAL flushing through the Transit Lounge buffer).

---

## 🅴 E

### Elastic License
*   **Definition**: A non-copyleft license that permits free usage, modification, and integration, but blocks using the software to provide a managed cloud database service.
*   **Systems Implementation**: Hybridized with the BSL 1.1 to safeguard Cluaizdb cloud deployments.

---

## 🅵 F

### FlatBuffers
*   **Definition**: An efficient, zero-copy binary serialization format designed for reading data directly without deserialization allocations.
*   **Systems Implementation**: Enabled via the `payload_format = "flatbuffers"` config. Allows WASM and Rust runtimes to read payload attributes from memory offsets without allocating in-memory companion structures.

### fsync (File Sync)
*   **Definition**: An operating system call that forces the kernel to flush dirty page cache frames directly to non-volatile physical storage blocks.
*   **Systems Implementation**: Called synchronously when writing a neuron with `Durability::Strict` to guarantee data permanence on SSD hardware.

---

## 🅶 G

### GenomeExecutor
*   **Definition**: The unified runtime executor managing the sandbox environments for executing DNA.
*   **Systems Implementation**: Implemented inside `crates/genome/src/executor.rs`. Evaluates Rhai, WASM, and JSON scripts during database write, read, and lifecycle transactions.

---

## 🅷 H

### Hot Storage Tier
*   **Definition**: The highest-performance database state where records are mapped directly in RAM.
*   **Systems Implementation**: The neuron's payload, spatial vector, and graph edges are kept uncompressed inside LMDB's page table map, facilitating sub-millisecond retrieval.

---

## 🅸 I

### IMDB (In-Memory Database)
*   **Definition**: A database engine that relies primarily on main memory (RAM) for computer data storage.
*   **Systems Implementation**: Supported natively in Cluaizdb via memory-mapped environments, allowing virtual memory mappings to act as primary memory caches.

---

## 🅹 J

### JSON (JavaScript Object Notation)
*   **Definition**: A lightweight data-interchange format structured as text.
*   **Systems Implementation**: Transpiled internally by Cluaizdb into a binary-packedMsgPack-style byte stream to support zero-allocation key lookups inside where filters.

### JUJU
*   **Definition**: The official real-time, interactive 3D spatial visualization canvas for Cluaizdb.
*   **Systems Implementation**: Renders the dynamic topological node relationships, vector clusters, and edge activations directly inside a web-interface via WebGL/WebSockets.

---

## 🅻 L

### LMDB (Lightning Memory-Mapped Database)
*   **Definition**: An ultra-fast, ultra-compact transactional key-value store using a B-Tree structure, mapping files directly into virtual memory.
*   **Systems Implementation**: The foundational storage library underneath Cluaizdb (`engine-lmdb`). It executes zero-copy disk reads by returning pointers directly to the OS page cache.

---

## 🅼 M

### MDB_val
*   **Definition**: The basic binary structure used by LMDB to exchange keys and values.
*   **Systems Implementation**: Defined in C as a struct containing `size_t mv_size` and a pointer `void *mv_data`. Cluaizdb casts these pointers directly to Rust structs (`UniversalNeuron`) to avoid memory copies.

### MVCC (Multi-Version Concurrency Control)
*   **Definition**: A database concurrency protocol that allows readers to access stable historical states of data while writers append mutations concurrently, avoiding reader-writer blocking.
*   **Systems Implementation**: Inherited from LMDB's single-writer, multi-reader architecture. Ensures graph traversals are never blocked by high-frequency ingestion streams.

---

## 🅽 N

### Neuron (UniversalNeuron)
*   **Definition**: The fundamental database record unit inside Cluaizdb.
*   **Systems Implementation**: Implemented inside `components/cluaizdb-types`. Houses payload bytes, vector arrays, adjacency pointers, and rulesets inside a unified memory block.

### NeuronEdge
*   **Definition**: The structural representation of a directed, weighted link between two database nodes.
*   **Systems Implementation**: Contains the destination UUID, a `f32` weight, and an access timestamp, enabling native graph capabilities.

---

## 🅾️ O

### OT (Operational Transformation)
*   **Definition**: A concurrency control technology used to support real-time, multi-user collaborative editing.
*   **Systems Implementation**: Outlined in the operational transformation paradigm guide [23-operational-transformation.md](../cluaizdb-types/23-operational-transformation.md) to manage collaborative document nodes.

---

## 🅿️ P

### PayloadType
*   **Definition**: The type classification tag assigned to incoming payloads during ingestion.
*   **Systems Implementation**: Supports `Text`, `Audio`, `Video`, `Code`, `VoltageStream`, and `Binary` (default), directing the database engine to apply relevant index, search, and compression pipelines.

### Protobuf (Protocol Buffers)
*   **Definition**: A language-neutral, platform-neutral binary serialization format developed by Google.
*   **Systems Implementation**: Supported via the `payload_format = "protobuf"` config. Requires deserialization into an intermediate companion object before field validation can proceed.

---

## 🆁 R

### Rhai
*   **Definition**: An embedded scripting language engine designed for Rust applications.
*   **Systems Implementation**: Employed as one of the 4 core DNA engines, registering custom helper functions (like `cosine_similarity`) to evaluate logic at the database layer.

---

## 🆂 S

### Sensory Tissue
*   **Definition**: A dedicated, isolated write-optimized database shard partition (`sensory_tissue.mdb`) in Cluaizdb.
*   **Systems Implementation**: Bypasses heavy B-Tree sorting and index logic to quickly ingest raw, high-frequency IoT telemetry and sensor logs.

### SIMD (Single Instruction, Multiple Data)
*   **Definition**: A parallel execution model allowing a CPU to perform the same operation on multiple data elements in a single CPU instruction.
*   **Systems Implementation**: Applied in Cluaizdb's distance functions to evaluate vector proximity using AVX hardware registers.

### Specular Graph
*   **Definition**: The fast-path graph engine in Cluaizdb.
*   **Systems Implementation**: Leverages bitwise intersections and index-free adjacencies to navigate multi-hop traversals at microsecond speeds.

### StorageTier
*   **Definition**: The three physical states of data residency managed by the Dreamer compactor daemon.
*   **Systems Implementation**: Supports **Hot** (in RAM, uncompressed), **Warm** (payload stripped, graph edges kept), and **Cold** (payload + edges compressed with ZSTD on disk).

---

## 🆃 T

### Transit Lounge
*   **Definition**: A lock-free, concurrent ring buffer cache residing in system RAM.
*   **Systems Implementation**: Intercepts records marked for `Durability::Lite`. Buffers writes in memory and flushes them to the WAL + LMDB in batches every 50ms to protect storage devices from excessive write-wear.

---

## 🆆 W

### WAL (Write-Ahead Log)
*   **Definition**: An append-only log file on non-volatile disk storage where all transactional mutations are recorded prior to applying them to the database.
*   **Systems Implementation**: Secures crash consistency. In the event of power loss, Cluaizdb replays the log sequentially during boot to restore system state.

### WASM (WebAssembly)
*   **Definition**: A binary instruction format for a stack-based virtual machine, executing code at near-native performance.
*   **Systems Implementation**: The primary engine for strict-typed DNA validation rulesets. Executes pre-compiled binaries inside an isolated sandbox boundary.

---

## 🆋 Z

### ZSTD (Zstandard)
*   **Definition**: A fast, real-time compression algorithm offering high compression ratios.
*   **Systems Implementation**: Invoked by the Dreamer compactor daemon using Level 9 tuning to compress cold blocks prior to archive writes.
