# 📖 Introduction & System Mental Model

> **Component:** `docs/get-started`  
> **Architecture:** 4-Layer Cognitive Database Subsystem  
> **Philosophy:** Embedded In-Process Multi-Model Engine for AI Agents

---

## 🧭 The 4 Core Architectural Layers

CluaizDB is designed around a clean separation of concerns, structured into 4 cohesive layers:

```mermaid
flowchart TB
    subgraph LAYER1["Layer 1: Access & Integration"]
        RUST["Rust API<br/>Direct crate embedding"]
        FFI["C-ABI (cluaizdb.h)<br/>C, C++, Python, Go"]
        HTTP["Axum HTTP Daemon<br/>REST &amp; JSON-RPC"]
        MCP["Model Context Protocol<br/>Claude Desktop / Cursor / Antigravity"]
    end

    subgraph LAYER2["Layer 2: Cognitive Governance"]
        SIGNAL["Task Signals &amp; Sensory Intake"]
        GENOME["Agent Genomes &amp; DNA Modules"]
        INTERCEPTOR["Action Interceptor &amp; Policy Engine"]
        TOPOLOGY["Taxonomic Topology &amp; Concept Clusters"]
        ASYNC_LOG["Async Experience Queue"]
    end

    subgraph LAYER3["Layer 3: Query & Compute Engine"]
        CDQL["CDQL Query Planner &amp; Executor"]
        HNSW["In-Process MV-HNSW Vector ANN Index"]
        GORILLA["Gorilla Time-Series Compression"]
        WASMTIME["Wasmtime Sandboxed Bytecode Runtime"]
        GROUP_COMMIT["Double-Buffered Group Committer"]
    end

    subgraph LAYER4["Layer 4: Storage Foundation"]
        LMDB["LMDB Memory-Mapped B+Tree Substrate"]
        RECORDS["Document / Record Storage"]
        INDEXES["Composite Secondary Prefix Indices"]
        EDGES["Bi-Directional Graph Storage (Outbound &amp; Inbound)"]
        DISK[("data.mdb &amp; lock.mdb on Disk")]
    end

    LAYER1 --> LAYER2
    LAYER1 --> LAYER3
    LAYER2 --> LAYER3
    LAYER3 --> LAYER4
    LAYER4 --> DISK
```

---

## 🧠 Mental Model Breakdown

1. **Storage Foundation (Layer 4)**:
   - Houses the persistent B+Tree tables via LMDB.
   - Provides $O(\log N)$ point lookups, bi-directional graph traversal, composite secondary indices, and zero ghost pointer cleanup.
2. **Query & Compute Engine (Layer 3)**:
   - Bridges multiple database paradigms into a unified statement via CDQL pipelines (`->`).
   - Manages in-process vector ANN search, delta-of-delta time-series compression, and sandboxed WebAssembly execution.
3. **Cognitive Governance (Layer 2)**:
   - Acts as an intelligent supervisor for AI agents.
   - Pre-evaluates tool intents and queries, blocking destructive operations and injecting bounded `< 50 token` memory context packets.
4. **Access & Integration (Layer 1)**:
   - Polyglot interfaces for any host environment: embedded Rust traits, C/Python FFI headers, Axum HTTP endpoints, or MCP tool handlers.
