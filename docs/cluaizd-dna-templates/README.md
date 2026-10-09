# 🧬 Cluaizdb DNA Templates: The A-to-Z Developer Guide

Welcome to the **Cluaizdb DNA Ecosystem**. This directory is not just a collection of examples; it is the fundamental manual for engineering intelligence directly into your database shards.

If you are coming from traditional database architectures (PostgreSQL, MongoDB, Redis), you must unlearn the concept of "Database Constraints" and "Middleware Services". In Cluaizdb, the engine itself is completely agnostic to business logic. It does not know what a "User" is, what "Spam" is, or what "Time-To-Live (TTL)" means.

**Everything—every validation, every connection, every memory decay—is defined by you using DNA Templates.**

---

## 🏗️ 1. What is a DNA Template?

A DNA Template is a script (written in Rhai, CDQL, or compiled to WASM) that intercepts raw memory operations inside the Cluaizdb Rust engine. 

When you attach a DNA template to a workspace or a specific node, you are effectively giving that partition a "brain". The engine will pause its execution pipeline at specific lifecycle hooks, inject the physical memory (payloads, vectors, graph edges) into your script, and wait for your script's decision before proceeding.

### The 6 Core Biological Hooks
| Hook | Engine Trigger Phase | Purpose |
| --- | --- | --- |
| `on_write` | *Pre-Commit (WAL phase)* | Validation, Spam Prevention (Deep Archer), Schema Enforcement. |
| `on_read` | *Post-Fetch (Mmap phase)* | Dynamic RBAC, Edge Weight Reinforcement (Long Term Potentiation). |
| `on_index` | *Search Evaluation* | Custom Vector Math, Hybrid Scoring algorithms. |
| `on_traverse` | *Graph Walk (BFS/DFS)* | Path pruning, permission filtering during deep graph jumps. |
| `on_dream` | *Idle CPU/NPU State* | Subconscious processing, stochastic walks, Eureka generation. |
| `on_lifecycle` | *Background GC Thread* | Biological forgetting, ZSTD compression transitions, pruning. |

---

## ⚙️ 2. The Engine Synchronization Matrix

Writing a DNA script is only 50% of the architecture. The other 50% is how your script interacts with the Cluaizdb Rust Engine's physical memory settings. **Your `config.json` must explicitly define the `[database]` engine settings required for your template to run efficiently.**

If you pair a heavy computational DNA script with the wrong engine memory setting, you will bottleneck the entire cluster.

### A. Serialization & Memory: `payload_format`
When the engine hands data to your DNA script, it must serialize it. You must configure `payload_format` in your `cluaizdb.toml` or DNA `config.json` overrides.

- **`payload_format = "flatbuffers"` (The High-Performance Default)**
  - **Why:** Flatbuffers allow the DNA script to read properties of the payload *without* deserializing the whole object. This is **Zero-Copy**.
  - **When to use:** Crucial for `on_write` firewalls (like Deep Archer). You want to validate data in microseconds before committing to LMDB. If you use JSON here, the deserialization tax will cripple your write throughput.
- **`payload_format = "json"`**
  - **Why:** Extremely flexible, human-readable.
  - **When to use:** For standard web-apps where throughput is < 1,000 TPS and development speed is prioritized over microsecond latency.

### B. Locking Mechanisms: `concurrency_mode`
When your DNA script executes, how does it lock the memory?
- **`concurrency_mode = "mutex"`**
  - **Why:** Ensures absolute thread safety at the cost of blocking. If a background DNA script (like the Dreaming Engine) is modifying graph edges, it locks the shard.
  - **When to use:** When your DNA script performs heavy graph mutations (`on_dream` forging new Hypothesis edges) and you cannot risk dirty reads from API clients.
- **`concurrency_mode = "dashmap"` (High-Performance Sharded Locks)**
  - **Why:** Allows immense concurrent reads while a DNA script is writing by sharding the memory locks.
  - **When to use:** When your application is read-heavy (e.g., a real-time recommendation engine) and you cannot afford the Dreaming Engine or TTL GC threads locking the database.

---

## 📂 3. Navigating This Directory

Inside this directory, you will find three master templates. Each folder contains an A-to-Z breakdown of how to build and configure that specific intelligence:

1. **`deep_archer/`**: The AI Firewall. Learn how to use Flatbuffers and the `on_write` hook to mathematically block AI hallucinations and spam.
2. **`dreaming_engine/`**: Subconscious Insights. Learn how to use Mutex locking and the `on_dream` hook to generate new connections during server downtime.
3. **`neural_ttl/`**: Biological Forgetting. Learn how to use the `on_lifecycle` hook to compress old memories to ZSTD and purge heavy payloads.

### Multi-Language Support
Each folder contains the template written in 3 ways so you can choose your weapon:
- **`*.rhai`**: Fast, easy to read, dynamic scripting.
- **`*.cdql`**: Native query language for declarative logic.
- **`*.auto_wasm.rs`**: Rust code that Cluaizdb will automatically compile to WASM for absolute C-level execution speed.

Dive into the folders above to begin engineering your nervous system.
