# CDQL Parameterized Queries (Binary Binding)

Cluaizdb supports a high-performance **Parameterized Query Protocol** to handle massive data payloads (like 4096-dimensional vectors or heavy binary code blobs) without the extreme overhead of string parsing. 

Whenever you need to insert or query against heavy data, you should **never** hardcode the data into the CDQL string. Instead, use the `?` placeholder syntax.

## The Syntax
CDQL uses the `?` symbol to denote a placeholder. 

**Example (Bad - Slow String Parsing):**
```cdql
insert into Memory(id: "123", vector: [0.1, -0.4, 0.8, ...])
```

**Example (Good - 0ms Parsing Tax):**
```cdql
insert into Memory(id: "123", vector: ?)
```

When the CDQL parser sees the `?` symbol, it halts text evaluation and binds that position to the raw binary payload supplied in the secondary channel.

---

## 1. Using Parameters via FFI (Native Rust/C)

For engine integrations (like `cluaize-engine`), use the parameterized FFI endpoint. This achieves true 0ms latency by passing a direct pointer to the binary struct.

```rust
// The tiny query shell
let query = "insert into VectorDB(id: \"node-1\", data: ?)";

// The heavy binary payload
let payload_bytes = my_heavy_vector.as_bytes();

// Bind and execute
let status = unsafe {
    cluaizdb_ffi_execute_parameterized(
        query.as_ptr() as *const i8,
        payload_bytes.as_ptr(),
        payload_bytes.len()
    )
};
```

---

## 2. Using Parameters via HTTP (REST API)

For external clients (Python, JS) using the REST API, do **not** use JSON to transmit parameters. Instead, use `multipart/form-data` to isolate the text query from the binary payload.

**Endpoint:** `POST /v1/query-parameterized`

**Format:**
```http
POST /v1/query-parameterized
Content-Type: multipart/form-data; boundary=---boundary

-----boundary
Content-Disposition: form-data; name="query"

insert into Memory(id: "123", vector: ?)
-----boundary
Content-Disposition: form-data; name="param1"; filename="vector.bin"
Content-Type: application/octet-stream

<RAW BINARY BYTES OF THE VECTOR>
-----boundary--
```

The Axum server will read the `"query"` string to understand the command, and map the raw bytes from `"param1"` directly to the `?` placeholder, completely bypassing JSON deserialization tax for the heavy vector.
