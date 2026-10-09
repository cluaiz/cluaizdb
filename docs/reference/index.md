# 📚 Cluaizdb Reference Manual

Welcome to the **cluaizdb Reference Manual**. This document serves as the definitive architectural and operational specification for the Cluaizdb Database. Unlike traditional relational or document-oriented databases, Cluaizdb is engineered from the ground up to eliminate the bottlenecks associated with Inter-Process Communication (IPC), heavy query parsers, and synchronous disk flushing. It is a high-throughput, zero-copy compute node designed to execute data mutations and vector-based similarity searches at the physical limits of modern hardware.

### Architectural Philosophy
At the core of Cluaizdb is the Zero-Copy Memory Map (`memmap2`) layered over a lock-free Multi-Version Concurrency Control (MVCC) B-Tree. Traditional databases typically suffer from severe garbage collection pauses and CPU cache-misses because they deserialize data from the disk into heap memory before evaluating queries. Cluaizdb bypasses this by parsing byte arrays directly from the Operating System's page cache. When a query is executed, the engine streams data directly to the active TCP socket, ensuring that physical RAM usage remains relatively flat regardless of the total dataset size.

To guarantee strict ACID durability without sacrificing speed, every mutation is first piped directly to an append-only Write-Ahead Log (WAL). This ensures that ingestion pipelines—often driven via the high-throughput HTTP `/neuron` endpoint—are never stalled by secondary index rebalancing or B-Tree node splitting. 

### Absolute Flexible Execution
The most significant departure from legacy database architectures is the deliberate minimalism of the Cluaizdb Query Language (CDQL). Instead of bloating the CDQL parser with hundreds of hardcoded mathematical functions (such as `SUM`, `AVG`, or `GROUP BY`), Cluaizdb delegates complex logic to sandboxed WebAssembly (WASM) and Rhai execution affordances. 

Developers can inject these executable scripts (referred to internally as DNA) directly into the database. The engine compiles and executes these scripts locally against the memory map, allowing for Turing-complete Map-Reduce aggregations, dynamic on-the-fly data redaction, and synchronous event hooks (`on_write`, `on_delete`) without incurring the latency of network round-trips to a separate backend server.

### How to Use This Manual
This reference dictionary is divided into five distinct pillars. It is specifically designed for Database Administrators tuning the LMDB virtual address space, Systems Engineers building zero-allocation network integrations, and Backend Developers writing complex vector similarity searches utilizing hardware SIMD intrinsics.

Please navigate to the specific subsystem reference below to explore the underlying time complexity, byte-level architecture, and configuration parameters of Cluaizdb:

---

## 1. [CDQL Syntax Dictionary](./cdql_index.md)
The primary query language reference (CDQL) governs all high-level interactions with the zero-copy memory map. Unlike SQL, which requires parsing and planning massive string-based queries that allocate heavily on the heap, CDQL is explicitly engineered to minimize latency. It bypasses traditional deserialization bottlenecks by scanning raw byte arrays directly from the Operating System's page cache. This section details the complete syntax and architectural execution of core read traversals (`find`), conditional byte-level filtering (`where`), hardware-accelerated SIMD vector similarity searches (`similar`), and MVCC-compliant lock-free mutations (`update`, `delete`). By mastering CDQL, developers can execute millions of queries per second without triggering aggressive garbage collection pauses.

**Quick Links:**
[`cosine_distance`](./cdql/cosine_distance.md) &nbsp;\|&nbsp; [`crispr`](./cdql/crispr.md) &nbsp;\|&nbsp; [`delete`](./cdql/delete.md) &nbsp;\|&nbsp; [`euclidean_distance`](./cdql/euclidean_distance.md) &nbsp;\|&nbsp; [`find`](./cdql/find.md) &nbsp;\|&nbsp; [`force`](./cdql/force.md) &nbsp;\|&nbsp; [`gt`](./cdql/gt.md) &nbsp;\|&nbsp; [`insert`](./cdql/insert.md) &nbsp;\|&nbsp; [`limit`](./cdql/limit.md) &nbsp;\|&nbsp; [`lt`](./cdql/lt.md) &nbsp;\|&nbsp; [`similar`](./cdql/similar.md) &nbsp;\|&nbsp; [`update`](./cdql/update.md) &nbsp;\|&nbsp; [`where`](./cdql/where.md)

