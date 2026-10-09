# Auto-WASM DNA Engine

> **The fastest, safest way to execute dynamic business logic directly inside the database kernel.**

## What is Auto-WASM?

In traditional database systems, you pull data out of the database, send it over the network to a Node.js or Python backend, process it, and send the result back. This "network tax" is incredibly slow.

Cluaizdb solves this by letting you push your backend logic *into* the database using **DNA (Dynamic Neural Affordance)**. 

While Cluaizdb supports a dynamic `Rhai` scripting engine and pre-compiled `.wasm` binaries, **Auto-WASM** is the ultimate developer experience. It allows you to submit raw Rust code directly to the `POST /dna/setup` endpoint. Cluaizdb will automatically compile it into an ultra-optimized WebAssembly binary in the background and hot-reload it into the active execution pipeline.

## Why Auto-WASM is the Preferred Engine

1. **Microsecond Execution (`0.05ms`):** Because WASM is a strongly-typed binary format executing within Wasmtime, it runs at near-native C/Rust speeds. It completely bypasses the interpreter overhead of engines like Rhai or V8.
2. **Strict Memory Layout:** Unlike dynamic Document databases, Auto-WASM uses strict memory layouts. You define exactly what your JSON payload looks like using Rust `struct` definitions. This allows the memory allocator to be hyper-efficient.
3. **Hot-Reloading:** You can update the Rust code, hit the API, and Cluaizdb will seamlessly swap the WASM module without dropping a single active connection.

## Example: Writing an Auto-WASM Template

When writing an Auto-WASM template, you must define the strict data types for your payload using `serde`.

Here is an example of an `on_write` validation hook that prevents a user from inserting an invalid age.

```rust
use serde::{Deserialize, Serialize};

// 1. Define the STRICT memory layout of the expected JSON payload
#[derive(Deserialize, Serialize)]
struct UserPayload {
    name: String,
    age: i32,
    is_active: bool,
}

// 2. The entry point called by the Cluaizdb Kernel
#[no_mangle]
pub extern "C" fn validate_write(ptr: *const u8, len: usize) -> i32 {
    // Reconstruct the raw JSON bytes from the database memory
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    // Deserialize using strict types
    match serde_json::from_slice::<UserPayload>(slice) {
        Ok(user) => {
            // Apply business logic: Age must be 18 or older
            if user.age < 18 {
                return 0; // 0 = Reject Write (Validation Failed)
            }
            return 1; // 1 = Approve Write
        }
        Err(_) => {
            // Payload doesn't match our strict struct definition
            return 0; // Reject
        }
    }
}
```

## How to Deploy Auto-WASM

You submit this raw string to the server. The Cluaizdb Engine takes care of the compilation.

```bash
curl -X POST http://localhost:8080/dna/setup \
  -H "Content-Type: application/json" \
  -d '{
    "engine": "auto-wasm",
    "name": "user_validator",
    "code": "use serde::{Deserialize, Serialize}; ..."
  }'
```

Once compiled, any Neuron labeled `User` will automatically have this logic executed *before* it is allowed to be written to the LMDB storage.

---

> [!IMPORTANT]
> **Why Type Definitions Matter**
> You cannot just say `payload["age"]` in WASM. You MUST define `struct UserPayload { age: i32 }`. This strict memory constraint is the secret behind Cluaizdb's microsecond latency. If a document does not match the struct, `serde` safely fails the deserialization.
