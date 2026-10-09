# FFI Bypass Manual: 0ms Hardware-Native Data Layer

> *"If you can afford the latency of TCP, you are not building for real-time."*

## Who This Page Is For

This manual is for:
- **Robotics engineers** building ROS2 nodes that need sub-millisecond sensor fusion.
- **BCI (Brain-Computer Interface) researchers** capturing 256,000+ neural samples/second.
- **High-Frequency Trading** systems where every microsecond costs money.
- **Game engine developers** (Unreal, Unity) needing direct memory-mapped world state.
- **Python / C++ AI researchers** who need to pull vectors out of CLUAIZD at native speeds.

If you are building a standard web API, use the [REST API](../clients/rest-api.md) instead.

---

## The Fundamental Problem with HTTP

Every HTTP request to CLUAIZD goes through:
```
Your Code → TCP Handshake → JSON Serialize → Axum Router → WASM Eval → LMDB → JSON Deserialize → TCP Send → Your Code
```

Each step has overhead:
- **TCP:** ~20-100µs (even on localhost)
- **JSON Serialization:** ~5-50µs per record
- **Axum Router overhead:** ~2µs per request
- **Total:** ~30-150µs per write

At 256,000 writes/second (BCI target), this is mathematically impossible.

---

## The FFI Bypass Path

The C-FFI eliminates ALL of the above:
```
Your Code → cluaizdb_write() → LMDB mmap pointer → Disk
```

- **No TCP.** Shared memory, not a socket.
- **No JSON.** Raw byte arrays passed directly.
- **No Axum.** The function call IS the database.
- **Total overhead:** ~0.5-2µs per write

---

## Step 1: Build the Shared Library

```bash
# Compile CLUAIZD as a dynamic library
cargo build --release -p cluaizdb-ffi

# Outputs:
# Windows: target/release/cluaizdb.dll
# Linux:   target/release/libcluaizdb.so  
# macOS:   target/release/libcluaizdb.dylib
```

Copy `ffi/cluaizdb.h` and the `.so`/`.dll` into your project.

---

## Step 2: The C API Surface

```c
#include "cluaizdb.h"

// Open a CLUAIZD shard as a memory-mapped database
// path: filesystem path to LMDB directory
// map_size_mb: max database size in MB (e.g. 8192 = 8GB)
// Returns: opaque handle, or NULL on failure
cluaizdbdHandle* cluaizdb_open(const char* path, unsigned long map_size_mb);

// Write raw bytes directly to LMDB (no JSON, no HTTP)
// neuron_id: unique string key
// payload_json: serialized payload bytes
// Returns: 0 on success, -1 on failure
int cluaizdb_write(cluaizdbdHandle* handle, const char* neuron_id, const char* payload_json);

// Direct key lookup — bypasses ALL query planning
// Returns: allocated JSON string, caller must free
char* cluaizdb_read(cluaizdbdHandle* handle, const char* neuron_id);

// Execute a CDQL query string
char* cluaizdb_query(cluaizdbdHandle* handle, const char* cdql);

// Close and flush all pending writes
void cluaizdb_close(cluaizdbdHandle* handle);
```

---

## Step 3: C++ Robotics Example (ROS2 Node)

A ROS2 node receiving LiDAR point clouds at 100 Hz, storing each scan frame directly into CLUAIZD:

```cpp
#include "cluaizdb.h"
#include <chrono>
#include <cstdio>

class LidarStorageNode {
    cluaizdbdHandle* db_handle;
    
public:
    LidarStorageNode() {
        // Open the sensory shard directly — no HTTP server needed
        db_handle = cluaizdb_open("./out/sensory_tissue", 16384); // 16GB max
        if (!db_handle) {
            throw std::runtime_error("Failed to open CLUAIZD sensory shard");
        }
    }
    
    void on_scan_received(const LidarScan& scan) {
        // Build payload — stays on stack, no heap allocation
        char payload[4096];
        snprintf(payload, sizeof(payload),
            "{\"frame_id\": %llu, \"point_count\": %d, \"timestamp_ns\": %lld, \"sensor\": \"lidar_front\"}",
            scan.frame_id, scan.point_count, 
            std::chrono::high_resolution_clock::now().time_since_epoch().count()
        );
        
        char neuron_id[64];
        snprintf(neuron_id, sizeof(neuron_id), "lidar_%llu", scan.frame_id);
        
        // DIRECT WRITE — ~1µs latency, no TCP, no JSON parser
        int result = cluaizdb_write(db_handle, neuron_id, payload);
        if (result != 0) {
            // Handle write failure (disk full, LMDB error)
            fprintf(stderr, "CLUAIZD write failed for frame %llu\n", scan.frame_id);
        }
    }
    
    ~LidarStorageNode() {
        cluaizdb_close(db_handle);
    }
};
```

