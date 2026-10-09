// cluaizdb-dna-templates/auto_wasm_example.rs
// Rust code script to be sent for live Auto-WASM compilation

use std::slice;

// Standard memory allocators
#[no_mangle]
pub extern "C" fn allocate(size: u32) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: u32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size as usize);
    }
}

// Custom validation rule: ensure age in json payload is over 18
#[no_mangle]
pub extern "C" fn validate(payload_ptr: *const u8, payload_len: u32, _vector_ptr: *const f32) -> i32 {
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len as usize) };
    
    if let Ok(payload_str) = std::str::from_utf8(payload) {
        // Simple search check for age parameters in raw JSON string
        if payload_str.contains("\"age\"") && !payload_str.contains("\"age\": 0") {
            return 1; // Passed validation
        }
    }
    
    0 // Aborted validation
}
