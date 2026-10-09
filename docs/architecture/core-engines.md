# Core Execution Engines

Cluaizdb is not a monolithic database. Under the hood, it operates as a sophisticated orchestrator over **four distinct, high-performance execution engines** built entirely in Rust. 

When you issue a CDQL query, the `cluaizdb-server` parser translates your syntax into an Abstract Syntax Tree (AST) and routes it to the specific hardware-optimized engine designed for that workload.

This architecture is what allows Cluaizdb to behave as a Document Store, a Vector Database, a Graph Database, and a Time-Series Database simultaneously—without the overhead of running four separate database clusters.

---

## 1. Vector Search Engine (`cluaizdb-index-mvhsnw`)
*The Pinecone / Milvus Replacement*

To support generative AI, LLM embeddings, and semantic search, Cluaizdb uses a heavily optimized **Multi-Version Hierarchical Navigable Small World (MV-HNSW)** index.

- **SIMD Acceleration:** Core distance metrics (Cosine, Euclidean, Dot Product) are executed using hardware-level SIMD instructions (AVX-512 / NEON) for maximum throughput.
- **Sub-millisecond Latency:** Bypasses O(N) linear scans entirely. Searches across millions of embeddings execute in microsecond territory.
- **Zero-Copy Integration:** The vector index holds pointers directly into the memory-mapped LMDB storage, meaning vectors are never duplicated in RAM.

**CDQL Trigger:** `-> similar_to(vector: [...], metric: "cosine")`

---

## 2. Graph Traversal Engine (`cluaizdb-graph-engine`)
*The Neo4j / ArangoDB Replacement*

Relational SQL JOINs scale poorly when traversing complex, deeply nested relationships (e.g., social networks, fraud detection rings, neural pathways). Cluaizdb replaces JOINs with a native Graph Engine.

- **Adjacency Lists:** Every Universal Neuron physically stores its outbound edges/connections in a native Rust `Vec<NeuronEdge>`.
- **In-Memory BFS/DFS:** Traversals do not require disk seeks. The Graph Engine hops from node to node in L1/L2 CPU cache using Breadth-First or Depth-First Search algorithms.
- **Shortest Path Calculation:** Natively computes the shortest path between two nodes (e.g., Dijkstra's algorithm equivalent) instantly.

**CDQL Trigger:** `-> traverse(edge: "friends", hops: 3)` or `-> shortest_path(to: "node_b")`

---

## 3. Time-Series Engine (`cluaizdb-time-series`)
*The InfluxDB / TimescaleDB Replacement*

IoT devices, stock tickers, and robotics telemetry stream millions of data points per second. Storing this as standard JSON documents would instantly exhaust disk space and I/O bandwidth.

- **Gorilla Compression Algorithm:** Implements the famous Gorilla compression strategy.
  - **Timestamps:** Uses Delta-of-Delta compression. If a sensor reports exactly every 1 second, the engine stores a single `0` bit instead of an 8-byte integer.
  - **Floats:** Uses XOR compression. If the temperature value doesn't change, it stores `0` bits instead of a 64-bit float.
- **Massive Density:** Compresses 100GB of raw JSON telemetry into ~2GB of bit-packed buffers.

**CDQL Trigger:** `-> time_window(size: "5m") -> aggregate(avg(temperature))`

---

## 4. Distributed Consensus Engine (`cluaizdb-consensus-raft`)
*The Zookeeper / etcd Replacement*

To guarantee $99.999\%$ uptime in multi-node Enterprise environments, Cluaizdb relies on a robust distributed consensus protocol.

- **Raft Implementation:** A clean, safe Rust implementation of the Raft protocol.
- **Leader Election:** If the primary database node crashes, a follower node detects the heartbeat failure and promotes itself to Leader within 150-300 milliseconds.
- **Replicated State Machine:** Write operations (`insert`, `delete`, `update`) are appended to a distributed WAL (Write-Ahead Log) and are not confirmed to the user until a quorum (majority) of nodes acknowledge the write.

**CDQL Trigger:** All state-mutating commands (e.g., `insert into...`) are automatically routed through the Raft Engine when clustering is enabled.
