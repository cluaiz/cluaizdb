# DNA Architecture (Genomes)

CLUAIZD relies on a Modern concept called **Genomes**. The core rust engine (`engine-lmdb`) contains zero business logic, schemas, or indexing optimizations. It only understands how to rapidly serialize and deserialize byte arrays.

All intelligence is injected into the engine using DNA.

## What is a Genome?
A Genome is an isolated script (written in WASM or Rhai) that attaches to a Neuron. When the CLUAIZD core engine interacts with that Neuron, it triggers specific hooks defined by the Genome.

### The 5 Biological Hooks

Every Genome can implement up to **5 hooks**. Each hook is a Rhai script string (or WASM function) that fires at a specific lifecycle event:

---

#### 1. `on_write` — Ingestion Gate
**Fires:** Before every `POST /neuron` write, before WAL commit.  
**Returns:** `#{ action: "Allow" }` to permit, or `#{ action: "Abort", error: "msg" }` to reject.  
**Use for:** Schema validation, append-only enforcement, field presence checks.

```json
{
  "on_write": "let res = #{ action: \"Allow\" };\nif !payload.contains(\"id\") { res.action = \"Abort\"; res.error = \"Missing id\"; }\nres"
}
```

---

#### 2. `on_read` — Access Control & Transformation
**Fires:** When a CDQL query reads this neuron's payload.  
**Returns:** `#{ payload: modified_payload }`.  
**Use for:** Row-Level Security (decrypt for authorized users), field masking, live data reshaping.

```json
{
  "on_read": "let res = #{ payload: payload };\nif !caller.has_role(\"admin\") { res.payload.remove(\"ssn\"); }\nres"
}
```

---

#### 3. `on_index` — Async Post-Write Indexing
**Fires:** Asynchronously AFTER a successful write (non-blocking).  
**Returns:** `#{ text_fields: ["title", "body"], language: "english" }`.  
**Use for:** Extracting fields into the Inverted Index for full-text search, generating vector embeddings.

```json
{
  "on_index": "let res = #{};\nres.text_fields = [\"title\", \"content\", \"tags\"];\nres.language = \"english\";\nres"
}
```

---

#### 4. `on_lifecycle` — Dreamer Eviction & Tiering
**Fires:** Every N seconds by the background Dreamer engine (configurable in `config.toml`).  
**Returns:** `#{}` for no action, `#{ action: "Evict" }` to delete, `#{ new_tier: "Cold", compress_level: 9 }` to compress.  
**Use for:** TTL eviction (In-Memory Cache-like), automatic Cold-tier demotion for old data.

```json
{
  "on_lifecycle": "let res = #{};\nif age_ns > config.ttl_ns { res.action = \"Evict\"; }\nres",
  "parameters": { "ttl_ns": 3600000000000 }
}
```

---

#### 5. `on_traverse` — Graph Edge Filtering *(Advanced)*
**Fires:** During `GET /graph/{id}/traverse` — once per edge, deciding whether to follow it.  
**Returns:** `#{ follow: true }` to include the edge, `#{ follow: false }` to skip it.  
**Use for:** Access-controlled graph traversal (skip edges below a weight threshold, skip edges to blocked users).

```json
{
  "on_traverse": "let res = #{ follow: true };\nif edge_weight < config.min_weight { res.follow = false; }\nres",
  "parameters": { "min_weight": 0.5 }
}
```

> [!NOTE]
> `on_traverse` is only evaluated by `GET /graph/{id}/traverse`. The CDQL `traverse()` pipeline operator uses its own `min_weight` parameter instead. Use `on_traverse` for HTTP graph traversal with per-neuron access control policies.

---

## Full Genome Template

```json
{
  "engine": "rhai",
  "sync_write": "lite",
  "on_write": "let res = #{ action: \"Allow\" }; res",
  "on_read": "let res = #{ payload: payload }; res",
  "on_index": null,
  "on_lifecycle": "let res = #{}; if age_ns > config.ttl_ns { res.action = \"Evict\"; } res",
  "on_traverse": null,
  "parameters": {
    "ttl_ns": 3600000000000,
    "min_weight": 0.0
  },
  "wasm_module": null
}
```

## The 4 Execution Engines

Cluaizdb supports 4 distinct execution modes for DNA validation and filtering. These modes give developers the flexibility to choose between ease-of-use and maximum performance.

### 1. `cdql` (Native JSON Translation)
**Best for:** 90% of users who only need declarative data filtering.
**How it works:** The user provides a JSON rule (e.g., `{"age": {">": 18}}`). The database translates this JSON into a CDQL Abstract Syntax Tree (AST).
**Speed:** `0.05 ms` (Native Rust speed).

### 2. `wasm` (Direct Native Compiled)
**Best for:** Pro developers doing complex custom mathematics or proprietary algorithms.
**How it works:** The developer writes code in Rust or C++, compiles it locally to a `.wasm` file, and uploads the Base64 bytes directly via the API.
**Speed:** `0.05 ms` (Hardware Machine Code).

### 3. `auto-wasm` (Auto-Compiled Rust)
**Best for:** Developers who want complex math and strict typing but don't want to compile WASM themselves.
**How it works:** The user uploads a raw Rust script. Our `cluaizdb` server automatically compiles it into a `.wasm` binary in the background and saves it to the `active_dnas/` directory.
**Hot-Reload:** Yes! The File Watcher detects the new binary and Hot-Reloads it into RAM instantly with zero downtime.
**Learn More:** See the [Auto-WASM Guide](auto-wasm-guide.md) to understand why defining strict struct types is crucial.

### 4. `rhai` (Legacy Interpreter)
**Best for:** Rapid prototyping or legacy dynamic scripts.
**How it works:** The user uploads a dynamic Rhai script. The server parses the AST and evaluates the logic line-by-line during execution.
**Speed:** `2.0 - 5.0 ms` (Slowest option).

---

## ⚡ Zero-Downtime RAM Hot-Reloading

One of Cluaizdb's most powerful features is **Absolute Zero Path Hot-Reloading** for WASM modules.

Historically, databases require a full restart to load new plugins, or they suffer massive SSD I/O bottlenecks by reading plugins from the disk on every query. Cluaizdb solves this using a background RAM synchronization engine.

### How it Works:
1. **The `active_dnas/` Directory:** The server creates an isolated directory in the root folder.
2. **Global RAM Cache:** Cluaizdb initializes a highly-concurrent, thread-safe `DashMap` in RAM that holds pre-compiled WASM modules.
3. **The File Watcher:** A background `tokio` thread uses OS-level events (`notify` crate / FSEvents) to monitor `active_dnas/`.
4. **Instant Swap:** When a new `.wasm` file is uploaded (either manually or via the `auto-wasm` API), the watcher detects it and instantly swaps the compiled `Module` inside the RAM cache. 

### Why it Matters:
When the Database FFI Engine (`engine-lmdb`) needs to execute a DNA hook, it **never touches the SSD**. It pulls the pre-compiled binary directly from the RAM Cache. This results in **0.0001 ms** execution overhead and allows developers to hot-swap database logic in production with **zero downtime**.
