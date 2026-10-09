# CDQL Performance Tuning

> *"A fast query is not an accident. It is a design decision."*

## Understanding the CDQL Execution Layers

Every CDQL query passes through 3 layers. Understanding which layer your query uses is the key to optimization.

```
Layer 1: Fast-Path (0ms)
  ├─ Triggered by: find id("exact_uuid")
  ├─ Skips: WASM eval, shard scan, all filters
  └─ Cost: 1 LMDB key lookup = < 0.1ms

Layer 2: Indexed Scan (1-5ms)
  ├─ Triggered by: find *(field: "exact_value") with indexed field
  ├─ Skips: Full-shard scan
  └─ Cost: B-tree descent + result set iteration

Layer 3: Full-Shard Scan (10ms - 10s)
  ├─ Triggered by: find * (no filters) or non-indexed field filters
  ├─ Scans: Every neuron in the shard
  └─ Cost: O(n) — proportional to shard size
```

---

## Rule 1: Always Use the Fast-Path for Single-Record Lookups

This is the single most impactful optimization in CLUAIZD.

```text
// ❌ BAD — triggers a full-shard scan
find User(user_id: "user_aryan_xyz123")  // 10ms+

// ✅ GOOD — triggers the Fast-Path
find id("user_aryan_xyz123")             // < 0.1ms
```

**When to use:** Any time you know the exact Neuron ID. This is appropriate for session lookups, resource fetches by primary key, and health checks.

---

## Rule 2: Filter Early, Compute Late

The CDQL Planner executes pipeline stages in the order you write them. A `similar_to()` computation is expensive — it computes dot products against every vector in the working set. Reduce the working set first.

```text
// ❌ BAD — runs vector search on ALL 1 million users
find User -> similar_to(vector: [...]) -> filter active: true -> limit 10

// ✅ GOOD — reduces to 50,000 active users FIRST, then vector search
find User(active: true, subscription: "pro") -> similar_to(vector: [...]) -> limit 10
```

---

## Rule 3: Use Tenant Shards to Avoid Cross-Contamination

If your main shard has 10 million neurons and you only ever query the 100,000 "product" neurons, a full shard scan is still scanning 10 million records.

**Solution:** Move product neurons to a dedicated shard:

```bash
# Write products to their own isolated shard
curl -X POST "http://localhost:7331/neuron?tenant_id=product_catalog" -d '{...}'

# Query only the 100,000 products shard — not 10 million main records
curl -X POST "http://localhost:7331/query?tenant_id=product_catalog" \
  -d '{"cdql": "find Product(category: \"electronics\") -> limit 20"}'
```

---

## Rule 4: Pre-Filter with Hard Constraints Before Soft Scores

"Hard constraints" (exact matches, range scans) are always cheaper than "soft scores" (vector similarity, full-text BM25 ranking).

```text
// ❌ BAD — BM25 text scoring on 1M records, then filter
find * -> search(query: "pizza", fuzzy: true) -> filter city: "Delhi" -> limit 10

// ✅ GOOD — narrow to Delhi's 10,000 restaurants first, then text score
find Restaurant(city: "Delhi", open_now: true) -> search(query: "pizza", fuzzy: true) -> limit 10
```

---

## Rule 5: Limit Deep Graph Traversals

A `traverse(hops: 1..10)` can exponentially explode your working set. A user with 500 friends has 500 nodes at hop 1. At hop 2, each of those 500 friends has 500 friends — 250,000 nodes. At hop 3: 125 million nodes.

Always cap traversals with a reasonable hop limit AND a `limit` clause:

```text
// ❌ DANGEROUS — exponential graph explosion
find id("user_alice") -> traverse(edge: "friends", hops: 1..10) -> limit 1000000

// ✅ SAFE — bounded traversal with early limit
find id("user_alice") -> traverse(edge: "friends", hops: 1..3) -> limit 100
```

---

## Rule 6: Use the C-FFI for High-Throughput Writes

HTTP overhead limits you to approximately 50,000 writes/second (TCP handshake + JSON parsing + Axum routing). The C-FFI writes directly to LMDB's memory-mapped file, achieving over 1,000,000 writes/second.

```python
# ❌ BAD for high throughput — HTTP adds ~20µs per write
for reading in sensor_readings:
    requests.post("http://localhost:7331/neuron", json=reading)  # 20µs per write

# ✅ GOOD — C-FFI adds ~1µs per write (20x faster)
import ctypes
cluaizdb = ctypes.CDLL("./libcluaizdb.so")
handle = cluaizdb.cluaizdbb_open(b"./out/sensory_tissue", 8192)
for reading in sensor_readings:
    cluaizdb.cluaizdbb_write(handle, b"bci_stream", json.dumps(reading).encode())
```

---

## Rule 7: Monitor with the `/ws/telemetry` Endpoint

Baseline your performance before optimizing. The `/ws/telemetry` endpoint exposes key metrics:

```bash
curl http://localhost:7331/ws/telemetry
```

```json
{
  "status": "ok",
  "shards_open": 3,
  "total_neurons_hot": 145230,
  "total_neurons_warm": 82100,
  "total_neurons_cold": 1203400,
  "ram_used_percent": 42.3,
  "wal_entries_pending": 0,
  "dreamer_last_cycle_ms": 12
}
```

- If `ram_used_percent` is above 85%, the Dreamer is aggressively demoting neurons. Consider adding more RAM or reducing data retention.
- If `dreamer_last_cycle_ms` is above 1000ms, your shard is too large. Split into multiple tenant shards.
- If `wal_entries_pending` is non-zero after 30 seconds, the LMDB writer is falling behind the write rate.

---

## Query Performance Benchmarks (Reference)

| Query Type | Typical Latency | Bottleneck |
|---|---|---|
| `find id("x")` (Fast-Path) | `< 0.1ms` | LMDB mmap |
| `find *(field: exact)` (Shard scan, 100K neurons) | `1-5ms` | LMDB B-tree |
| `find * -> similar_to()` (Vector, 100K neurons, 384-dim) | `10-50ms` | Float32 dot products |
| `find * -> traverse(hops: 1..3)` (Graph, 10K edges) | `5-20ms` | Edge pointer dereference |
| `find * -> search(fuzzy: true)` (Full-text, 100K docs) | `10-30ms` | Inverted index lookup |
| `find * -> time_window("5m") -> aggregate()` | `20-100ms` | In-memory bucket sort |
