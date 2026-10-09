# CDQL Rosetta Stone — Universal Query Cheatsheet

> *"दुनिया के किसी भी database से आओ — 10 minutes में CDQL सीख लो।"*

यह single page हमारे entire database universe का निचोड़ है। Document Store developer हो, Relational DB veteran हो, या Graph DB specialist — अपनी पुरानी भाषा को नीचे दिए गए table में ढूंढो और उसका CDQL equivalent तुरंत मिल जाएगा।

---

## 🗺️ The Complete Translation Matrix

### 1. Relational SQL (Relational DB / Relational DB)

| SQL Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `SELECT * FROM users` | `find User` | Full shard scan |
| `SELECT * FROM users WHERE age > 18` | `find User -> filter age > 18` | WASM filter on payload |
| `SELECT * FROM users LIMIT 10` | `find User -> limit 10` | Truncate working set |
| `SELECT * FROM users ORDER BY name ASC` | `find User -> sort_by("name", asc: true)` | In-memory sort |
| `SELECT name, email FROM users` | `find User -> project(keep: ["name", "email"])` | Payload reshape |
| `SELECT u.*, o.total FROM users u JOIN orders o ON u.id = o.user_id` | `find User -> join(target: "Order", on: "id == target.user_id", type: "inner")` | In-memory hash join |
| `SELECT dept, COUNT(*) FROM users GROUP BY dept` | `find User -> group_by("dept") -> aggregate(count())` | Bucket grouping |
| `SELECT dept, SUM(salary) FROM users GROUP BY dept` | `find User -> group_by("dept") -> aggregate(sum(salary))` | Numeric aggregation |
| `INSERT INTO users VALUES (...)` | `POST /neuron` with JSON payload | LMDB + WAL write |
| `SELECT * FROM users WHERE id = 'abc'` | `find id("abc")` | **0ms Fast-Path — LMDB direct** |

---

### 2. Document NoSQL (Document Store)

| Document Store Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `db.users.find({status: "active"})` | `find User(status: "active")` | JSON field filter |
| `db.users.find({age: {$gt: 18}})` | `find User -> filter age > 18` | Numeric compare |
| `db.users.find({age: {$gte: 18, $lte: 65}})` | `find User -> filter age >= 18 -> filter age <= 65` | Range filter |
| `db.users.find({tags: {$in: ["premium"]}})` | `find User -> filter tags contains "premium"` | Array contains |
| `db.posts.aggregate([{$unwind: "$tags"}])` | `find Post -> unwind("tags")` | Array expansion |
| `db.posts.aggregate([{$project: {title: 1, author: 1}}])` | `find Post -> project(keep: ["title", "author"])` | Field selection |
| `db.users.findById("abc123")` | `find id("abc123")` | **0ms Fast-Path** |
| `db.users.find().sort({name: 1}).limit(20)` | `find User -> sort_by("name", asc: true) -> limit 20` | Sort + truncate |

---

### 3. Graph Database (Graph DB Cypher)

| Cypher Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `MATCH (u:User {id: "alice"}) RETURN u` | `find id("user_alice")` | Fast-Path |
| `MATCH (u:User)-[:FRIENDS]->(f) RETURN f` | `find User -> traverse(edge: "friends", hops: 1..1)` | Index-free adjacency |
| `MATCH (u:User)-[:FRIENDS*1..5]->(f) RETURN f` | `find User -> traverse(edge: "friends", hops: 1..5)` | Multi-hop traversal |
| `MATCH (u:User)-[:FRIENDS*]->(f) WHERE f.city = "Delhi"` | `find User -> traverse(edge: "friends", hops: 1..5) -> filter city: "Delhi"` | Traverse + filter |
| `MATCH p=shortestPath((a)-[*]->(b)) RETURN p` | `find id("a") -> shortest_path(to: "b")` | Dijkstra (planned) |
| `MATCH (a)-[r:BUYS {weight: > 0.8}]->(b)` | `-> traverse(edge: "buys", min_weight: 0.8, hops: 1..1)` | Weighted edge filter |

---

### 4. Vector / AI Database (Vector DB / Vector DB)

| Vector DB Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `index.query(vector=[...], top_k=10)` | `find * -> similar_to(vector: [...], metric: "cosine") -> limit 10` | Float32 dot products |
| `index.query(vector=[...], filter={"color": "red"})` | `find Product(color: "red") -> similar_to(vector: [...])` | **Hybrid Search** |
| `index.query(vector=[...], metric="euclidean")` | `find * -> similar_to(vector: [...], metric: "l2")` | L2 distance |
| `index.query(vector=[...], metric="dotproduct")` | `find * -> similar_to(vector: [...], metric: "dot")` | Dot product |

---

### 5. Time-Series (Time-Series DB / Time-Series DB)

