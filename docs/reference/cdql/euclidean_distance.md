# `euclidean_distance` Reference

The `euclidean_distance` algorithm calculates the straight-line distance between two points in high-dimensional space. Unlike cosine distance, it accounts for both the magnitude and orientation of the vectors.

## Syntax Rules

```text
find similar using euclidean_distance [ limit <K> ]
```

---

## Architecture: How it works under the hood

Euclidean distance is computationally lighter than cosine distance but behaves differently with non-normalized data.

### 1. The Mathematical Computation
The formula used internally by the cluaizdb engine is the standard L2 Norm:

$$ \sqrt{\sum_{i=1}^{n} (A_i - B_i)^2} $$

### 2. Squared Optimization
During the internal Min-Heap sort to find the top `K` results, the cluaizdb engine optimizes CPU cycles by omitting the final square root operation ($\sqrt{x}$). Since the square root is a monotonic function, comparing the squared distances yields the exact same ranking, saving hundreds of thousands of expensive CPU instructions during a massive scan.

---

## Time Complexity

| Scenario               | Complexity   | Notes                                   |
| :--------------------- | :----------- | :-------------------------------------- |
| **Vector Calculation** | **O(d)**     | Per vector, where `d` is the dimension. |
| **Full Query**         | **O(N * d)** | Linear scan across all `N` records.     |

---

## Examples

### 1. Spatial / Exact Data Matching

```json
{
  "cdql": "find similar using euclidean_distance limit 3",
  "query_vector": [1.0, 5.5, 9.2, 3.1]
}
```
