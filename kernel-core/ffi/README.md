# 🔌 kernel-core/ffi: Zero-Overhead C Dynamic Linkage Bridge
> **Module:** C-ABI FFI Shared Library (`cdylib` / `cluaizdb.dll` / `libcluaizdb.so`)  
> **Recall Target:** `< 5 µs` In-Process Local Memory Recall  
> **Clients:** `cluaiz` AI Inference Engine, Python SDK, C++ Robotics

---

## 🎯 1. Purpose
`kernel-core/ffi` provides direct, in-process C-linkage entry points into `cluaizdb-substrate`. By bypassing HTTP and network socket serialization, native processes (such as the `cluaiz` inference engine, llama.cpp drivers, or embedded agents) can read and write memories within microsecond physical SSD cached page-cache latency.

---

## 🏛️ 2. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- Export `extern "C"` functions with C calling conventions (`cdylib`, `staticlib`, `rlib`).
- Safely marshal raw pointer handoffs, byte buffers, and return codes.
- Protect against null pointer dereferences and catch unwinding panics (`std::panic::catch_unwind`).

### ❌ What this crate CANNOT do:
- Perform network communication or spin up web servers.
- Depend on high-level Python/JavaScript runtimes.
- Leak memory or allow unmanaged pointer lifetimes across boundaries.

---

## 📦 3. Core Exported C Functions

```c
// Open & Close
void* cluaizdb_open(const char* path, size_t map_size);
void  cluaizdb_close(void* handle);

// Microsecond Point I/O
int32_t cluaizdb_put(void* handle, uint32_t col_id, uint64_t id_hi, uint64_t id_lo, const uint8_t* payload, size_t payload_len);
int32_t cluaizdb_get(void* handle, uint32_t col_id, uint64_t id_hi, uint64_t id_lo, uint8_t* out_buf, size_t max_buf_len, size_t* out_len);
int32_t cluaizdb_delete(void* handle, uint32_t col_id, uint64_t id_hi, uint64_t id_lo);

// Durability Barrier
int32_t cluaizdb_sync(void* handle);
```

---

## ⚡ 4. Return Codes
- `0`: Success (`CLUAIZDB_OK`)
- `-1`: Null Handle Error (`CLUAIZDB_ERR_NULL_HANDLE`)
- `-2`: Record Not Found (`CLUAIZDB_ERR_NOT_FOUND`)
- `-3`: Buffer Overflow (`CLUAIZDB_ERR_BUFFER_TOO_SMALL`)
- `-4`: Internal Storage / IO Error (`CLUAIZDB_ERR_STORAGE`)
- `-5`: Panic Caught (`CLUAIZDB_ERR_PANIC`)
