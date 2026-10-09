# `cosine_distance` Reference

The `cosine_distance` algorithm is utilized during a `similar` vector search. It calculates the angular distance between two high-dimensional vectors, making it highly effective for measuring semantic similarity in Natural Language Processing (NLP).

## Syntax Rules

```text
find similar using cosine_distance [ limit <K> ]
```

---

## Architecture: How it works under the hood

Cosine Distance is fundamentally a measurement of orientation, ignoring the magnitude (length) of the vectors.

### 1. The Mathematical Computation
The formula used internally by the cluaizdb is:

$$ 1 - \frac{A \cdot B}{||A|| \cdot ||B||} $$

Where $A$ is the query vector and $B$ is the stored record vector. The result ranges from $0$ (identical orientation) to $2$ (opposite orientation).

### 2. SIMD Vectorization
The dot product ($A \cdot B$) and the magnitude computations are executed utilizing hardware-level Vector Processing Units (VPUs) or SIMD instructions. The engine batches multiple stored vectors into CPU registers, executing the floating-point arithmetic simultaneously rather than via sequential loops.

---

## Time Complexity

| Scenario               | Complexity   | Notes                                                           |
| :--------------------- | :----------- | :-------------------------------------------------------------- |
| **Vector Calculation** | **O(d)**     | Per vector, where `d` is the dimension (e.g., 1536 for OpenAI). |
| **Full Query**         | **O(N * d)** | Linear scan across all `N` records.                             |

---

## Examples

### 1. Standard Implementation

```json
{
  "cdql": "find similar using cosine_distance limit 10",
  "query_vector": [0.1, -0.4, 0.8, 0.2]
}
```
