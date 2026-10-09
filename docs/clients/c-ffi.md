# C-FFI Bindings: Direct Memory Access

> **Max throughput: ~1,000,000 writes/second**  
> **Latency: ~0.5-2µs per operation**  
> **Protocol: Shared memory (no TCP, no JSON parsing)**

The C-FFI completely bypasses the HTTP server, Axum router, and JSON serialization layers. It writes directly into LMDB's memory-mapped file from your process's address space.

> [!IMPORTANT]
> For a full deep-dive into WHY the FFI is faster and how to use it for BCI/Robotics use cases, see the [FFI Bypass Manual](../architecture/ffi-bypass.md). This page covers the core API reference.

---

## Step 1: Build the Library

```bash
# Compile as a dynamic shared library
cargo build --release -p cluaizdb-ffi

# Output locations:
# Windows:  target/release/cluaizdb.dll
# Linux:    target/release/libcluaizdb.so
# macOS:    target/release/libcluaizdb.dylib
```

Copy the output library and `ffi/cluaizdb.h` into your project.

---

## Full API Reference (`cluaizdb.h`)

```c
#include "cluaizdb.h"

// ─────────────────────────────────────────────
// cluaizdb_open()
// Open (or create) an LMDB shard at the given path.
//
// @path:        Filesystem path to the LMDB directory (created if missing)
// @map_size_mb: Maximum database size in MB (e.g. 8192 = 8 GB)
// @return:      Opaque handle, or NULL on failure
// ─────────────────────────────────────────────
cluaizdbdHandle* cluaizdb_open(const char* path, unsigned long map_size_mb);

// ─────────────────────────────────────────────
// cluaizdb_write()
// Write raw bytes into LMDB. 
// Note: If the neuron has a label that matches an Auto-WASM/WASM module in the `active_dnas/` directory,
// this function WILL trigger the RAM-Cached validation hook at microsecond speed!
//
// @handle:       Handle from cluaizdb_open()
// @neuron_id:    String key (any unique identifier)
// @payload_json: JSON string payload
// @return:       0 on success, -1 on failure
// ─────────────────────────────────────────────
int cluaizdb_write(cluaizdbdHandle* handle, const char* neuron_id, const char* payload_json);

// ─────────────────────────────────────────────
// cluaizdb_read()
// Direct LMDB key lookup — O(1) Fast-Path.
//
// @handle:    Handle from cluaizdb_open()
// @neuron_id: The key to look up
// @return:    Heap-allocated JSON string. CALLER MUST FREE with cluaizdb_free_string().
// ─────────────────────────────────────────────
char* cluaizdb_read(cluaizdbdHandle* handle, const char* neuron_id);

// ─────────────────────────────────────────────
// cluaizdb_query()
// Execute a CDQL query string against this shard.
//
// @handle: Handle from cluaizdb_open()
// @cdql:   CDQL query string
// @return: Heap-allocated JSON array string. CALLER MUST FREE with cluaizdb_free_string().
// ─────────────────────────────────────────────
char* cluaizdb_query(cluaizdbdHandle* handle, const char* cdql);

// ─────────────────────────────────────────────
// cluaizdb_free_string()
// Free a string returned by cluaizdb_read() or cluaizdb_query().
// ALWAYS call this to prevent memory leaks.
// ─────────────────────────────────────────────
void cluaizdb_free_string(char* ptr);

// ─────────────────────────────────────────────
// cluaizdb_close()
// Flush all pending writes and close the LMDB environment.
// ─────────────────────────────────────────────
void cluaizdb_close(cluaizdbdHandle* handle);
```

---

## C++ Example: Sensor Ingestion

```cpp
#include "cluaizdb.h"
#include <cstdio>
#include <cstring>

int main() {
    // Open a dedicated sensory shard (isolated from main database)
    cluaizdbdHandle* handle = cluaizdb_open("./neuron/sensory_tissue", 8192);
    if (!handle) {
        fprintf(stderr, "Failed to open CLUAIZD\n");
        return 1;
    }

    // Write 1,000,000 sensor readings
    for (int i = 0; i < 1000000; i++) {
        char id[64];
        char payload[256];
        snprintf(id, sizeof(id), "reading_%07d", i);
        snprintf(payload, sizeof(payload),
            "{\"sensor\": \"temp_01\", \"value\": %.2f, \"ts\": %lld}",
            20.0 + (i % 10) * 0.5, (long long)i * 1000000LL);

        if (cluaizdb_write(handle, id, payload) != 0) {
            fprintf(stderr, "Write failed at index %d\n", i);
            break;
        }
    }

    // Direct key lookup — bypasses CDQL entirely
    char* result = cluaizdb_read(handle, "reading_0000500");
    if (result) {
        printf("Record: %s\n", result);
        cluaizdb_free_string(result);  // Always free!
    }

    // CDQL query via FFI
    char* query_result = cluaizdb_query(handle,
        "find * -> filter sensor: \"temp_01\" -> limit 10");
    if (query_result) {
        printf("Query: %s\n", query_result);
        cluaizdb_free_string(query_result);  // Always free!
    }

    cluaizdb_close(handle);
    return 0;
}
```

**Compile:**
```bash
gcc -o sensor_app sensor_app.c -L./target/release -lcluaizdb -Wl,-rpath,./target/release
```

---

## Python Integration via `ctypes`

```python
import ctypes
import json

# Load the shared library
cluaizdb = ctypes.CDLL("./target/release/libcluaizdb.so")

# Configure return types (CRITICAL — without this, Python will crash)
cluaizdb.cluaizdbb_open.restype = ctypes.c_void_p
cluaizdb.cluaizdbb_open.argtypes = [ctypes.c_char_p, ctypes.c_ulong]

cluaizdb.cluaizdbb_write.restype = ctypes.c_int
cluaizdb.cluaizdbb_write.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p]

cluaizdb.cluaizdbb_read.restype = ctypes.c_char_p
cluaizdb.cluaizdbb_read.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

cluaizdb.cluaizdbb_query.restype = ctypes.c_char_p
cluaizdb.cluaizdbb_query.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

cluaizdb.cluaizdbb_free_string.argtypes = [ctypes.c_char_p]
cluaizdb.cluaizdbb_close.argtypes = [ctypes.c_void_p]

# Open database
handle = cluaizdb.cluaizdbb_open(b"./neuron/sensory_tissue", 4096)

# Write data
payload = json.dumps({"electrode": 42, "voltage_uv": 0.42, "ts": 1717789200}).encode()
cluaizdb.cluaizdbb_write(handle, b"bci_reading_001", payload)

# Read back (direct O(1) lookup)
result_bytes = cluaizdb.cluaizdbb_read(handle, b"bci_reading_001")
if result_bytes:
    print(json.loads(result_bytes))

# Close
cluaizdb.cluaizdbb_close(handle)
```

---

## Performance vs HTTP

| Method | Throughput | Latency | Overhead |
|---|---|---|---|
| HTTP REST (`/neuron`) | ~50K writes/s | ~100µs | TCP + JSON + Axum routing |
| **C-FFI (`cluaizdb_write`)** | **~1M writes/s** | **~1µs** | **None — direct mmap** |
| C-FFI + Ring Buffer | ~2M writes/s | ~0.5µs | Amortized batching |

> [!CAUTION]
> `cluaizdb_write()` bypasses the slow JSON API and CDQL planner, but **it DOES execute WASM validation hooks** if the neuron matches a label in the `active_dnas/` RAM Cache. This ensures data integrity even at 1,000,000 writes/sec!
