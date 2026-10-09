# CLUAIZD AI Agent & Cursor Rules (`skill.md`)

This file is a **complete context specification** for AI coding assistants (Cursor, Claude, GitHub Copilot, GPT-4). Paste this entire file as a system prompt or `.cursorrules` context when working on any CLUAIZD codebase.

---

## 🧠 What CLUAIZD Is (Core Mental Model)

CLUAIZD is a **shape-shifting memory substrate** built in Rust on top of LMDB. You must never think of it as a traditional database.

**The single most important concept:**
- The Rust core engine (`engine-lmdb`) is **100% logicless**. It stores raw bytes. It knows nothing about schemas, indexes, TTLs, or query optimizations.
- ALL intelligence lives in external WASM/Rhai scripts called **Genomes** (`*.json` files in `genomes/`).
- Genomes are attached to individual data records (Neurons) and contain 4 hooks: `on_write`, `on_read`, `on_index`, `on_lifecycle`.

---

## 📁 Codebase Structure (Critical Paths)

```
cluaizdb/
├── crates/
│   ├── genome/src/cdql/      ← CDQL parser.rs + planner.rs (THE QUERY BRAIN)
│   ├── server/src/routes/    ← HTTP API (query.rs, data.rs, state.rs)
│   └── storage/
│       ├── engine-lmdb/      ← Raw LMDB read/write (NEVER add business logic here)
│       └── wal/              ← Write-Ahead Log for crash recovery
├── components/
│   └── cluaizdb-types/          ← UniversalNeuron, StorageTier, DNA structs
├── genomes/                  ← All 10 DNA scripts (sql_strict.json, etc.)
├── ffi/                      ← C-FFI bindings (cluaizdb.h + lib.rs)
└── docs/                     ← Documentation (registry.json + markdown)
```

---

## 🦀 Core Data Structure (Always Use This)

```rust
// The ONLY data container in the entire system. Everything is a UniversalNeuron.
pub struct UniversalNeuron {
    pub id: Uuid,
    pub tier: StorageTier,        // Hot | Warm | Cold
    pub raw_payload: Bytes,       // Raw JSON/binary. The engine NEVER parses this.
    pub vector_data: Vec<f32>,    // High-dimensional embedding. Can be empty.
    pub adjacency: Vec<Synapse>,  // Graph edges. Can be empty.
    pub dna: Option<Dna>,         // The genome script to attach. Can be None.
}

pub struct Synapse {
    pub target_id: Uuid,
    pub relation: String,         // Edge type: "friends", "bought_by", "causes", etc.
    pub weight: f32,              // Edge strength: 0.0 - 1.0
}

pub struct Dna {
    pub engine: String,           // "rhai" or "wasm"
    pub on_write: Option<String>, // Rhai script for write validation
    pub on_read: Option<String>,  // Rhai script for read transformation
    pub on_index: Option<String>, // Rhai script for index building
    pub on_lifecycle: Option<String>, // Rhai script for TTL/tier management
    pub parameters: serde_json::Value, // Config values accessible in Rhai as `config`
    pub wasm_module: Option<Vec<u8>>,  // Compiled WASM bytes (for wasm engine)
}
```

---

## ⚡ CDQL Grammar (Always Use This Syntax)

```
Query = FindClause ("->" PipelineStage)*

FindClause:
  find id("uuid")                              # 0ms Fast-Path
  find Label(field: value, ...)               # Labeled scan with filters
  find *(field: value, ...)                   # Wildcard scan

PipelineStage (in order of cheapness — cheapest first):
  filter <field> <op> <value>                 # Narrow result set
  sort_by("<field>", asc: bool)               # Sort
  limit <n>                                   # Cap results
  traverse(edge: "str", hops: N..M, min_weight: f) # Graph traversal
  join(target: "Label", on: "l == target.r", type: "inner"|"left")
  group_by("<field>")                          # Group
  aggregate(count(), sum(f), avg(f), max(f), min(f))
  time_window(size: "5m"|"1h"|"1d")           # Time-series bucketing
  similar_to(vector: [f32...], metric: "cosine"|"l2"|"dot") # Vector search
  search(query: "str", fuzzy: bool, fields: {field: weight}) # Full-text
  geo_near(lat: f64, lon: f64, radius: "5km") # Radius search
  range_scan(field: "str", start: val, end: val) # Ordered scan
  stream(bytes: start..end)                   # Byte-range blob streaming
  project(keep: ["field1", "field2"])          # Output shaping
  unwind("array_field")                       # Array expansion
```

