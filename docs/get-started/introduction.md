# What is CLUAIZD?

Welcome to the official documentation for **CLUAIZD (Core Network System Database)**.

## The Autonomic Memory Substrate
Traditional databases force you to choose an architecture before you start building. If you need relationships, you pick Graph DB. If you need search, you pick Search Engine. If you need JSON documents, you pick Document Store. This results in massive architectural fragmentation, data syncing headaches, and system bloat.

**CLUAIZD shatters this paradigm.**

Built in Rust on top of LMDB, CLUAIZD is a biological-inspired, autonomic memory substrate. Out of the box, it is a completely empty "Brain" (a graph of Neurons and Synapses). It possesses no hardcoded schemas, no specialized indexes, and no arbitrary limits.

It is a **10-in-1 shape-shifting engine**.

> [!NOTE]
> **The Zero-Logic Substrate Rule**
> The core Rust engine contains zero business logic or query optimization rules. It acts solely as a high-speed router. All "database intelligence" is dynamically injected via WASM Genomes.

## How it works: Genomes (DNA)
Instead of hardcoding features into the core engine, CLUAIZD relies on **Genomes** (WASM or Rhai scripts). When you insert a Neuron (data record) into CLUAIZD, you attach a specific Genome to it. 

For example:
- Attach `sql_strict.json` and the Neuron acts like a strict Relational table.
- Attach `time_series.json` and the Neuron will automatically compress old data and aggregate time windows.
- Attach `ephemeral_cache.json` and the Neuron behaves like In-Memory Cache, evicting itself from memory based on TTL.

## The cluaizdb Neural Query Language (CDQL)
To command these diverse Genomes, CLUAIZD introduces **CDQL** — a universal, pipeline-based query language. CDQL allows you to run Graph traversals, Vector searches, and Geo-Spatial queries seamlessly within the exact same pipeline string.

```text
// Find blue cars, traverse their owner graph, and filter by radius
find Car(color: "blue") 
  -> traverse(edge: "owner", hops: 1..2)
  -> geo_near(lat: 28.6, lon: 77.2, radius: "5km")
```

## Why CLUAIZD?
1. **0ms Latency:** C-FFI bindings allow robotics, Python engines, and C++ game engines to access memory-mapped data directly without TCP/HTTP overhead.
2. **Bits-to-Atoms Tiering:** The background "Dreamer" engine automatically downgrades unused payloads into heavy ZSTD compression (Cold Storage) to prevent OOM (Out-of-Memory) crashes without deleting data.
3. **Crash Safety:** A robust Write-Ahead Log (WAL) ensures your memory substrate recovers idempotently after power failures.

---
Let's build a nervous system. Proceed to the [Quickstart](quickstart) to spin up your engine.
