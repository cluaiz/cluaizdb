<p align="center">
  <img src="assets/banner.png" alt="cluaizdb Banner" width="100%">
</p>

<h1 align="center">
  <picture>
    <source srcset="https://fonts.gstatic.com/s/e/notoemoji/latest/1fabc/512.webp" type="image/webp">
    <img src="https://fonts.gstatic.com/s/e/notoemoji/latest/1fabc/512.gif" alt="🪼" width="32" height="32" style="vertical-align: middle;">
  </picture> 
  Cluaizdb
</h1>
<h3 align="center">Cluaiz Database</h3>
<p align="center"><strong>A High-Performance, Hardware-Native Multi-Model Database Engine</strong></p>

<p align="center"> 
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Built%20with-Rust-orange.svg" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-BSL%201.1-blue.svg" alt="License"></a>
  <a href="http://www.lmdb.tech/doc/"><img src="https://img.shields.io/badge/Core-LMDB-green.svg" alt="LMDB"></a>
  <a href="https://cluaiz.com"><img src="https://img.shields.io/badge/By-Cluaizdb-purple.svg" alt="Cluaizdb"></a>
</p>

---

**Cluaizdb** is a high-performance, hardware-native database engine built in **Rust** directly over **LMDB**. Instead of forcing you to choose between Graph, Document, Vector, or Relational architectures, Cluaizdb provides a zero-logic execution substrate that supports **30 different database paradigms** at runtime via **Dynamic DNA (WASM, Rhai, CDQL, and JSON rules)**.

---

## 🏛️ Deep Architectural Mechanics

### 1. Zero-Logic Core & The Memory-Mapped Router
The core Rust engine of Cluaizdb is intentionally policy-free. It views records simply as unified `UniversalNeuron` blocks containing:
- A raw, immutable/mutable byte payload (`Bytes`).
- A 16-dimensional hardware footprint vector (`[f32; 16]`).
- Adjacency vectors representing weighted graph edges (`NeuronEdge`).
- Dynamic DNA rulesets defining validation and lifecycle behaviors (`NeuronDna`).

Data logic, indexing configurations, and schema rules are completely isolated from the database engine and executed dynamically at the storage boundary.

```mermaid
graph TD
    Client["Client Request"] -->|"CDQL Query / Write"| Route["Router (crates/server/src/routes/write.rs)"]
    Route -->|"DNA Hooks Executed"| Executor["GenomeExecutor (crates/genome/src/executor.rs)"]
    Executor -->|"Lite Mode"| Transit["Transit Lounge (Lock-Free RAM Buffer)"]
    Executor -->|"Strict Mode"| Synchronous["Direct Write + WAL fsync"]
    Transit -->|"Asynchronous Flusher (50ms)"| LMDB[("LMDB Storage Engine")]
    Synchronous --> LMDB
```

---

### 2. High-Frequency Sensory Tissue Sharding
To handle massive telemetry throughput (256,000+ writes/second) without degrading database read latencies, Cluaizdb implements physical sharding:
- **Sensory Shards (`sensory_tissue.mdb`)**: Isolated, high-frequency, append-only structures designed to ingest fast temporal/sensor streams.
- **Cognitive Tissue Shards (`cognitive_tissue.mdb`)**: The main database environment for traversing rich graphs, executing vector similarity lookups, and evaluating document indexes.

---

### 3. Dynamic DNA Hook Engine
The `GenomeExecutor` exposes three core transactional hooks to execute logic directly before and during disk mutations:

*   **`on_write` (Validation & Durability Hook)**:
    *   Fires before any record commit.
    *   Determines if a write is allowed (`Allow`), deferred to the WAL only (`Defer`), or rejected (`Abort`).
    *   Controls physical durability via `sync_write: "strict"` (synchronous write + OS `fsync` call) vs `sync_write: "lite"` (flushed to the lock-free Transit Lounge, then written asynchronously every 50ms).
*   **`on_read` (Reinforcement Hook)**:
    *   Triggered on record retrieval.
    *   Mutates graph edge weights dynamically (e.g., reinforcing active paths by a percentage or decaying unused ones).
*   **`on_lifecycle` (Garbage Collection & Compaction Hook)**:
    *   Fires during the compactor/dreamer thread scan.
    *   Can trigger **Apoptosis** (self-destruction of the neuron), clear payloads to transition the neuron from `Hot` to `Warm` memory tiers, or apply custom edge decay.