## 2. [DNA Execution Affordances](./dna_master_index.md)
Cluaizdb eliminates the need for hundreds of bloated, hardcoded mathematical functions and trigger schemas by utilizing an Absolute Flexible Architecture known as Execution Affordances (DNA). This subsystem allows developers to inject highly optimized WebAssembly (WASM) or Rhai scripts directly into individual records. The Engine instantiates a secure sandbox to execute these scripts locally, interacting with the memory map via zero-allocation pointers. This section exhaustively covers Event Hooks for dynamic data redaction and transaction rollbacks (`on_write`, `on_read`, `on_delete`), internal Engine APIs for fetching secondary records mid-transaction (`ctx.fetch`), and massively parallel, lock-free Map-Reduce aggregations (`sum`, `avg`, `group_by`) executed across Tokio Green Threads.

**Quick Links:**
[`avg`](./dna/avg.md) &nbsp;\|&nbsp; [`ctx.abort`](./dna/ctx_abort.md) &nbsp;\|&nbsp; [`ctx.fetch`](./dna/ctx_fetch.md) &nbsp;\|&nbsp; [`ctx.time`](./dna/ctx_time.md) &nbsp;\|&nbsp; [`group_by`](./dna/group_by.md) &nbsp;\|&nbsp; [`on_delete`](./dna/on_delete.md) &nbsp;\|&nbsp; [`on_read`](./dna/on_read.md) &nbsp;\|&nbsp; [`on_write`](./dna/on_write.md) &nbsp;\|&nbsp; [`sum`](./dna/sum.md)

## 3. [HTTP & WebSocket API](./api_master_index.md)
While CDQL handles the logic, the actual binary transmission between the Engine and external applications relies on a highly specialized networking stack. This reference details the protocol-level mechanics required to build native integrations in Node.js, Python, or Rust. It covers the cryptographic handshake utilizing Ed25519 signatures and JWTs for edge-level Authorization, ensuring invalid requests are dropped before reaching the parser. Furthermore, it explains the asynchronous Direct I/O Pipelining used by the `POST /neuron` route for maximum ingestion throughput, as well as the persistent, full-duplex WebSocket architecture (`ws:// /stream`) designed to handle streaming millions of records with OS-level TCP backpressure mitigation.

**Quick Links:**
[`Authorization`](./api/auth.md) &nbsp;\|&nbsp; [`POST /neuron`](./api/post_neuron.md) &nbsp;\|&nbsp; [`ws:// /stream`](./api/ws_stream.md)

## 4. [Configuration Reference](./config_master_index.md)
Deploying cluaizdb in a production environment requires precise architectural tuning. The Configuration Reference is designed specifically for Database Administrators (DBAs) and Systems Engineers responsible for optimizing the engine to the physical constraints of the host hardware. This section deeply explores the critical parameters defined in the `cluaizdb.toml` configuration file. It provides exhaustive guidelines on allocating the optimal Virtual Memory Address (VMA) space for the LMDB environment (`memory_map_size`), tuning the Write-Ahead Log (WAL) synchronization behavior (`wal_sync`) to balance extreme disk throughput against ACID crash durability, and binding the internal TCP sockets and Prometheus telemetry endpoints.

**Quick Links:**
[`memory_map_size`](./config/memory_map_size.md) &nbsp;\|&nbsp; [`port_binding`](./config/port_binding.md) &nbsp;\|&nbsp; [`telemetry`](./config/telemetry.md) &nbsp;\|&nbsp; [`wal_sync`](./config/wal_sync.md)

## 5. [Data Types Reference](./types_master_index.md)
Understanding the physical storage constraints and serialization strategies of cluaizdb is vital for schema optimization. This reference breaks down exactly how the engine translates developer-friendly data structures into compact, memory-aligned binary formats. It details the internal transformation of schemaless, deeply nested payloads into BSON/MsgPack equivalents to enable zero-allocation byte-level filtering (`json`). Additionally, it explains the memory-padding alignment required for high-dimensional arrays (`vector`), ensuring that 32-bit floats sit perfectly inside the CPU's L1 cache boundaries. This alignment is what allows AVX-512 SIMD instructions to load and compute semantic distances without incurring CPU stall cycles.

**Quick Links:**
[`json`](./types/json.md) &nbsp;\|&nbsp; [`vector`](./types/vector.md)
