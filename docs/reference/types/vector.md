# `vector` Data Type

In cluaizdb, Vectors are high-dimensional arrays of 32-bit floating-point numbers (`f32`). They are the core data structure enabling AI-driven semantic similarity searches.

## Architectural Storage

### Contiguous Memory Allocation
When a vector array (e.g., `[0.12, -0.44, 0.89]`) is submitted via the `POST /neuron` API, the Engine does not store it as a standard JSON Array. A JSON Array string (`"[0.12, -0.44]"`) occupies roughly 14 bytes per number due to ASCII string encoding.

Instead, the cluaizdb Engine serializes the array into a contiguous, binary C-style array of `f32` primitives. This reduces the memory footprint to exactly 4 bytes per number. A 1536-dimensional OpenAI embedding consumes precisely 6,144 bytes on disk.

### Memory Alignment for SIMD
The vector bytes are heavily padded and memory-aligned to 32-byte or 64-byte boundaries on the physical disk. This ensures that when the Engine executes a `find similar` query, the memory addresses map perfectly into the CPU's L1 cache, allowing AVX-512 SIMD instructions to load chunks of the vector into the hardware registers with zero CPU stall cycles.

## Supported Limitations

| Parameter | Engine Limit | Notes |
| :--- | :--- | :--- |
| **Max Dimensions** | **32,768** | Exceeding this limit will cause the payload to be rejected during the WAL write phase. |
| **Precision** | **32-bit Float** | 64-bit floats (`f64`) are truncated to `f32` during ingestion to maximize SIMD throughput. |
