# ⚡ Quickstart

Get CLUAIZD running and your first data in under **60 seconds**.

## Prerequisites
- **Rust Toolchain:** Version `1.75+` — install via [rustup.rs](https://rustup.rs/).
- **LLVM / Clang:** Required by `bindgen` to compile the LMDB C binding.

## Step 1: Run the Server

```bash
git clone https://github.com/cluaiz/cluaizdb.git
cd cluaizdb
cargo run -p cluaizdb-server
```

You should see:
```
INFO cluaizdb CLUAIZD Server v0.0.1 starting
INFO Running WAL crash recovery...
INFO WAL boot recovery complete ✅
INFO Server listening on 0.0.0.0:7331
```

> [!TIP]
> **Port is 7331** — not 8080. CLUAIZD uses port 7331 by default.

---

## Step 2: Write Your First Neuron

CLUAIZD stores everything as a `UniversalNeuron`. No `CREATE TABLE`. No schema.

```bash
curl -X POST http://localhost:7331/neuron \
     -H "Content-Type: application/json" \
     -d '{
       "id": "user_aryan",
       "tier": "Hot",
       "raw_payload": [123, 34, 110, 97, 109, 101, 34, 58, 32, 34, 65, 114, 121, 97, 110, 34, 125],
       "vector_data": [],
       "adjacency": []
     }'
```

`raw_payload` is raw UTF-8 bytes. The above bytes decode to `{"name": "Aryan"}`.

**Response:** `{ "status": "written", "id": "user_aryan" }`

## 3. Query via CDQL

Now, we can query our neuron using the cluaizdb Neural Query Language (CDQL). 

```bash
curl -X POST http://localhost:7331/query \
     -H "Content-Type: application/json" \
     -d '{
       "cdql": "find id(\"user_1\")"
     }'
```

### The Fast Path
Because we used `find id(...)`, the CDQL Planner automatically engages the **Fast Path**. It bypasses the WASM execution engine entirely and hits LMDB directly, returning the data in `0ms`.

---

## What's Next?
- Learn how data is stored across tiers in [Bits to Atoms](../architecture/bits-to-atoms.md).
- Dive deep into [Genomes & DNA Architecture](../genomes/dna-architecture.md).
