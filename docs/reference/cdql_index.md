# 📖 CDQL Master Reference (A-Z)

Welcome to the **cluaizdb Query Language (CDQL)** syntax dictionary.
This is a comprehensive, alphabetical reference of all CDQL keywords, operators, and functions used for querying the database engine.

| Keyword | Description |
|---|---|
| [`?` (Parameter Placeholder)](../query-language/parameterized_queries.md) | Halts text evaluation and binds the position directly to a raw binary payload supplied in the secondary channel. Bypasses parsing overhead.<hr>`insert into Memory(vector: ?)` |
| [`cosine_distance`](./cdql/cosine_distance.md) | Executes hardware-accelerated angular distance calculations for normalized vector embeddings, utilized primarily in NLP and semantic search workflows.<hr>`find similar using cosine_distance` |
| [`crispr`](./cdql/crispr.md) | Bypasses automated execution affordances to manually inject or sever direct 64-bit memory pointers (Graph Edges) between isolated records.<hr>`POST /crispr/force/uuid` |
| [`delete`](./cdql/delete.md) | Injects a tombstone marker into the B-Tree index, ensuring immediate exclusion from subsequent traversals while deferring physical memory compaction.<hr>`delete where id == '...'` |
| [`euclidean_distance`](./cdql/euclidean_distance.md) | Computes strict L2-Norm spatial point distances. Internal Min-Heap sorts evaluate squared distances to optimize CPU caching behavior.<hr>`find similar using euclidean_distance` |
| [`find`](./cdql/find.md) | Initiates a zero-copy LMDB read iteration. Bypasses IPC overhead to stream raw byte arrays directly from the OS page cache to the TCP buffer.<hr>`find *` |
| [`force`](./cdql/force.md) | Atomically writes a weighted directional memory pointer into a record's adjacency array, writing exclusively to the WAL for crash durability.<hr>`force/uuid-1 target_id='uuid-2'` |
| [`geo_near`](./cdql/geo_near.md) | Executes a **Haversine great-circle radius filter** against stored latitude/longitude coordinates. Neurons outside the target radius are excluded; survivors are sorted closest-first by inverse-distance score.<hr>`find * -> geo_near(lat: 28.6139, lon: 77.2090, radius_km: 50.0)` |
| [`gt`](./cdql/gt.md) | Evaluates strict boundary constraints. Utilizes zero-allocation byte casting to evaluate values before heap instantiation occurs.<hr>`where age > 18` |
| [`insert`](./cdql/insert.md) | Streams raw binary or JSON payloads directly to the Write-Ahead Log (WAL), mapping immediately into the primary MVCC B-Tree.<hr>`insert into Memory(id: '123')` |
| [`limit`](./cdql/limit.md) | Halts memory traversal iterators upon threshold completion, preventing backpressure on the network socket during massive sequential scans.<hr>`find * limit 10` |
| [`lt`](./cdql/lt.md) | Evaluates reverse-boundary constraints. For indexed fields, it invokes reverse B-Tree traversal and backward iterators to optimize sort ordering.<hr>`where age < 18` |
| [`range`](./cdql/range.md) | Executes an **inclusive field-level boundary scan**. Reads a named JSON key from each neuron's payload and retains records whose value falls within `[start, end]`. Supports numeric (f64) and lexicographic (UTF-8) modes.<hr>`find * -> range(field: "age", start: 18, end: 35)` |
| [`search`](./cdql/search.md) | Executes a **full-text / inverted-index query** against all neuron payloads. Tokenises the query and payload text, computes a match score, and returns results sorted by relevance. Supports exact and fuzzy (substring) matching.<hr>`find * -> search(query: "database engine", fuzzy: false)` |
| [`similar`](./cdql/similar.md) | Re-routes the query planner to the AI Vector subsystem, leveraging CPU SIMD instructions to perform parallel high-dimensional proximity scans.<hr>`find similar using cosine_distance` |
| [`update`](./cdql/update.md) | Performs MVCC-compliant, lock-free memory mutations. Allocates a new payload block and atomically swaps the B-Tree pointer to prevent read blocking.<hr>`update json where id == '...'` |
| [`where`](./cdql/where.md) | Applies byte-level evaluation logic. If unindexed, rejects records prior to JSON deserialization; if indexed, targets precise B-Tree nodes.<hr>`find json where status == 'active'` |


---

> **Looking for Advanced Mathematics? (`SUM`, `AVG`, `GROUP BY`)**
> 
> Because cluaizdb uses an **Absolute Flexible Architecture**, we do not hardcode hundreds of static mathematical keywords into the CDQL parser. If you need to perform complex aggregations, map-reduce, or custom math, you simply inject a **WASM Execution Affordance** (Script) into the record's `dna` field. The engine executes your custom logic directly inside the database memory space at zero-copy speeds.
