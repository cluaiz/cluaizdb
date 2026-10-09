# 📦 Binary Serialization Paradigms: FlatBuffers & Protobuf

This guide explains how Cluaizdb processes high-performance binary serialization formats (**FlatBuffers** and **Protobuf**), how they interact with dynamic DNA validation hooks, and why FlatBuffers eliminates the need for intermediate "companion objects" (in-memory structs/allocations) during parsing.

---

## 🏛️ FlatBuffers vs Protobuf vs JSON

In high-throughput systems (like Robotics sensor fusion, BCI telemetry streams, and high-frequency IoT pipelines), JSON parsing introduces a heavy CPU deserialization tax. Cluaizdb supports structured binary formats to optimize ingestion speeds:

| Feature | JSON | Protocol Buffers (Protobuf) | FlatBuffers |
| --- | --- | --- | --- |
| **Parsing Overhead** | High (String parsing & matching) | Medium (Deserialization / Decoding required) | **Zero-Copy** (Direct memory cast, no parsing) |
| **Data Format** | Text | Binary | Binary |
| **Schema Required** | No (Schema-less) | Yes (`.proto`) | Yes (`.fbs`) |
| **Heap Allocation** | High (Creates object trees/maps) | Medium (Creates companion message objects) | **Zero** (Reads directly from raw byte buffer) |
| **Use Case** | Web APIs, dynamic data | Microservices, structured tables | Real-time streams, telemetry, memory-mapped files |

---

## 🧬 Zero-Copy Concept: Why No "Companion Object" (CO) is Created

In standard serialization formats (like JSON and Protobuf), the parsing library must read the binary/text stream, allocate memory on the heap, and construct a **Companion Object (CO)** (e.g., a Rust struct, Java object, or Python dict) before you can access the fields.

### 1. The Protobuf/JSON Flow (Deserialization Tax)
```mermaid
graph TD
    Bytes[Raw Payload Bytes] -->|Parsing & Allocation| CO[Companion Object Created in Heap]
    CO -->|Field Access| Logic[DNA Engine / Business Logic]
    style CO fill:#f9f,stroke:#333,stroke-width:2px
```
* **Downside**: Heap allocations, garbage collection overhead (in managed languages), and CPU cycles wasted mapping bytes to memory structs.

### 2. The FlatBuffers Flow (Zero-Copy)
FlatBuffers organizes its binary structure using internal offsets and virtual tables (vtables). The serialized data itself contains all the structural information.
```mermaid
graph TD
    Bytes[Raw Payload Bytes] -->|Direct Pointer Cast| Logic[DNA Engine / Offset Lookup]
    style Bytes fill:#bbf,stroke:#333,stroke-width:2px
```
* **How it works**: You read the field directly from the raw byte array at a specific offset. No intermediate struct or companion object is created or allocated.

---

## 🧬 Binary Payloads & The DNA Validation Layer

When `payload_format` is set to `protobuf` or `flatbuffers`, the neuron's `raw_payload` stores structured binary segments instead of JSON text.

Since **FlatBuffers** is zero-copy, the DNA WASM engine reads attributes directly from the memory pointer of the raw bytes without deserializing the payload:

```mermaid
graph TD
    Client[Python/C++ Client] -->|Structured FlatBuffers bytes| Server[Cluaizdb Core]
    Server -->|Direct Pointer Reference| WASM[DNA WASM Sandbox]
    WASM -->|Zero-Copy Offset Read| Check[Evaluate Schema Field]
    Check -->|Pass| Store[LMDB Direct Write]
```

---

## 🛠️ Schema Configurations

### 1. FlatBuffers Schema Template (`user.fbs`)

```protobuf
// user.fbs
namespace Cluaizdb.Serial;

table UserProfile {
  id: string;
  username: string;
  age: int;
  active: bool;
}

root_type UserProfile;
```

#### WASM DNA Validation (Zero-Copy offset reading in Rust):
```rust
// Inside WASM DNA module
// Checks the FlatBuffers byte array directly without deserializing

#[no_mangle]
pub extern "C" fn validate(payload_ptr: *const u8, _payload_len: u32, _vector_ptr: *const f32) -> i32 {
    let payload = unsafe { std::slice::from_raw_parts(payload_ptr, _payload_len as usize) };
    
    // FlatBuffers stores offsets at the start of the buffer.
    // Read age property at offset 12 (as defined by user.fbs table offsets)
    if payload.len() > 16 {
        let age_offset = payload[12] as usize;
        if age_offset < 18 {
            return 0; // Abort: Underage user
        }
    }
    
    1 // Allow
}
```

---

### 2. Protobuf Schema Template (`sensor.proto`)

```protobuf
// sensor.proto
syntax = "proto3";
package cluaizdb.sensor;

message TelemetryPacket {
  string device_id = 1;
  uint64 timestamp_ms = 2;
  repeated float voltages = 3;
}
```

#### Rhai DNA Validation (Using JSON conversions):
```rust
// genomes/protobuf_check.rhai
// Parses decoded protobuf parameters

let packet = json(payload); // Decoded message passed to Rhai

// Ensure voltage is not empty
if packet.voltages.len() == 0 {
    return #{
        "action": "Abort",
        "error": "Protobuf validation failed: voltages list is empty."
    };
}

return #{
    "action": "Allow"
};
```
