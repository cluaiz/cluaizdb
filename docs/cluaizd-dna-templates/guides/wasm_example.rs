// cluaizdb-dna-templates/wasm_example.rs
// Rust template for compiling a WASM DNA Module

use std::slice;
use std::mem;

// 1. Memory Management Exports (Required by Cluaizdb WASM Executor)

#[no_mangle]
pub extern "C" fn allocate(size: u32) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size as usize);
    let ptr = buffer.as_mut_ptr();
    mem::forget(buffer); // Relinquish ownership to WASM memory space
    ptr
}

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: u32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size as usize);
    }
}

// 2. DNA Hook: validate (on_write validation)
// Returns 1 for success/allow, 0 for abort
#[no_mangle]
pub extern "C" fn validate(payload_ptr: *const u8, payload_len: u32, vector_ptr: *const f32) -> i32 {
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len as usize) };
    let _vector = unsafe { slice::from_raw_parts(vector_ptr, 16) }; // 16-D vector array

    // Example check: Ensure payload is not empty
    if payload.is_empty() {
        return 0; // Abort
    }

    1 // Allow
}

// 3. DNA Hook: execute_query (on_index search query)
#[no_mangle]
pub extern "C" fn execute_query(
    query_ptr: *const u8,
    query_len: u32,
    payload_ptr: *const u8,
    payload_len: u32,
) -> i32 {
    let query = unsafe { slice::from_raw_parts(query_ptr, query_len as usize) };
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len as usize) };

    // Example match: check if payload contains query text
    if payload.windows(query.len()).any(|window| window == query) {
        return 1; // Matched
    }

    0 // No match
}