---

## 🧬 Genome Rules (Critical — Never Violate)

1. **NEVER add business logic to Rust core.** Schema validation, TTL, indexing — ALL of this belongs in Genome Rhai scripts.
2. **Genome hooks are Rhai scripts** returning a map. `on_write` must return `#{ action: "Allow" }` or `#{ action: "Abort", error: "msg" }`.
3. **WASM genomes** are for compute-heavy operations (vector math, text parsing). Use Rhai for simple rule scripts.
4. **DNA is attached per-neuron**, not per-table. Two neurons in the same "collection" can have different DNA.

### Genome Template
```json
{
  "on_write": "let res = #{ action: \"Allow\" };\n/* validate payload here */\nres",
  "on_read": "let res = #{ payload: payload };\n/* transform payload here */\nres",
  "on_index": null,
  "on_lifecycle": "let res = #{};\nif age_ns > config.ttl_ns { res.action = \"Evict\"; }\nres",
  "parameters": { "ttl_ns": 3600000000000 },
  "engine": "rhai"
}
```

---

## 🏗️ LMDB Rules (Critical — Never Violate)

1. **NEVER call LMDB directly from route handlers.** Always go through `engine-lmdb` crate functions.
2. **LMDB environments are per-shard.** The shard is determined by `tenant_id` query param.
3. **WAL must be written BEFORE LMDB.** The WAL guarantees crash-safe idempotent replay.
4. **LMDB map_size is fixed at open.** Do NOT attempt to resize at runtime.

---

## 🔒 The Zero-Logic Substrate Rule (THE LAW)

> "The Rust core engine must contain zero business logic."

If you are about to add an `if` statement in `engine-lmdb` that checks a field value, STOP. That logic belongs in a Genome script.

**Correct:**
```rust
// engine-lmdb/src/lib.rs — just bytes in, bytes out
pub fn write_neuron(env: &Env, neuron: &UniversalNeuron) -> Result<()> {
    let serialized = encode(neuron)?;
    lmdb_put(env, &neuron.id.to_string(), &serialized)?;
    Ok(())
}
```

**Incorrect:**
```rust
// engine-lmdb/src/lib.rs — NEVER DO THIS
pub fn write_neuron(env: &Env, neuron: &UniversalNeuron) -> Result<()> {
    if neuron.raw_payload.contains("password") {  // ← WRONG! This is a Zero-Logic violation
        return Err("Cannot store password field");
    }
    // ...
}
```

---

## 🚨 Common Mistakes AI Agents Make

| Mistake | Why It's Wrong | Correct Approach |
|---|---|---|
| Adding schema validation in Rust | Violates Zero-Logic Rule | Write an `on_write` Rhai hook in a Genome JSON file |
| Using `HashMap` for neuron storage | Bypasses LMDB mmap | Always use `engine-lmdb` crate functions |
| Calling `serde_json::from_slice` in engine-lmdb | Engine must be payload-agnostic | Parse in CDQL executor or Genome script |
| Writing a cron job for TTL eviction | Dreamer engine handles this | Set `on_lifecycle` hook in the Genome |
| Hardcoding the port `8080` | Should read from config | Use `config.server.port` from `config.toml` |
| Ignoring WAL on writes | Data loss on crash | Always append to WAL before LMDB |

---

## 📡 API Contract (Quick Reference)

```
POST /neuron              — Insert/update a UniversalNeuron
POST /query             — Execute CDQL query string
GET  /neuron/:id          — Get single neuron by ID
GET  /ws/telemetry            — Server health + metrics
GET  /state             — RAM, disk, shard info

Query Params:
  ?tenant_id=name       — Routes to specific LMDB shard (default: "default_sandbox")
```

---

## ✅ Before Submitting Any Code Change

- [ ] Does this add logic to `engine-lmdb`? (It shouldn't. Zero-Logic Rule.)
- [ ] Does this bypass the WAL? (It shouldn't. Crash safety.)
- [ ] Does this hardcode a schema? (It shouldn't. Use a Genome.)
- [ ] Does this assume all data is JSON? (`raw_payload` is raw bytes, can be any format.)
- [ ] Does this work with `tenant_id` sharding? (Multi-shard support is always required.)
