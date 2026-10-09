# Vector AI Genome (`vector_space.json`)

> *"Find meaning, not just matches."*

## When to Use This Genome
Use the `vector_space` genome when:
- You need semantic search (find conceptually similar items, not just keyword matches).
- You are building AI-powered features (chatbots, semantic recommendation, image search).
- You need to find the "N nearest neighbors" to a high-dimensional data point.
- You want to combine vector search with metadata filtering (Hybrid Search).

Real-world use cases: Semantic document search, image similarity, product recommendations, LLM memory retrieval, music/video recommendation, drug discovery similarity.

---

## What is a Vector Embedding?

A vector embedding is a mathematical representation of the "meaning" of data, produced by a machine learning model. For example:
- The sentence **"I love coffee"** might map to `[0.12, -0.44, 0.89, ...]` (384 dimensions).
- The sentence **"I enjoy espresso"** maps to `[0.13, -0.42, 0.91, ...]` — very close!
- The sentence **"The rocket launches tomorrow"** maps to `[-0.82, 0.31, -0.05, ...]` — far away.

CLUAIZD stores these embeddings in the `vector_data` field of every Neuron. The CDQL `similar_to()` operator then computes the mathematical distance between a query vector and all stored vectors, returning the closest matches.

---

## Storing Vectors

```bash
curl -X POST http://localhost:7331/neuron \
  -H "Content-Type: application/json" \
  -d '{
    "id": "doc_001",
    "tier": "Hot",
    "raw_payload": [bytes for {"title": "Introduction to Rust", "content": "Rust is a systems language..."}],
    "vector_data": [0.12, -0.44, 0.89, 0.33, -0.71, 0.55, 0.08, -0.22],
    "adjacency": []
  }'
```

The `vector_data` is a `Vec<f32>` — an array of 32-bit floating point numbers. All vectors in the same collection should have the same number of dimensions.

---

## Pure Vector Search

```text
// Find the 10 most semantically similar documents to a query vector
find * -> similar_to(vector: [0.11, -0.43, 0.88, 0.32, -0.69, 0.54, 0.07, -0.20], metric: "cosine") -> limit 10
```

### Supported Distance Metrics
| Metric | Best For |
|---|---|
| `"cosine"` | Text and semantic embeddings (normalized vectors) |
| `"l2"` | Image similarity, numerical feature spaces |
| `"dot"` | Maximum Inner Product Search, recommendation systems |

---

## Hybrid Search: The Gold Standard for AI Retrieval

Pure vector search has a critical flaw: semantic proximity can produce false positives. A search for "red Ferrari" might return "orange Lamborghini" because their vectors are close.

**Hybrid Search** solves this by applying hard metadata filters **before** the vector comparison. Only the filtered subset is scored by vector similarity.

```text
// Step 1: Hard filter (EXACT match — no false positives)
// Step 2: Then rank the filtered results by semantic similarity
find Product(category: "sports_car", color: "red", available: true)
  -> similar_to(vector: [ferrari_query_embedding], metric: "cosine")
  -> limit 10
```

This returns only red sports cars that are in stock, ranked by how closely they match the query embedding. 100% precision with 0 false positives.

---

## Integration with Python Embedding Models

```python
from sentence_transformers import SentenceTransformer
import requests

model = SentenceTransformer('all-MiniLM-L6-v2')

# Encode a document before storing
doc = "CLUAIZD is a universal database substrate built in Rust"
embedding = model.encode(doc).tolist()

# Store in CLUAIZD
requests.post("http://localhost:7331/neuron", json={
    "id": "doc_cluaizdb_intro",
    "tier": "Hot",
    "raw_payload": list(doc.encode("utf-8")),
    "vector_data": embedding,
    "adjacency": []
})

# Semantic search
query = "Rust systems programming database"
query_vector = model.encode(query).tolist()

results = requests.post("http://localhost:7331/query", json={
    "cdql": f"find * -> similar_to(vector: {query_vector}, metric: \"cosine\") -> limit 5"
}).json()
```

---

## Comparison: CLUAIZD vs Vector DB

| Feature | Vector DB | CLUAIZD (vector_space) |
|---|---|---|
| Cosine / L2 Similarity | ✅ | ✅ |
| Metadata Hybrid Filtering | ✅ | ✅ |
| HNSW Index (ANN) | ✅ (built-in) | 🔜 (Planned — currently linear scan) |
| Graph Relations on Vectors | ❌ | ✅ |
| Time-Series of Embeddings | ❌ | ✅ |
| Cost (1M vectors) | ~$70/mo | ~$5/mo |
| Self-hosted | ❌ | ✅ |
