# 🔌 HTTP / WebSocket API Reference (A-Z)

Welcome to the **cluaizdb API Reference**.
While CDQL is the query language, the actual data transmission occurs over ultra-fast HTTP/REST and WebSocket protocols. This ensures that cluaizdb can be integrated into any backend language (Node.js, Python, Go, Rust) without needing bulky proprietary drivers.

## API Reference Index

| Route / Protocol | Category | Description |
|---|---|---|
| [`Authorization`](./api/auth.md) | Security | Handshake mechanics using Ed25519 cryptographic signatures and JWTs.<hr>`Authorization: Bearer <TOKEN>` |
| [`POST /neuron`](./api/post_neuron.md) | Ingestion | Primary HTTP endpoint for inserting JSON payloads and vector embeddings into the WAL.<hr>`curl -X POST /neuron -d '...'` |
| [`ws:// /stream`](./api/ws_stream.md) | Traversal | Persistent full-duplex TCP socket for executing CDQL `find` queries and receiving asynchronous binary streams.<hr>`let socket = new WebSocket('ws://...')` |