---

### 4. Zero-Copy Serialization (FlatBuffers vs Protobuf)
When reading and validating incoming records, Cluaizdb supports **Zero-Copy FlatBuffers** integration:
- **No Companion Objects (CO)**: Traditional parsers read serialized payloads and reconstruct a secondary representation (like a JSON object or a Protobuf struct) in the heap. FlatBuffers reads values directly from offsets in the raw binary buffer.
- **WASM Memory Pointer Passing**: The database passes the raw memory pointer of the payload directly into the WebAssembly sandbox. The WASM module performs zero-copy offset reads to validate constraints, eliminating memory copies and heap allocations.

---

## 🧬 Multi-Paradigm Support (30 Database Engine Models)

By attaching different DNA scripts, **cluaizdb** natively supports the behavior of specialized databases within the same process. Detailed guides for all **30 paradigms** are located in the [docs/cluaizdb-types/](docs/cluaizdb-types/) directory:

| # | Paradigm | Link | # | Paradigm | Link |
|---|---|---|---|---|---|
| 1 | ⚡ Key-Value | [01-key-value.md](docs/cluaizdb-types/01-key-value.md) | 16 | 🏁 In-Memory | [16-in-memory.md](docs/cluaizdb-types/16-in-memory.md) |
| 2 | 🕸️ Graph | [02-graph.md](docs/cluaizdb-types/02-graph.md) | 17 | 🔗 Blockchain / Ledger | [17-ledger-blockchain.md](docs/cluaizdb-types/17-ledger-blockchain.md) |
| 3 | 📑 Document NoSQL | [03-document.md](docs/cluaizdb-types/03-document.md) | 18 | 🌍 Spatial Geographic | [18-spatial-geographic.md](docs/cluaizdb-types/18-spatial-geographic.md) |
| 4 | 🗄️ Relational | [04-relational.md](docs/cluaizdb-types/04-relational.md) | 19 | ⏱️ Event Sourcing | [19-event-sourcing.md](docs/cluaizdb-types/19-event-sourcing.md) |
| 5 | 🧠 Vector AI | [05-vector-ai.md](docs/cluaizdb-types/05-vector-ai.md) | 20 | 🤖 Autonomous | [20-autonomous.md](docs/cluaizdb-types/20-autonomous.md) |
| 6 | ⏱️ Time-Series | [06-time-series.md](docs/cluaizdb-types/06-time-series.md) | 21 | 📊 NewSQL | [21-newsql.md](docs/cluaizdb-types/21-newsql.md) |
| 7 | 🌍 Geo-Spatial | [07-geo-spatial.md](docs/cluaizdb-types/07-geo-spatial.md) | 22 | 📡 Streaming Reactive | [22-streaming-reactive.md](docs/cluaizdb-types/22-streaming-reactive.md) |
| 8 | 🏛️ Wide-Column | [08-wide-column.md](docs/cluaizdb-types/08-wide-column.md) | 23 | ⏱️ Temporal | [23-temporal.md](docs/cluaizdb-types/23-temporal.md) |
| 9 | 🔍 Full-Text Search | [09-full-text-search.md](docs/cluaizdb-types/09-full-text-search.md) | 24 | 🔲 Array / Tensor | [24-array-tensor.md](docs/cluaizdb-types/24-array-tensor.md) |
| 10| 📦 Blob / Object | [10-blob-object.md](docs/cluaizdb-types/10-blob-object.md) | 25 | 🌐 Federated / Virtual | [25-federated-virtual.md](docs/cluaizdb-types/25-federated-virtual.md) |
| 11| 🧩 Multi-Model | [11-multi-model.md](docs/cluaizdb-types/11-multi-model.md) | 26 | 📈 Multivalued Attributes | [26-multivalued.md](docs/cluaizdb-types/26-multivalued.md) |
| 12| 🌳 Hierarchical | [12-hierarchical.md](docs/cluaizdb-types/12-hierarchical.md) | 27 | 🎲 Native XML | [27-native-xml.md](docs/cluaizdb-types/27-native-xml.md) |
| 13| 🕸️ Network Model | [13-network.md](docs/cluaizdb-types/13-network.md) | 28 | 👥 Spatial Temporal | [28-spatial-temporal.md](docs/cluaizdb-types/28-spatial-temporal.md) |
| 14| 📦 Object-Oriented | [14-object-oriented.md](docs/cluaizdb-types/14-object-oriented.md) | 29 | 🕸️ Graph-Relational Hybrid | [29-graph-relational.md](docs/cluaizdb-types/29-graph-relational.md) |
| 15| 📊 Columnar | [15-columnar.md](docs/cluaizdb-types/15-columnar.md) | 30 | 🔌 Embedded In-Process | [30-embedded-in-process.md](docs/cluaizdb-types/30-embedded-in-process.md) |

