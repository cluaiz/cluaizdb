# 🏎️ Concurrency & Serialization Engine

CLUAIZD natively supports a highly tunable, dual-architecture setup for handling maximum concurrency and dynamic serialization. This flexibility allows nodes running on lightweight edge devices to behave safely, while heavy cloud instances can scale to maximum concurrent throughput.

## Concurrency Modes

CLUAIZD's Shard Manager dynamically routes tenants into one of two threading environments based on configuration:

### 1. `dashmap` (Multi-Threaded Lock-Free)
- **What it is**: Uses a highly concurrent, lock-free `DashMap` (sharded hash map) to manage active environment pointers.
- **When to use**: High-throughput scenarios (20k+ OPS), heavy read/write multiplexing, and large multi-core servers.
- **How it works**: Multiple threads can access and mutate different internal shards simultaneously without ever blocking each other, resulting in zero lock contention.

### 2. `mutex` (Single-Threaded Strict Lock)
- **What it is**: Uses a standard `tokio::sync::Mutex` to wrap the active environment maps.
- **When to use**: Highly strict atomic requirement workloads, single-core edge devices, or when lock contention is mathematically impossible (e.g., single-tenant setups).
- **How it works**: Guarantees absolute sequential isolation by allowing only one thread to mutate the Shard Manager's state at a time.

## Serialization Formats

Payload parsing is dynamically configured per-collection.

### 1. JSON (Default)
- **Use Case**: Rapid prototyping, human-readable debugging, standard web applications.
- **Benefits**: Supported universally. Easy to mutate via Rhai DNA scripts.

### 2. Protobuf (Protocol Buffers)
- **Use Case**: Fast binary encoded structured data for microservices and gRPC transit.
- **Benefits**: Strongly typed schemas, tiny memory footprint, highly optimized parsing over network boundaries.

### 3. FlatBuffers
- **Use Case**: Absolute maximum read performance and zero-copy access.
- **Benefits**: Allows the CLUAIZD C-FFI pipeline and memory-mapped LMDB environment to read complex nested data directly from RAM *without* decoding or parsing it first.

## Hierarchical Configuration System

These settings can be applied at two distinct layers:
1. **Global Tier (`cluaizdb.toml`)**: Sets the default behavior for the entire physical server cluster.
2. **Local Tier (`collection_config.json`)**: Specific databases (tenants) can completely override the global tier. 
   - *Example: You can run a JSON/Mutex database and a FlatBuffers/DashMap database natively on the exact same CLUAIZD instance!*
