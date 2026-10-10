<p align="center">
  <img src="assets/banner.png" alt="CluaizDB Banner" width="100%">
</p>

<h1 align="center">
  <picture>
    <source srcset="https://fonts.gstatic.com/s/e/notoemoji/latest/1fabc/512.webp" type="image/webp">
    <img src="https://fonts.gstatic.com/s/e/notoemoji/latest/1fabc/512.gif" alt="🪼" width="32" height="32" style="vertical-align: middle;">
  </picture> 
  CluaizDB
</h1>
<h3 align="center">Microsecond Hardware-Native Programmable Database Substrate</h3>
<p align="center"><strong>Zero-Opinion LMDB Storage Substrate • Sovereign Modular Compute Engines • Native Model Context Protocol (MCP)</strong></p>

<p align="center"> 
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Built%20with-Rust-orange.svg" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-BSL%201.1-blue.svg" alt="License BSL 1.1"></a>
  <a href="http://www.lmdb.tech/doc/"><img src="https://img.shields.io/badge/Storage-LMDB%20B%2BTree-green.svg" alt="LMDB"></a>
  <a href="https://modelcontextprotocol.io/"><img src="https://img.shields.io/badge/Protocol-MCP%20Native-purple.svg" alt="MCP"></a>
  <img src="https://img.shields.io/badge/Release-0.1.0--Alpha-yellow.svg" alt="Release 0.1.0 Alpha">
</p>

---

## ⚡ What is CluaizDB?

**CluaizDB** is a microsecond hardware-native programmable database substrate written in **Rust**. Built directly over memory-mapped **LMDB**, CluaizDB eliminates the impedance mismatch of conventional multi-model databases by separating physical storage from domain execution.

The database engine is strictly **zero-opinion**: it does not force opinions on schema or serialization formats. Records are raw bytes stored in physical collections with composite B+Tree secondary indexing, while specialized compute modules (Vector ANN, Graph BFS/Dijkstra, Time-Series Gorilla compression, CDQL query planner, and Wasmtime sandboxing) operate as sovereign, hot-swappable building blocks.

---

## 📊 Measured Hardware Micro-Benchmarks

All numbers measured on physical bare-metal hardware via `cargo bench` (Criterion framework):

| Operation | Engine Layer | Latency / Throughput | Details |
| :--- | :--- | :--- | :--- |
| **Point Record Recall** | `kernel-core/substrate` | **615 ns** | Direct LMDB B+Tree zero-copy page cache read |
| **C-ABI FFI Recall** | `kernel-core/ffi` | **693 ns** | In-process C Dynamic Library (`cluaizdb.dll`) |
| **HNSW Vector KNN** | `modules/index-engine/mvhsnw-vector` | **75 µs** | 128-dim cosine distance nearest neighbor search |
| **Group Commit Durability** | `modules/group-commit` | **~121 µs / write** | Coordinated batch SSD `force_sync()` ACID barrier (8,200+ writes/s) |
| **Gorilla Float Compression**| `modules/time-series` | **< 1.8 µs / block** | Delta-of-Delta timestamp + XOR float compression |
| **Sandboxed WASM Exec** | `runtimes/wasmtime-sandbox` | **< 85 µs** | Fuel-metered WebAssembly linear memory staging buffer |

---

## 🏛️ Modular Sovereign Architecture

```
                                  ┌───────────────────────────┐
                                  │   Clients & AI Agents     │
                                  └─────────────┬─────────────┘
                                                │
                 ┌──────────────────────────────┼──────────────────────────────┐
                 │                              │                              │
        ┌────────▼─────────┐          ┌─────────▼─────────┐          ┌─────────▼─────────┐
        │  apps/server-http │          │     apps/cli      │          │  kernel-core/ffi  │
        │  REST & JSON-RPC  │          │  Terminal Binary  │          │  C-ABI DynamicLib │
        └────────┬─────────┘          └─────────┬─────────┘          └─────────┬─────────┘
                 │                              │                              │
                 └──────────────────────────────┼──────────────────────────────┘
                                                │
                                    ┌───────────▼───────────┐
                                    │      api/mcp-spec     │
                                    │ Model Context Protocol│
                                    └───────────┬───────────┘
                                                │
      ┌─────────────────────────────────────────┼─────────────────────────────────────────┐
      │                                         │                                         │
┌─────▼──────────────┐                ┌─────────▼──────────┐                   ┌──────────▼───────────┐
│ modules/cdql-query │                │modules/index-engine│                   │ runtimes/wasmtime-   │
│ AST Query Planner  │                │   mvhsnw-vector    │                   │       sandbox        │
└─────┬──────────────┘                └─────────┬──────────┘                   └──────────┬───────────┘
      │                                         │                                         │
      ├─────────────────────────────────────────┴─────────────────────────────────────────┤
      │                                                                                   │
┌─────▼──────────────┐                                                         ┌──────────▼───────────┐
│modules/time-series │                                                         │modules/group-commit  │
│Gorilla Compression │                                                         │ACID Durability Barrier│
└─────┬──────────────┘                                                         └──────────┬───────────┘
      │                                                                                   │
      └─────────────────────────────────────────┬─────────────────────────────────────────┘
                                                │
                                 ┌──────────────▼──────────────┐
                                 │    kernel-core/substrate    │
                                 │   Physical LMDB B+Tree      │
                                 │   (Records, Index, Graph)   │
                                 └──────────────┬──────────────┘
                                                │
                                       ┌────────▼────────┐
                                       │   NVMe / SSD    │
                                       └─────────────────┘
```

---

## 📦 Workspace Crates

