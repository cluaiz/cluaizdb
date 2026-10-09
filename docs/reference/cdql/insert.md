# `insert` Command Reference

In the cluaizdb ecosystem, data ingestion (insertion) bypasses the CDQL text parser entirely. Instead of a text-based `INSERT INTO` command, record creation is handled via high-throughput HTTP POST routes or WebSocket streams to maximize binary ingestion speed.

## Syntax & Routing

To insert a new record, clients transmit payloads directly to the `/neuron` endpoint.

```http
POST /neuron
Content-Type: application/json
```

### Payload Structure

| Field          | Type           | Description                                                                                 |
| :------------- | :------------- | :------------------------------------------------------------------------------------------ |
| `raw_payload`  | `string`       | The actual data to be stored. Can be raw text, JSON strings, or base64 encoded binary.      |
| `payload_type` | `string`       | Hints the engine about the data structure (e.g., `"json"`, `"text"`).                       |
| `vector_data`  | `array[float]` | (Optional) High-dimensional embeddings for AI similarity search.                            |
| `dna`          | `object`       | (Optional) WASM or Rhai execution scripts attached to the record for active event handling. |

---

## Architecture: How it works under the hood

Insertion in cluaizdb is designed to handle millions of writes per second without blocking read operations.

### 1. The Write-Ahead Log (WAL)
Upon receiving the payload, the compute node immediately appends the serialized byte array to an append-only Write-Ahead Log (WAL) on the physical disk. This ensures `fsync` durability—if the server crashes in the next microsecond, no data is lost.

### 2. LMDB Append & B-Tree Rebalancing
After the WAL is secured, the data is mapped into the primary Zero-Copy LMDB environment. If indices are present, the internal B-Tree structures are updated. Cluaizdb utilizes LMDB's MVCC (Multi-Version Concurrency Control) so that concurrent `find` queries never lock during an `insert`.

### 3. Execution Affordance Trigger (DNA)
If the payload contains a `dna` script (e.g., an `on_write` trigger), the WASM runtime is invoked immediately after memory mapping, allowing the record to actively mutate other records or trigger WebHooks.

---

## Time Complexity

| Operation           | Complexity   | Notes                                                                                    |
| :------------------ | :----------- | :--------------------------------------------------------------------------------------- |
| **Standard Insert** | **O(1)**     | Standard append operations are constant time.                                            |
| **Indexed Insert**  | **O(log N)** | Rebalancing the secondary B-Tree indices takes logarithmic time based on the shard size. |

---

## Examples

### 1. Standard JSON Ingestion

```bash
curl -X POST http://localhost:7331/neuron \
-H "Content-Type: application/json" \
-d '{
  "raw_payload": "{\"user_id\": 101, \"name\": \"System Admin\"}",
  "payload_type": "json",
  "vector_data": [],
  "dna": null
}'
```

### 2. Vector Embedding Ingestion

Inserting data pre-computed by a vector embedding model (e.g., OpenAI `text-embedding-ada-002`) for future `cosine_distance` searches.

```bash
curl -X POST http://localhost:7331/neuron \
-H "Content-Type: application/json" \
-d '{
  "raw_payload": "The system architecture leverages zero-copy memory.",
  "payload_type": "text",
  "vector_data": [0.012, -0.044, 0.089, ...],
  "dna": null
}'
```
