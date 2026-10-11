# 💡 Why CluaizDB?

> **Component:** `docs/vision`  
> **Philosophy:** Unified In-Process Cognitive Engine  
> **Target:** AI Agents, Local-First Systems & Autonomous Workflows

---

## 🛑 The Fragmented Modern Database Stack

Modern applications and AI agents juggle separate databases for every workload:

```mermaid
flowchart TD
    APP["Modern Application / AI Agent"]
    
    subgraph FRAGMENTED["The Fragmented Legacy Stack"]
        REDIS["Redis (Key-Value Caching)<br/>Separate process, TCP socket, in-memory volatile"]
        PG["PostgreSQL (Relational Tables)<br/>Heavy background daemon, connection pool, migrations"]
        NEO4J["Neo4j (Relationship Graph)<br/>JVM daemon, memory hog, separate query language (Cypher)"]
        QDRANT["Qdrant / Pinecone (Vector Embeddings)<br/>Docker container, network latency on every search"]
        TS["TimescaleDB (Time-Series Metrics)<br/>Complex partitioning, disk amplification"]
    end

    APP -->|"TCP Socket 1"| REDIS
    APP -->|"TCP Socket 2"| PG
    APP -->|"TCP Socket 3"| NEO4J
    APP -->|"TCP Socket 4"| QDRANT
    APP -->|"TCP Socket 5"| TS
```

### The Pain Points:
1. **Docker & Daemon Tax**: Developers must configure Docker Compose, manage ports, and burn gigabytes of RAM just for local development.
2. **Network Serialization Overhead**: Every query converts data into JSON or protobuf, crosses local TCP sockets, and incurs kernel context switching.
3. **Application Glue Code**: Developers write fragile Python or TypeScript code to join relational users with vector search results and graph relationships.

---

## ⚡ The CluaizDB Architecture: Multi-Model In-Process Synthesis

CluaizDB was engineered from first principles in Rust to provide a single, embedded storage and query engine:

```mermaid
flowchart LR
    APP["AI Agent / Application Binary"]
    
    subgraph CLUAIZDB["CluaizDB In-Process Engine (Zero Network Hop)"]
        CDQL["CDQL Query Engine<br/>Single statement bridges KV, Graph, Vector &amp; Time-Series"]
        
        subgraph SUBSTRATE["Unified Storage Substrate (LMDB mmap)"]
            KV["Records (Document / KV)"]
            GRAPH["Bi-Directional Graphs"]
            VEC["MV-HNSW Vectors"]
            TS["Gorilla Time-Series"]
        end
    end

    APP <-->|"Direct Rust Trait / In-Process Memory"| CDQL
    CDQL <--> SUBSTRATE
```

### 1. In-Process Direct Memory Access
Runs directly inside your application binary using an LMDB-backed virtual memory map. No Docker containers, zero network roundtrip latency.

### 2. Multi-Model Synthesis via CDQL
A single query statement bridges across data models:
```text
find users(active = true) -> get friends -> similar_to(vector: [...], metric: "cosine") -> limit 5
```

### 3. Active Cognitive Governance
Instead of passive dumb storage, CluaizDB provides native agent governance—evaluating safety policies before write operations and providing strictly bounded (< 50 tokens) memory injection to prevent LLM prompt stuffing.
