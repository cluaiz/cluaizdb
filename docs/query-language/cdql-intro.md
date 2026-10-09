# cluaizdb Neural Query Language (CDQL)

Because CLUAIZD is a 10-in-1 shape-shifting engine, a standard query language like SQL or GraphQL is insufficient. SQL cannot express Graph Traversals effectively, and GraphQL cannot express Vector Mathematics.

Enter **CDQL**.

CDQL is a universal, pipeline-based query language. It borrows the `->` pipeline concept from languages like Splunk SPL or Document Store Aggregation Framework, allowing you to stitch together entirely different database paradigms into a single query.

## The Execution Pipeline

A CDQL query consists of a **Root Selector** followed by an arbitrary number of **Pipeline Stages**.

```text
<Root Selector> -> <Stage 1> -> <Stage 2> ...
```

### 1. The Root Selector (`find`)
Every query must start by selecting a base set of Neurons.

```text
find User(status: "active") 
```
This scans the LMDB memory-maps for Neurons labeled `User` with an active status.

### 2. The Stages
Once the initial dataset is loaded into the working memory, you pipe it through different database engines.

```text
-> traverse(edge: "purchases")     // Graph Engine
-> join(target: "products")        // Relational Engine
-> aggregate(sum(price))           // Time-Series/SQL Engine
```

## The CDQL Planner (The Absolute Power Layer)
You might wonder: *How does the system calculate a Relational Join on a Graph Traverse without crashing?*

The secret is the **CDQL Planner**. Before your string is executed, it compiles into an Abstract Syntax Tree (AST). The Planner analyzes the AST and maps it to the DNA Genomes attached to the targeted Neurons.

If you attempt to `join()` Neurons that have the `document_store` genome (which lacks strict schemas), the Planner executes a dynamic hash-join in WASM memory. If you target `sql_strict` genomes, it executes a highly optimized C-level join.

> [!TIP]
> **O(1) Fast-Path:**
> If you start a query with `find id("some_uuid")`, the CDQL Planner recognizes this as a strict Key-Value request. It completely bypasses the WASM execution engine, fetching the data directly from the LMDB memory-map in `0ms`.
