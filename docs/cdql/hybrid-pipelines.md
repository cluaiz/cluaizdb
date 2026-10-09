# Hybrid Pipelines: The Cross-Genome Synthesis Engine

> *"Why use 4 databases when one pipeline can do everything?"*

## The Core Insight

Most developers think about databases as silos. You query Relational DB for structured data. You query Vector DB for vectors. You query Graph DB for relationships. Then you write Python code to merge the results.

CLUAIZD breaks this paradigm. Because Genomes are dynamic policies attached to Neurons rather than hardcoded engine modes, a single CDQL pipeline can transparently invoke multiple paradigm engines in sequence — all within the same Rust memory process, zero network hops between steps.

---

## How Cross-Genome Synthesis Works (Under the Hood)

When the CDQL Planner receives a pipeline like:
```text
find Product(color: "red") -> traverse(edge: "bought_by") -> similar_to(vector: [...])
```

It does NOT route this to three separate services. Instead:

```
Step 1: find Product(color: "red")
  └─ Scans LMDB neurons with label "Product"
  └─ Evaluates payload JSON via json_filter (no WASM needed)
  └─ Working set: 1,000 red products

Step 2: -> traverse(edge: "bought_by")
  └─ For each of the 1,000 products, follows adjacency pointers
  └─ Index-free: O(1) pointer dereference per edge
  └─ Working set: 8,400 users who bought red products

Step 3: -> similar_to(vector: [...], metric: "cosine")
  └─ Passes the vector to the `cluaizdb-index-mvhsnw` HNSW Engine
  └─ Graph-based approximate nearest neighbor search bypasses flat scans
  └─ Returns top-K by cosine score in <1ms
  └─ Working set: 10 most relevant users
```

All three steps happen in the same Rust thread. No serialization. No deserialization. No TCP. The only I/O is reading from LMDB's memory-mapped file and the MV-HNSW index in RAM.

---

## The 7 Killer Hybrid Patterns

### Pattern 1: Geo + Full-Text + Rating Filter
**Use Case:** "Find highly-rated Italian restaurants within 3km that serve 'risotto'"

```text
find Restaurant(cuisine: "Italian", rating >= 4.5, open_now: true)
  -> geo_near(lat: 28.6139, lon: 77.2090, radius: "3km")
  -> search(fields: {menu: 2.0, name: 3.0}, query: "risotto", fuzzy: true)
  -> sort_by_score()
  -> limit 10
```

**Traditional approach:** PostGIS query → Search Engine query → Python merge → sort. 3 API calls, 2-3 seconds.
**CLUAIZD:** 1 pipeline, ~15ms.

---

### Pattern 2: Graph + Vector AI (People You May Know + Interests)
**Use Case:** LinkedIn-style recommendation: friends of friends who share your professional interests.

```text
find id("user_aryan")
  -> traverse(edge: "connected_to", hops: 2..2)  // Friends of friends ONLY
  -> filter already_connected: false               // Exclude direct connections
  -> similar_to(vector: [my_professional_embedding], metric: "cosine")
  -> sort_by_score()
  -> limit 20
```

---

### Pattern 3: Time-Series + Anomaly Detection (Vector)
**Use Case:** Find sensor readings that are both recent AND statistically anomalous (far from normal baseline).

```text
find Sensor(building: "DataCenter_A")
  -> filter time between "now-30m" and "now"
  -> time_window(size: "5m")
  -> aggregate(avg(temperature))
  -> similar_to(vector: [normal_baseline_vector], metric: "l2")
  -> sort_by_score(ascending: false)  // MOST DIFFERENT first
  -> limit 5
```

---

### Pattern 4: Document + Graph + Relational (E-Commerce Order Pipeline)
**Use Case:** Find pending orders from high-value customers, join with product details, check if any product is flagged for review.

```text
find Order(status: "pending", created_after: "now-24h")
  -> join(target: "User", on: "user_id == target.id", type: "inner")
  -> filter target.lifetime_value_usd > 1000
  -> join(target: "Product", on: "product_id == target.id", type: "inner")
  -> filter target.review_flagged: false
  -> sort_by("created_at", asc: false)
  -> limit 50
```

---

### Pattern 5: Full-Text + Vector (Hybrid Semantic Search)
**Use Case:** Search a documentation knowledge base where exact keyword AND semantic meaning both matter.

```text
find Article(published: true, language: "en")
  -> search(query: "database performance", fuzzy: true)   // Keyword relevance
  -> similar_to(vector: [query_embedding], metric: "cosine")  // Semantic relevance  
  -> sort_by_score()
  -> limit 10
```

This gives you the best of both worlds: BM25 text relevance AND semantic similarity, combined into a single score.

---

### Pattern 6: Geo + Graph (Hyperlocal Social Network)
**Use Case:** Find local events created by people within 3 degrees of connection, happening within 10km.

```text
find id("user_me")
  -> traverse(edge: "friends", hops: 1..3)
  -> filter created_events > 0
  -> geo_near(lat: 28.6139, lon: 77.2090, radius: "10km")
  -> filter event_date > "now"
  -> sort_by_distance(to: [28.6139, 77.2090])
  -> limit 20
```

---

### Pattern 7: Time-Series + Blob (Media + Telemetry Co-located)
**Use Case:** Find the video recording from a specific drone flight, retrieve only the telemetry summary and the first 10 seconds of footage.

```text
// Step 1: Get the telemetry metadata
find FlightLog(drone_id: "drone_007")
  -> filter time between "2026-06-07T10:00:00Z" and "2026-06-07T10:30:00Z"
  -> aggregate(avg(altitude_m), max(speed_kmh))

// Step 2: Stream just the first 10 seconds of the raw video
find id("flight_video_007_2026-06-07")
  -> stream(bytes: 0..40000000)  // First ~40MB = ~10 seconds of 4K
```

---

## Performance Characteristics of Hybrid Pipelines

| Pipeline Complexity | Typical Latency | Bottleneck |
|---|---|---|
| Single filter | 1-5ms | LMDB scan |
| Filter + traverse (1 hop) | <1ms | Graph Engine Adjacency lookup |
| Vector Search (HNSW) | <1ms | MV-HNSW memory latency |
| Filter + geo + text + vector | 5-20ms | Compute Engines (Pipeline reduction) |
| Time-series window + aggregate | <2ms | Gorilla Decompression & Bucketing |

> [!TIP]
> **Pipeline order is execution order.** Always put the fastest, most selective operation first. The working set shrinks at each step, making all subsequent operations cheaper.
> 
> **Rule of thumb:** `find(filter) → geo → text → vector` — from most selective to least selective.
