# `sum` Aggregation Reference

In traditional SQL databases, `SUM(field)` is a hardcoded parser keyword. In cluaizdb, `sum` is simply an imported WASM execution affordance that folds values across the LMDB memory map.

## Architectural Execution

### 1. Vector Folding (SIMD Accumulation)
When the `dna.aggregate("sum", field)` instruction is dispatched across a dataset, the cluaizdb engine does not pull JSON objects into the heap sequentially. Instead, it isolates the numerical byte arrays of the targeted field and feeds them directly into the CPU's Vector Processing Registers. This allows it to fold (sum) up to 16 or 32 integers simultaneously per clock cycle using AVX2 or AVX-512 intrinsics.

### 2. Lock-Free Map-Reduce
If the dataset spans multiple internal shards, the cluaizdb engine spawns lightweight Green Threads (via Tokio). Each thread computes the localized sum of its shard's memory map simultaneously. Finally, a master reduction step adds the thread outputs together, yielding massive parallelization without Mutex locks.

## Time Complexity

| Operation | Complexity | Notes |
| :--- | :--- | :--- |
| **Summation** | **O(N / v)** | Where `N` is the number of records and `v` is the SIMD vector width (e.g., 16). Massively faster than standard `O(N)`. |

## Example: Executing a Summation

Because `sum` is an affordance, it is executed via a query script rather than raw CDQL text.

```rust
fn execute_query(ctx) {
    // 1. Traverse and filter the memory map (Zero-Copy)
    let records = ctx.find_json("where department == 'engineering'");
    
    // 2. Invoke the WASM summation affordance on the 'salary' field
    let total_payroll = dna.aggregate("sum", records, "salary");
    
    // 3. Return the single floating-point number
    ctx.return_result(total_payroll);
}
```