---

## ⚡ CDQL: Cluaiz Database Query Language

**Cluaizdb** utilizes **CDQL**, a pipeline-based query language capable of executing multiple data paradigms sequentially within a single query:

```text
// Example: Find active users → traverse their friend graph → filter by location → semantic search
find User(status: "active")
  -> traverse(edge: "friends", hops: 1..3)
  -> geo_near(lat: 28.6, lon: 77.2, radius: "5km")
  -> search(query: "Pizza", fuzzy: true)
  -> limit 20
```

---

## 🏗️ 3-Tier Storage Architecture

The compactor daemon (Dreamer Engine) automatically transitions records through three memory states based on access frequency and global memory limits:

| Tier | Storage State | Storage Target | Typical Latency | Payload State |
| ---- | ------------- | -------------- | --------------- | ------------- |
| 1    | **Hot**       | LMDB `mmap` (RAM) | `< 1ms` | Uncompressed, ready for instant access |
| 2    | **Warm**      | LMDB (Disk-Backed) | `1-5ms` | Payloads are stripped, vectors and edges retained |
| 3    | **Cold**      | ZSTD compressed block | `50ms+` | Fully compressed, rehydrated on demand |

---

## 🚀 Getting Started

### Run the Server

```bash
git clone https://github.com/cluaiz/cluaizdb.git
cd cluaizdb

cargo run --release -p cluaizdb-server
# Server starts at http://localhost:7331
```

### Build the C-FFI Library (Robotics / Python / C++)

For zero-latency local deployments, build the native shared library:

```bash
cargo build --release -p cluaizdb-ffi
```

---

## 📚 Documentation Directory

Detailed implementation documents:

### Core Concepts & Guides
- 🌟 **[Why cluaizdb?](docs/vision/why-cluaizdb.md)** — Cost savings, paradigm comparison
- ⚡ **[Quickstart](docs/get-started/quickstart.md)** — Up and running in 60 seconds
- 🗺️ **[Rosetta Stone Cheatsheet](docs/cdql/rosetta-stone.md)** — Your DB's syntax → CDQL in 10 minutes
- 🧬 **[The 4 Engines](docs/genomes/dna-architecture.md)** — How WASM, Rhai, JSON, and CDQL DNA structure cluaizdb
- 🗂️ **[30 Database Paradigms](docs/cluaizdb-types/30-embedded-in-process.md)** — In-depth guide to all supported database engine types
- 📦 **[DNA Templates & Binary Serialization](docs/cluaizdb-dna-templates/README.md)** — Executable rulesets and Zero-Copy serialization formats (FlatBuffers, Protobuf)

### Deep Architecture
- 🧠 **[Dreamer Engine](docs/architecture/dreamer-engine.md)** — Background compaction & asynchronous analytics
- ⚡ **[FFI Bypass](docs/architecture/ffi-bypass.md)** — Achieving 0ms latency with C-FFI
- 🗄️ **[LMDB Zero-Copy](docs/architecture/lmdb-zero-copy.md)** — The foundational storage layer mechanics
- 🛡️ **[Sensory Shards](docs/architecture/sensory-shards.md)** — Handling 256k+ writes/sec cleanly

 

## Star History

<a href="https://www.star-history.com/?repos=Cluaiz%2Fcluaizdb&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=Cluaiz/cluaizdb&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=Cluaiz/cluaizdb&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=Cluaiz/cluaizdb&type=date&legend=top-left" />
 </picture>
</a>

## 📜 License & Usage

**cluaizdb** is released under a **BSL 1.1 / Elastic License Hybrid**.

<p align="center"><em>Built with ❤️ by <a href="https://cluaiz.com"><strong>Cluaiz Technologies</strong></a></em></p>
