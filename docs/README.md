# 📚 Cluaizdb Documentation

Welcome to the official Cluaizdb documentation. This folder contains everything you need to understand, run, and scale Cluaizdb.

---

## 🧭 Table of Contents

### 1. 🌟 Vision & Overview
Understand why Cluaizdb exists and the philosophy behind its architecture.
- [Why Cluaizdb?](vision/why-cluaizdb.md)
- [For AI Agents](vision/for-ai-agents.md)

### 2. ⚡ Getting Started
Get up and running locally in under 60 seconds.
- [Quickstart](get-started/quickstart.md)
- [Installation Guide](get-started/installation.md)
- [Your First Mutating DB](get-started/your_first_mutating_db.md)

### 3. 🏗️ Architecture Deep Dive
Learn the internals of how Cluaizdb achieves 0ms latency and supports multiple database models.
- [LMDB Zero-Copy Storage](architecture/lmdb-zero-copy.md)
- [FFI Bypass (0ms Latency)](architecture/ffi-bypass.md)
- [Sensory Shards (High-Frequency)](architecture/sensory-shards.md)
- [The Dreamer Engine (GC & Analytics)](architecture/dreamer-engine.md)
- [Concurrency & Serialization Engine](architecture/concurrency-serialization.md)

### 4. 🗃️ 30 Database Paradigms (Engine Taxonomy)
Cluaizdb operates as a multi-model database engine, supporting 30 distinct paradigms native to the `UniversalNeuron` core:
- **Index & Deep Dive Guides**: Check out the comprehensive markdown guides located directly inside [docs/cluaizdb-types/](cluaizdb-types/):
  - [01-key-value.md](cluaizdb-types/01-key-value.md) | [02-graph.md](cluaizdb-types/02-graph.md) | [03-document.md](cluaizdb-types/03-document.md)
  - [04-relational.md](cluaizdb-types/04-relational.md) | [05-vector-ai.md](cluaizdb-types/05-vector-ai.md) | [06-time-series.md](cluaizdb-types/06-time-series.md)
  - [07-geo-spatial.md](cluaizdb-types/07-geo-spatial.md) | [08-wide-column.md](cluaizdb-types/08-wide-column.md) | [09-full-text-search.md](cluaizdb-types/09-full-text-search.md)
  - [10-blob-object.md](cluaizdb-types/10-blob-object.md) | [11-multi-model.md](cluaizdb-types/11-multi-model.md) | [12-hierarchical.md](cluaizdb-types/12-hierarchical.md)
  - [13-network.md](cluaizdb-types/13-network.md) | [14-object-oriented.md](cluaizdb-types/14-object-oriented.md) | [15-columnar.md](cluaizdb-types/15-columnar.md)
  - [16-in-memory.md](cluaizdb-types/16-in-memory.md) | [17-ledger-blockchain.md](cluaizdb-types/17-ledger-blockchain.md) | [18-spatial-geographic.md](cluaizdb-types/18-spatial-geographic.md)
  - [19-event-sourcing.md](cluaizdb-types/19-event-sourcing.md) | [20-autonomous.md](cluaizdb-types/20-autonomous.md) | [21-newsql.md](cluaizdb-types/21-newsql.md)
  - [22-streaming-reactive.md](cluaizdb-types/22-streaming-reactive.md) | [23-temporal.md](cluaizdb-types/23-temporal.md) | [24-array-tensor.md](cluaizdb-types/24-array-tensor.md)
  - [25-federated-virtual.md](cluaizdb-types/25-federated-virtual.md) | [26-multivalued.md](cluaizdb-types/26-multivalued.md) | [27-native-xml.md](cluaizdb-types/27-native-xml.md)
  - [28-spatial-temporal.md](cluaizdb-types/28-spatial-temporal.md) | [29-graph-relational.md](cluaizdb-types/29-graph-relational.md) | [30-embedded-in-process.md](cluaizdb-types/30-embedded-in-process.md)


### 5. 🧬 DNA Rulesets & Serialization Templates
See executable rulesets, schema files, and performance blueprints in the [DNA Templates Hub](cluaizdb-dna-templates/README.md):
- **Dynamic Rhai Scripts**: [CRUD Schema](cluaizdb-dna-templates/crud_schema.rhai) | [Weight Decay](cluaizdb-dna-templates/weight_decay.rhai) | [Time Series Decay](cluaizdb-dna-templates/time_series_decay.rhai) | [Hybrid Vector-Tag Search](cluaizdb-dna-templates/hybrid_search.rhai)
- **Zero-Copy WASM**: [Rust-WASM DNA Engine](cluaizdb-dna-templates/wasm_example.rs) | [Auto-WASM Schema Verification](cluaizdb-dna-templates/auto_wasm_example.rs)
- **Declarative Frameworks**: [JSON Rule Schema](cluaizdb-dna-templates/json_rules.json) | [CDQL Ingestion Pipeline](cluaizdb-dna-templates/cdql_example.md)
- **Binary Serialization**: [FlatBuffers & Protobuf Guide (Zero-Copy & Companion Object Avoidance)](cluaizdb-dna-templates/serialization_formats.md)

### 6. 🧬 The 4 Execution Engines (DNA)
Explore the 4 runtime modes (CDQL, WASM, Auto-WASM, Rhai) and the built-in paradigms.
- [DNA Architecture & The 4 Engines](genomes/dna-architecture.md)
- [Auto-WASM Strict Types Guide](genomes/auto-wasm-guide.md)
- [Key-Value DNA](genomes/key-value.md)
- [Graph Network DNA](genomes/graph-network.md)
- [Vector & AI DNA](genomes/vector-ai.md)
- [Document NoSQL DNA](genomes/document-nosql.md)

### 7. ⚡ CDQL (Cluaiz Database Query Language)
Master the universal query language pipeline.
- [CDQL Intro](query-language/cdql-intro.md)
- [Syntax Reference](query-language/syntax-reference.md)
- [Advanced Pipelines](cdql/advanced-pipelines.md)
- [Rosetta Stone Cheatsheet](cdql/rosetta-stone.md)

### 8. 📡 Clients & APIs
How to connect to Cluaizdb from your application.
- [REST API Reference](clients/rest-api.md)
- [C-FFI Bindings](clients/c-ffi.md)

---

*Note: All documentation is fully indexed and searchable via `registry.json`.*