| Time-Series DB / SQL Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `SELECT * FROM sensors WHERE time > now() - 1h` | `find Sensor -> filter time between "now-1h" and "now"` | Time range filter |
| `SELECT mean(value) FROM sensors GROUP BY time(5m)` | `find Sensor -> time_window(size: "5m") -> aggregate(avg(value))` | Time bucket grouping |
| `SELECT max(value) FROM sensors GROUP BY time(1h)` | `find Sensor -> time_window(size: "1h") -> aggregate(max(value))` | Max aggregation |

---

### 6. Key-Value (In-Memory Cache)

| In-Memory Cache Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `GET key` | `find id("key")` | **0ms Fast-Path, LMDB direct** |
| `SET key value EX 600` | `POST /neuron` with `on_lifecycle` TTL genome | Dreamer eviction |
| `KEYS user_*` | `find User -> limit 1000` | Prefix scan (pattern planned) |
| `INCR counter` | `update id("counter") -> set(count: count + 1)` | Atomic update |

---

### 7. Wide-Column (Wide-Column DB CDQL)

| CDQL Command | CDQL Equivalent | Under the Hood |
|---|---|---|
| `SELECT * FROM events WHERE partition = 'sensor_x'` | `find Event(partition: "sensor_x")` | Partition filter |
| `SELECT * FROM events WHERE ts BETWEEN X AND Y` | `find Event -> range_scan(field: "ts", start: X, end: Y)` | Ordered scan |
| `UPDATE t SET val=2 WHERE id='x' IF val=1` | `update id("x") -> set(val: 2) -> if(val == 1)` | Compare-And-Set |

---

### 8. Full-Text Search (Search Engine)

| Search Engine Query | CDQL Equivalent | Under the Hood |
|---|---|---|
| `{ "match": { "content": "pizza" } }` | `find * -> search(query: "pizza", fuzzy: false)` | Inverted index lookup |
| `{ "fuzzy": { "content": { "value": "pizaa" } } }` | `find * -> search(query: "pizaa", fuzzy: true)` | Levenshtein distance |
| `{ "multi_match": { "fields": ["title^3", "body"] } }` | `find * -> search(fields: {title: 3.0, body: 1.0}, query: "rust db")` | BM25 field boosting |
| Sort by relevance | `-> sort_by_score()` | BM25 score sort |

---

### 9. Geo-Spatial (PostGIS / Document Store Geo)

| Geo Query | CDQL Equivalent | Under the Hood |
|---|---|---|
| `ST_DWithin(point, center, 5000)` | `find * -> geo_near(lat: 28.6, lon: 77.2, radius: "5km")` | Haversine formula |
| `ST_Contains(polygon, point)` | `find * -> geo_within(polygon: [[...], [...]])` | Polygon containment |
| `ORDER BY ST_Distance(point, center)` | `-> sort_by_distance(to: [28.6, 77.2])` | Distance sort |

---

### 10. Blob / Object Storage (Object Store / Object Store)

| Object Store Operation | CDQL Equivalent | Under the Hood |
|---|---|---|
| `GET object` (full) | `find id("uuid")` | ZSTD decompress + return |
| `GET object` (Range: bytes=0-1048576) | `find id("uuid") -> stream(bytes: 0..1048576)` | Byte-range streaming |
| `HEAD object` (metadata only) | `find id("uuid") -> pluck_metadata()` | Shell read, no payload |

---

## ⚡ Quick-Start: The 5 Most Common Patterns

```text
// 1. Fast single-record fetch (0ms — use this for anything by ID)
find id("your_neuron_id")

// 2. Filter + limit (the bread and butter)
find User(status: "active") -> filter age > 18 -> limit 50

// 3. Graph traversal (social networks, recommendations)
find id("user_alice") -> traverse(edge: "friends", hops: 1..2) -> limit 20

// 4. Hybrid AI search (vector + metadata — the killer feature)
find Product(category: "electronics", in_stock: true)
  -> similar_to(vector: [0.12, -0.44, 0.89], metric: "cosine")
  -> limit 10

// 5. Time-series aggregation (IoT, metrics)
find Sensor(id: "temp_01")
  -> filter time between "now-1h" and "now"
  -> time_window(size: "5m")
  -> aggregate(avg(value))
```

---

## 🚨 Common Mistakes & How to Avoid Them

| ❌ Mistake | ✅ Fix |
|---|---|
| `find User(id: "abc")` — slow scan | `find id("abc")` — 0ms Fast-Path |
| `find * -> similar_to() -> filter active: true` | `find User(active: true) -> similar_to()` — filter first! |
| `traverse(hops: 1..10)` without limit | `traverse(hops: 1..3) -> limit 100` — always cap traversal |
| HTTP for 256K IoT writes/sec | Use C-FFI `cluaizdb_write()` — 20x faster |
