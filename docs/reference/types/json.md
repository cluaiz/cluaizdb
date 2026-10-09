# `json` Data Type

Cluaizdb allows developers to ingest heavily nested, schemaless JSON payloads via the `raw_payload` attribute, providing the flexibility of document stores like MongoDB.

## Architectural Storage

### Binary BSON/MsgPack Transformation
While the client submits data as UTF-8 encoded text strings (e.g., `{"key": "value"}`), storing raw text on disk is notoriously slow for read operations because the Engine would have to parse the string on every query.

To solve this, cluaizdb instantly transpiles the UTF-8 JSON string into a compact, binary-packed format (similar to BSON or MsgPack) before writing it to the LMDB memory map. 

### Zero-Allocation Key Lookups
Because the JSON is stored in a binary format, traversing nested keys in a `where` clause (e.g., `where user.profile.age > 18`) does not require allocating a `serde_json::Value` on the heap. The Engine performs byte-level offset jumps to locate the `age` key, casting the memory bits directly into an integer for comparison.

## Supported Limitations

| Parameter | Engine Limit | Notes |
| :--- | :--- | :--- |
| **Max Depth** | **255 Levels** | Deeply recursive JSON objects are rejected to prevent Stack Overflow vulnerabilities. |
| **Max Payload Size** | **16 Megabytes** | Larger payloads must be split, as massive contiguous memory blocks stall the B-Tree rebalancer. |
