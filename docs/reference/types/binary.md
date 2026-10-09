# `binary` Data Type

The `binary` payload format is the default raw format in Cluaizdb. It is used to store raw, unstructured byte buffers.

## Architectural Storage

### Direct Block Placement
Binary payloads are written directly to the LMDB leaf page as a raw byte array with zero transpile overhead. If the payload exceeds the inline threshold (approx 2KB depending on compile config), it is automatically placed in an LMDB overflow page, keeping the B-Tree root indices small and shallow.

## Use Cases
- Serialized FlatBuffers/Protobuf payloads verified by DNA hooks.
- Compressed sensor telemetry or custom binary formats.
