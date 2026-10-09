# Sensory Shards: Isolation Architecture

> *"Don't let a firehose flood your water glass."*

## The Problem: Write-Heavy Streams Kill Read Performance

Imagine a BCI (Brain-Computer Interface) device recording neural signals at 256,000 writes/second. In a standard database, every write acquires a lock, competes with read queries, and puts pressure on the same buffer pool.

The result: your main application queries slow to a crawl while the sensor firehose hammers the database.

CLUAIZD solves this with **Sensory Shards** — completely isolated LMDB environments dedicated to high-throughput write streams.

---

## How Sharding Works in CLUAIZD

Each LMDB environment is a fully independent set of files on disk (`.mdb` + lock file). They share NO memory, NO write locks, and NO B-tree pages. A write to one shard cannot block a read in another.

```
out/
├── default_sandbox.mdb       ← Main cortical database (standard queries)
├── default_sandbox.mdb-lock
├── sensory_tissue.mdb        ← BCI / IoT write firehose (isolated)
├── sensory_tissue.mdb-lock
├── media_library.mdb         ← Blob storage shard (object_store genome)
└── media_library.mdb-lock
```

---

## Routing to a Specific Shard

Every CLUAIZD HTTP endpoint accepts a `tenant_id` parameter. The Shard Manager uses this to open (or reuse) the corresponding LMDB environment:

```bash
# Write to the main database (default shard)
curl -X POST http://localhost:7331/neuron \
  -d '{"id": "user_aryan", ...}'

# Write to the sensory shard — isolated from main DB
curl -X POST "http://localhost:7331/neuron?tenant_id=sensory_tissue" \
  -d '{"id": "bci_reading_001", ...}'

# Query only within the sensory shard
curl -X POST "http://localhost:7331/query?tenant_id=sensory_tissue" \
  -d '{"cdql": "find * -> range_scan(field: \"timestamp\", start: X, end: Y)"}'
```

---

## Lazy Shard Initialization

CLUAIZD does not require you to pre-create shards. The **Shard Manager** (`crates/server/src/shard_manager.rs`) uses lazy initialization:

1. First request with `tenant_id=sensory_tissue` → Shard Manager creates `out/sensory_tissue.mdb`.
2. All subsequent requests reuse the already-open LMDB environment (held in an `Arc<Mutex<HashMap>>` cache).
3. No configuration file changes needed. No restart required.

---

## Performance Isolation Guarantees

Because LMDB environments are completely independent:

| Operation | Affected Shards |
|---|---|
| BCI writes at 256,000/s to `sensory_tissue` | ❌ Does NOT affect `default_sandbox` reads |
| Heavy CDQL query scan on `default_sandbox` | ❌ Does NOT block `sensory_tissue` writes |
| ZSTD compression of cold `media_library` blob | ❌ Does NOT affect either database |

This is **hardware-level isolation** — each shard maps its own section of physical memory.

---

## Recommended Shard Strategy

| Use Case | Recommended Shard `tenant_id` |
|---|---|
| Main application data (users, products, orders) | `default_sandbox` (default) |
| BCI / EEG / EMG sensor streams | `sensory_tissue` |
| Video / Audio blob storage | `media_library` |
| AI agent memory & embeddings | `ai_memory` |
| Audit logs (append-only, immutable) | `audit_ledger` |
| Ephemeral cache (sessions, rate limits) | `cache_layer` |

---

## The C-FFI with Shards

For maximum performance on the `sensory_tissue` shard (bypassing TCP/HTTP entirely):

```c
// Open the sensory shard directly from C code
cluaizdbdHandle* handle = cluaizdb_open("./out/sensory_tissue", 8192);

// Write 256,000 BCI readings/second directly via memory-mapped I/O
for (int i = 0; i < 256000; i++) {
    char payload[256];
    snprintf(payload, sizeof(payload),
        "{\"electrode\": %d, \"value\": %.4f, \"ts\": %lld}",
        electrode_id[i], voltage[i], timestamp_ns[i]);
    cluaizdb_write(handle, "bci_stream", payload);
}

cluaizdb_close(handle);
```

This bypasses Axum, the HTTP stack, and the shard routing layer entirely. The C-FFI writes directly to LMDB's memory-mapped file.
