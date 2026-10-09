# Auto-WASM Guide: Writing Zero-Latency DNA

`auto-wasm` is one of the 4 execution modes in Cluaizdb. It allows you to write raw Rust code, send it to the database via API, and have it automatically compile and hot-reload into the database's core memory.

> [!IMPORTANT]
> The defining feature of `auto-wasm` is **Strict Typing**. Because the code compiles down to native machine instructions, you MUST define the exact layout of your data (`String`, `bool`, `f32`). This strictness is what allows `auto-wasm` to execute at `0.05ms` per hook, bypassing the heavy overhead of dynamic JSON parsing found in traditional NoSQL databases.

## Why Define Types?

In databases like MongoDB or when using our `rhai` execution mode, the database constantly guesses the type of your data at runtime. This "dynamic typing" costs CPU cycles.

When you use `auto-wasm`, you use `serde` in Rust to define the exact shape of your incoming payload. When the FFI engine passes memory to your WASM binary, it maps directly to your struct in `O(1)` time. 

If a user tries to insert a payload that doesn't match your struct (e.g., passing a string `"true"` instead of a boolean `true`), the WASM instantly aborts the transaction before it even hits the disk.

## How to Write Auto-WASM DNA

You interact with the `POST /dna/setup` API. Set the engine to `auto-wasm` and pass your raw Rust code as a string.

```json
{
  "name": "strict_user_profile",
  "engine": "auto-wasm",
  "code": "..."
}
```

### The Boilerplate Code

Here is the template for the Rust code you would provide in the `"code"` field above. Notice how we explicitly define `UserProfilePayload` to enforce strict types.

```rust
use serde::{Deserialize, Serialize};

// 1. DEFINE YOUR STRICT TYPES
// This is critical. The database will enforce this schema at the hardware level.
#[derive(Deserialize)]
struct UserProfilePayload {
    username: String,
    is_active: bool,
    age: i32,
}

// 2. EXPOSE THE FFI HOOK
#[no_mangle]
pub extern "C" fn on_write(ptr: *const u8, len: usize) -> i32 {
    // Read the raw bytes from the host engine
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    // Attempt to parse the payload strictly into our struct
    match serde_json::from_slice::<UserProfilePayload>(slice) {
        Ok(profile) => {
            // Further custom validation logic
            if profile.age < 18 {
                return 0; // Abort transaction (underage)
            }
            if profile.username.is_empty() {
                return 0; // Abort transaction
            }
            
            1 // Allow transaction
        }
        Err(_) => {
            // Strict Type Mismatch! (e.g. `age` was a string instead of an int)
            // Instantly abort transaction.
            0
        }
    }
}
```

## How It Works Under The Hood

1. **Compilation:** When the API receives your code, it spins up a temporary Cargo project in the background and runs `cargo build --target wasm32-unknown-unknown --release`.
2. **Hot-Reload:** The resulting `.wasm` binary is placed in the `active_dnas/` directory.
3. **Caching:** Cluaizdb's internal File Watcher detects the new WASM file and instantly swaps it into the global RAM Cache without dropping any active connections.

By defining your types upfront, `auto-wasm` guarantees absolute data integrity and microsecond latencies for your database operations.