| Crate | Path | Responsibility |
| :--- | :--- | :--- |
| **`cluaizdb-substrate`** | [`kernel-core/substrate`](kernel-core/substrate) | Physical LMDB storage substrate. Manages raw byte collections, composite B+Tree secondary indexing, and bidirectional Graph tables with native BFS & Dijkstra shortest path traversal. |
| **`cluaizdb-ffi`** | [`kernel-core/ffi`](kernel-core/ffi) | Ultra-low latency C-ABI dynamic library (`cluaizdb.h`, `cluaizdb.dll`) for embedding into C++, Python, Zig, or Robotics runtimes. |
| **`cluaizdb-group-commit`** | [`modules/group-commit`](modules/group-commit) | Hardware-level batch write coordinator that issues physical `force_sync()` barriers to guarantee ACID durability. |
| **`cluaizdb-index-mvhsnw`** | [`modules/index-engine/mvhsnw-vector`](modules/index-engine/mvhsnw-vector) | Thread-safe Hierarchical Navigable Small World (HNSW) graph for high-throughput Vector ANN search supporting Cosine and L2 metrics. |
| **`cluaizdb-cdql`** | [`modules/cdql-query`](modules/cdql-query) | Sovereign query engine featuring Tokenizer $\to$ Parser $\to$ AST $\to$ Planner $\to$ Evaluator with Full-Text search and Haversine Geo-spatial distance proximity filters. |
| **`cluaizdb-time-series`** | [`modules/time-series`](modules/time-series) | Facebook Gorilla time-series compression and decompression implementation (Delta-of-Delta timestamps + XOR float stream). |
| **`cluaizdb-wasm-sandbox`**| [`runtimes/wasmtime-sandbox`](runtimes/wasmtime-sandbox) | Deterministic, fuel-metered (50k instruction cap) WebAssembly execution sandbox with a linear memory staging buffer that prevents transaction lock deadlocks. |
| **`cluaizdb-mcp`** | [`api/mcp-spec`](api/mcp-spec) | Native Anthropic Model Context Protocol (MCP) server exposing database vector queries, graph inspection, and record retrieval to LLM agent workflows. |
| **`cluaizdb-server-http`** | [`apps/server-http`](apps/server-http) | Headless daemon exposing asynchronous REST and JSON-RPC 2.0 `/mcp` endpoints via Axum. |
| **`cluaizdb-cli`** | [`apps/cli`](apps/cli) | Unified administrative and interactive CLI binary. |
| **`cluaizdb-benchmarks`** | [`tests/benchmarks`](tests/benchmarks) | Criterion-based micro-benchmark suite measuring physical memory and SSD latencies. |

---

## 🚀 Quickstart Guide

### 1. Build the Workspace

```bash
# Clone the repository
git clone https://github.com/cluaiz/cluaizdb.git
cd cluaizdb

# Compile the entire modular workspace in release mode
cargo build --release
```

### 2. Run All Unit Tests

```bash
cargo test --workspace
```
*All 28 sovereign module tests pass in under 5 seconds with zero warnings.*

### 3. Using the CLI (`cluaizdb`)

```bash
# Store a record durably (Collection 1, ID "doc_101")
cargo run --release -p cluaizdb-cli -- put --col 1 --id "doc_101" --payload '{"title":"Hardware Native","author":"Cluaiz"}'

# Retrieve a record (< 5 µs recall)
cargo run --release -p cluaizdb-cli -- get --col 1 --id "doc_101"

# Index a secondary search term into B+Tree composite index
cargo run --release -p cluaizdb-cli -- index-term --col 1 --term "Hardware" --id "doc_101"

# Seek all record IDs matching a term
cargo run --release -p cluaizdb-cli -- seek-term --col 1 --term "Hardware"

# Start the sovereign server daemon on port 8080 (REST + MCP)
cargo run --release -p cluaizdb-cli -- serve --port 8080
```

### 4. Native Model Context Protocol (MCP) Integration

CluaizDB includes native MCP support for AI agent tools:

```bash
# Inspect all exposed MCP tools
cargo run --release -p cluaizdb-cli -- mcp-list

# Call a tool directly via JSON-RPC
cargo run --release -p cluaizdb-cli -- mcp-call --method "db_inspect_node" --params '{"collection_id": 1, "record_id": "doc_101"}'
```

---

## 🛠️ C-ABI Embedding (C / C++ / Python)

CluaizDB compiles directly to a shared library (`cluaizdb.dll` / `libcluaizdb.so`) for in-process memory access:

```c
#include "cluaizdb.h"
#include <stdio.h>

int main() {
    CluaizDbHandle* db = cluaizdb_open("./data", 1024 * 1024 * 1024);
    if (!db) {
        printf("Failed to open database\n");
        return 1;
    }

    const char* val = "{\"status\": \"active\"}";
    cluaizdb_put(db, 1, "agent_1", val, strlen(val));

    char buf[256];
    size_t len = cluaizdb_get(db, 1, "agent_1", buf, sizeof(buf));
    printf("Retrieved (%zu bytes): %.*s\n", len, (int)len, buf);

    cluaizdb_close(db);
    return 0;
}
```

---

## 📜 License & Terms

CluaizDB is distributed under the **[Business Source License 1.1 (BSL 1.1)](LICENSE)**:

- **Free Use:** Permitted for internal, research, personal, and general business operations.
- **Restrictions:** Cannot be used to provide a commercial managed database service (DBaaS), cloud hosting, or competing cloud products without a commercial license from Cluaiz Technologies.
- **Change Date:** Automatically converts to **Apache License 2.0** on **June 6, 2030**.

<p align="center"><em>Engineered with precision by <a href="https://cluaiz.com"><strong>Cluaiz Technologies</strong></a></em></p>
