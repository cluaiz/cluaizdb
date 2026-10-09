# `avg` Aggregation Reference

The `avg` (Average) affordance calculates the arithmetic mean of a specified numerical field across a dataset. 

## Architectural Execution

### 1. Dual-Register Accumulation
Calculating an average requires two running variables: the total `sum` and the total `count`. Rather than iterating over the memory map twice, the cluaizdb engine utilizes dual-register accumulation. During the SIMD vector folding phase, one register accumulates the numerical values, while a parallel instruction counts the valid, non-null entries. 

### 2. Floating-Point Precision Constraints
Because JSON payloads often mix integers and floating-point numbers inconsistently, the `avg` affordance enforces a strict cast to `f64` (64-bit float) at the byte level before accumulation. This prevents precision loss or integer overflow during massive aggregations.

## Time Complexity

| Operation | Complexity | Notes |
| :--- | :--- | :--- |
| **Averaging** | **O(N / v)** | Operates at the exact same SIMD speed as `sum`, since the count operation is pipelined alongside the addition. |

## Example: Executing an Average

```rust
fn execute_query(ctx) {
    // 1. Fetch relevant records
    let records = ctx.find_json("where status == 'delivered'");
    
    // 2. Invoke the average affordance
    let avg_delivery_time = dna.aggregate("avg", records, "delivery_seconds");
    
    // 3. Return the calculated average
    ctx.return_result(avg_delivery_time);
}
```
