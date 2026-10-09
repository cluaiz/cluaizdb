# 🧬 DNA Execution Affordances (A-Z)

Welcome to the **cluaizdb DNA Reference**. 
Because cluaizdb is built on an **Absolute Flexible Architecture**, it intentionally avoids bloating the CDQL parser with hundreds of hardcoded math functions, triggers, or map-reduce commands. 

Instead, developers inject **Execution Affordances (DNA)** directly into records using one of the **4 Execution Engines (CDQL, WASM, Auto-WASM, Rhai)**. The Database Engine executes these scripts at zero-copy speeds in a secure sandbox.

For a deep dive into writing strict-typed `0.05ms` code, check out the [Auto-WASM Guide](../genomes/auto-wasm-guide.md).

## Reference Index

| Keyword / Hook | Category | Description |
|---|---|---|
| [`avg`](./dna/avg.md) | Aggregation | Computing mathematical averages across shards using WASM accumulators.<hr>`dna.aggregate("avg", field)` |
| [`ctx.abort`](./dna/ctx_abort.md) | Engine API | Safely aborts the current write transaction and triggers a WAL rollback.<hr>`ctx.abort("Validation failed")` |
| [`ctx.fetch`](./dna/ctx_fetch.md) | Engine API | Issues a zero-copy pointer dereference to fetch another record during execution.<hr>`let parent = ctx.fetch(uuid);` |
| [`ctx.time`](./dna/ctx_time.md) | Engine API | Retrieves the engine's internal Epoch timestamp, circumventing OS syscall overhead.<hr>`let now = ctx.time();` |
| [`group_by`](./dna/group_by.md) | Aggregation | Executing multi-dimensional bucketing operations via memory-mapped HashMaps.<hr>`dna.bucket_by(field)` |
| [`on_delete`](./dna/on_delete.md) | Event Hook | Synchronous hook invoked immediately before tombstone insertion.<hr>`fn on_delete(ctx)` |
| [`on_lifecycle`](./dna/on_lifecycle.md) | Event Hook | Synchronous hook invoked during background dreamer compaction for garbage collection.<hr>`fn on_lifecycle(ctx)` |
| [`on_read`](./dna/on_read.md) | Event Hook | Synchronous hook invoked during a `find` traversal, enabling on-the-fly redaction or computation.<hr>`fn on_read(ctx)` |
| [`on_write`](./dna/on_write.md) | Event Hook | Synchronous hook invoked before WAL commit, enabling data validation or cascading mutations.<hr>`fn on_write(ctx)` |
| [`sum`](./dna/sum.md) | Aggregation | High-performance numerical accumulation using SIMD vector folding.<hr>`dna.aggregate("sum", field)` |
| [`sync_write`](./dna/sync_write.md) | Engine Config | Dynamic OS page-cache vs hardware block-level durability control on a per-genome basis.<hr>`"sync_write": "lite"` |

