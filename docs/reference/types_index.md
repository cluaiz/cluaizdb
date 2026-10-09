# 📦 Data Types Reference (A-Z)

Welcome to the **cluaizdb Data Types Reference**.
Understanding how the Engine serializes and stores specific data structures in the LMDB memory map is crucial for optimizing your schemas and maximizing query performance.

## Types Index

| Type | Category | Description |
|---|---|---|
| [`binary`](./types/binary.md) | Raw | Unstructured binary data streams stored directly in the LMDB database block. |
| [`code`](./types/code.md) | Raw | Raw text buffers representing source code snippets or scripts, optimized for parsing. |
| [`json`](./types/json.md) | Structured | Schemaless, deeply nested JSON objects mapped internally via binary representation.<hr>`{"role": "admin", "age": 25}` |
| [`text`](./types/text.md) | Structured | UTF-8 encoded text buffers, indexed for tokenization and fast keyword searching. |
| [`vector`](./types/vector.md) | AI/ML | High-dimensional arrays of 32-bit floats (`f32`), optimized for SIMD distances.<hr>`[0.1, -0.9, 0.45]` |
| [`voltage_stream`](./types/voltage_stream.md) | IoT/Robotics | Compact arrays of telemetry readings, optimized for time-series aggregation. |
