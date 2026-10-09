# 🗃️ Chapter 3: The 9 Paradigms Cookbook

Why pay for 4 different databases when one engine can mutate to act like all of them?

Below are quick configurations showing how to use Cluaizdb CNSDB as different types of databases just by changing your JSON payload and CDQL.

## 1. Document Store (MongoDB Style)

If you just want to store and retrieve JSON objects.

**Insert:**

```json
POST /neuron
{
  "raw_payload": "{\"user_id\": 101, \"name\": \"Aryan\", \"role\": \"admin\"}",
  "payload_type": "json",
  "vector_data": [],
  "model_creator_hash": "e3b0c4...",
  "dna": null
}
```

**Query (CDQL):**

```json
POST /query
{
  "cdql": "find json where role == 'admin'"
}
```

## 2. Vector Database (Pinecone / Qdrant Style)

If you are building an AI app (RAG) and need to find semantically similar text embeddings using Cosine Similarity.

**Insert:**

```json
POST /neuron
{
  "raw_payload": "Cluaizdb is a universal database engine.",
  "payload_type": "text",
  "vector_data": [0.12, -0.44, 0.89, ...], // Your 1536-dim OpenAI embedding here
  "model_creator_hash": "e3b0c4...",
  "dna": null
}
```

**Query (CDQL):**

```json
POST /query
{
  "cdql": "find similar using cosine_distance limit 5",
  "query_vector": [0.10, -0.40, 0.85, ...]
}
```

## 3. Graph Database (Neo4j Style)

If you want to map relationships between users (e.g., "Aryan follows Rahul").

**Insert Node 1 (Aryan):**
Gets UUID: `uuid-1`

**Insert Node 2 (Rahul):**
Gets UUID: `uuid-2`

**Create Relationship (Edge):**
You can use the CRISPR API to force a synaptic connection.

```json
POST /crispr/force/uuid-1
{
  "target_id": "uuid-2",
  "weight": 1.0
}
```

**Query (Graph Traversal):**

```bash
GET /graph/uuid-1/traverse?depth=2
```

## 4. Caching Engine (Redis Style)

If you want ephemeral data that auto-deletes after a TTL (Time-to-Live).
Just pass a WASM or Rhai script in the `dna` field that decays the synaptic weight over time!

```json
POST /neuron
{
  "raw_payload": "SessionToken_XYZ",
  "payload_type": "text",
  "vector_data": [],
  "model_creator_hash": "e3b0c4...",
  "dna": {
    "engine": "rhai",
    "on_read": "if config.ttl < now() { return delete_signal(); }",
    "parameters": {"ttl": 1780000000}
  }
}
```

---

_Explore the `genomes/` folder in the repository to see more complex pre-built database paradigms!_