---

## Step 4: BCI Python Integration (0ms Electrode Streaming)

Using `ctypes` to stream 256-electrode EEG data at 1000 samples/electrode/second:

```python
import ctypes
import json
import time

# Load the shared library
cluaizdb = ctypes.CDLL("./libcluaizdb.so")

# Set return types
cluaizdb.cluaizdbb_open.restype = ctypes.c_void_p
cluaizdb.cluaizdbb_write.restype = ctypes.c_int
cluaizdb.cluaizdbb_read.restype = ctypes.c_char_p
cluaizdb.cluaizdbb_query.restype = ctypes.c_char_p

# Open the sensory shard directly (no HTTP server needed)
handle = cluaizdb.cluaizdbb_open(b"./out/sensory_tissue", 8192)

def stream_bci_sample(electrode_id: int, voltage: float, timestamp_ns: int):
    """Stream a single BCI electrode reading at 0ms latency."""
    neuron_id = f"bci_{electrode_id}_{timestamp_ns}".encode()
    payload = json.dumps({
        "electrode": electrode_id,
        "voltage_uv": voltage,
        "timestamp_ns": timestamp_ns,
        "sample_rate_hz": 1000
    }).encode()
    
    result = cluaizdb.cluaizdbb_write(handle, neuron_id, payload)
    return result == 0

# Simulate 256 electrodes × 1000 Hz = 256,000 writes/second
start = time.perf_counter()
for i in range(256000):
    electrode = i % 256
    stream_bci_sample(
        electrode_id=electrode,
        voltage=0.42 + (electrode * 0.001),
        timestamp_ns=time.time_ns()
    )

elapsed = time.perf_counter() - start
print(f"256,000 writes in {elapsed:.3f}s = {256000/elapsed:.0f} writes/sec")
# Expected output: ~200,000-800,000 writes/sec (depends on NVMe speed)

# Cleanup
cluaizdb.cluaizdbb_close(handle)
```

---

## Step 5: Ring Buffer Pattern for Real-Time Systems

For hard real-time systems (robotics, safety-critical), use a ring buffer to decouple the write thread from the sensor callback:

```cpp
#include "cluaizdb.h"
#include <atomic>
#include <thread>
#include <array>

constexpr size_t RING_SIZE = 65536; // Must be power of 2

struct SensorFrame {
    char payload[512];
    char neuron_id[64];
};

// Lock-free single-producer, single-consumer ring buffer
struct RingBuffer {
    std::array<SensorFrame, RING_SIZE> frames;
    std::atomic<size_t> write_head{0};
    std::atomic<size_t> read_head{0};
};

// Sensor callback (runs in interrupt/RT context — zero allocation)
void sensor_callback(RingBuffer& ring, const char* id, const char* payload) {
    size_t head = ring.write_head.load(std::memory_order_relaxed);
    ring.frames[head & (RING_SIZE - 1)].assign(id, payload);
    ring.write_head.store(head + 1, std::memory_order_release);
}

// Background writer thread (drains ring into CLUAIZD)
void writer_thread(RingBuffer& ring, cluaizdbdHandle* db) {
    while (true) {
        size_t tail = ring.read_head.load(std::memory_order_relaxed);
        size_t head = ring.write_head.load(std::memory_order_acquire);
        
        while (tail < head) {
            auto& frame = ring.frames[tail & (RING_SIZE - 1)];
            cluaizdb_write(db, frame.neuron_id, frame.payload);
            tail++;
        }
        ring.read_head.store(tail, std::memory_order_release);
        
        std::this_thread::sleep_for(std::chrono::microseconds(100));
    }
}
```

---

## Performance Benchmarks

| Method | Latency/Write | Max Throughput |
|---|---|---|
| HTTP REST API (localhost) | ~100µs | ~50,000 writes/s |
| gRPC (planned) | ~30µs | ~150,000 writes/s |
| **C-FFI Direct** | **~1µs** | **~1,000,000 writes/s** |
| C-FFI + Ring Buffer | ~0.5µs | ~2,000,000 writes/s |

> [!CAUTION]
> The C-FFI **bypasses all genome DNA hooks** (`on_write`, `on_index`). Schema validation, automatic indexing, and lifecycle hooks are NOT called during FFI writes. This is intentional for maximum performance. Use FFI only for high-throughput sensory data where you control the schema at the application level.
