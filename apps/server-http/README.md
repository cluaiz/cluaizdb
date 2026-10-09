# 🌐 apps/server-http: Sovereign Edge Server & MCP Host
> **Module:** Axum HTTP 2.0 / REST & Model Context Protocol (MCP) Daemon  
> **Recall Target:** Sub-millisecond JSON API + Native LLM Tool Dispatch  
> **Clients:** Claude Desktop, AI Agents, Python SDK, Web Applications

---

## 🎯 1. Purpose
`apps/server-http` is the official edge server binary for CluaizDB Kernel v2. It coordinates the underlying sovereign engines:
1. `kernel-core/substrate`: In-process point queries (< 5 µs).
2. `modules/group-commit`: SSD ACID durable batch pipeline (~121 µs).
3. `modules/index-engine/mvhsnw-vector`: Cosine/L2 ANN vector search.
4. `runtimes/wasmtime-sandbox`: Fuel-metered programmable WASM genomes.
5. `api/mcp-spec`: Claude-native Model Context Protocol dispatcher.

---

## 🛠️ 2. API Endpoints

### 📡 System & Health
- `GET /health` -> `{ "status": "ok", "version": "0.2.0", "engine": "cluaizdb" }`
- `GET /stats` -> Real-time physical record count, page usage, and commit latency telemetry.

### 📝 Records (< 5 µs Recall & Durable Group Commit)
- `POST /v1/records` -> Durable async write staged to group-commit.
- `GET /v1/records/:collection_id/:record_id` -> Point lookup directly from SSD page-cache.
- `DELETE /v1/records/:collection_id/:record_id` -> Atomically removes record.

### 🔍 Vector Search
- `POST /v1/vectors/search` -> Sub-millisecond approximate nearest neighbor query.

### 🤖 Model Context Protocol (MCP)
- `POST /mcp` -> Standard JSON-RPC 2.0 payload processed by `McpHandler`.

---

## 🏛️ 3. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- Expose network sockets, parse HTTP requests, route JSON payloads.
- Coordinate dependencies via shared `AppState`.

### ❌ What this crate CANNOT do:
- Contain raw LMDB pointer operations (strictly calls `cluaizdb-substrate`).
- Depend on frontend UI code.
