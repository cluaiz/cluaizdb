# `POST /neuron` Ingestion API

The `/neuron` HTTP route is the primary ingestion point for cluaizdb. It is designed to handle massive throughput bursts by immediately dumping network packets into the Write-Ahead Log (WAL).

## Architectural Execution

### 1. HTTP to WAL Pipelining
When an HTTP POST request hits the router, the Engine does not wait to deserialize the entire JSON body into the heap. Instead, the Tokio Async runtime buffers the incoming TCP network bytes and pipes them directly into the underlying storage block. This is known as **Direct I/O Pipelining**, which bypasses the CPU's L3 Cache entirely for maximum throughput.

### 2. Async B-Tree Commits
Once the WAL confirms the bytes are safely on the physical NVMe/SSD drive, the Engine returns an `HTTP 201 Created` response to the client. *After* the client receives the response, a background worker thread parses the payload and rebalances the B-Tree index asynchronously. This guarantees that ingestion speeds are never bottlenecked by indexing overhead.

## API Specification

**Endpoint:** `POST http://<HOST>:7331/neuron`

### Request Body Schema (JSON)

| Key | Type | Description |
|---|---|---|
| `raw_payload` | `string` | The actual data string. Can be JSON, XML, or base64. |
| `payload_type` | `string` | `json` or `text`. Informs the engine how to cast the bytes. |
| `vector_data` | `float[]` | (Optional) High-dimensional float array for similarity search. |
| `dna` | `object` | (Optional) Injected WASM or Rhai scripts for execution affordances. |

## Example Request

```bash
curl -X POST http://127.0.0.1:7331/neuron \
-H "Authorization: Bearer <TOKEN>" \
-H "Content-Type: application/json" \
-d '{
  "raw_payload": "{\"name\": \"System Admin\", \"access_level\": 99}",
  "payload_type": "json",
  "vector_data": [],
  "dna": null
}'
```
