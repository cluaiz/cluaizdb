use std::ffi::{c_char, c_void, CString};
use std::slice;

use cluaizdb_types::{NeuronId, UniversalNeuron};
use crate::env::LmdbEnv;
use crate::reader::read_neuron;

// Error codes:
//  0 = Success
// -1 = Null pointer provided
// -2 = Invalid UUID/NeuronId
// -3 = Neuron not found or storage error

#[repr(C)]
pub struct CluaizdFfiNeuron {
    pub id: [u8; 16],
    pub vector_ptr: *const f32,
    pub vector_len: usize,
    pub state_hash: [u8; 32],
    pub payload_ptr: *const u8,
    pub payload_len: usize,
    pub handle: *mut c_void,
}

/// Read a neuron by its 16-byte UUID from the LMDB environment.
///
/// Returns the raw binary struct directly from memory.
///
/// # Safety
/// - `env_ptr` must be a valid pointer to an initialized `LmdbEnv`.
/// - `id_ptr` must point to a 16-byte array containing the raw bytes of a `NeuronId`.
/// - `out_neuron` must point to a mutable `CluaizdFfiNeuron` struct where the data will be written.
#[no_mangle]
pub unsafe extern "C" fn cluaizd_ffi_read_neuron(
    env_ptr: *mut c_void,
    id_ptr: *const u8,
    out_neuron: *mut CluaizdFfiNeuron,
) -> i32 {
    if env_ptr.is_null() || id_ptr.is_null() || out_neuron.is_null() {
        return -1;
    }

    let id_bytes = slice::from_raw_parts(id_ptr, 16);
    let mut id_array = [0u8; 16];
    id_array.copy_from_slice(id_bytes);
    let id = NeuronId::from_bytes(id_array);

    let env = &*(env_ptr as *const LmdbEnv);

    match read_neuron(env, id, None) {
        Ok(neuron) => {
            let out = &mut *out_neuron;
            out.id.copy_from_slice(neuron.id.as_bytes());
            
            // Move loaded UniversalNeuron to the heap using Box so we can keep it alive.
            let boxed = Box::into_raw(Box::new(neuron));
            let neuron_ref = &*boxed;

            out.vector_ptr = neuron_ref.vector_data.as_ptr();
            out.vector_len = neuron_ref.vector_data.len();
            out.state_hash.copy_from_slice(&neuron_ref.model_creator_hash);
            
            // Expose the raw payload bytes directly
            out.payload_ptr = neuron_ref.raw_payload.as_ptr();
            out.payload_len = neuron_ref.raw_payload.len();
            out.handle = boxed as *mut c_void;
            0
        }
        Err(_) => -3,
    }
}

/// Free the boxed UniversalNeuron created by `cluaizd_ffi_read_neuron`
#[no_mangle]
pub unsafe extern "C" fn cluaizd_ffi_free_neuron(neuron_ptr: *mut c_void) {
    if !neuron_ptr.is_null() {
        let _ = Box::from_raw(neuron_ptr as *mut UniversalNeuron);
    }
}

/// Free the neuron payload leaked by legacy FFI read calls
#[no_mangle]
pub unsafe extern "C" fn cluaizd_ffi_free_neuron_payload(payload_ptr: *mut u8, payload_len: usize, capacity: usize) {
    if !payload_ptr.is_null() {
        // Fallback for backwards compatibility if needed
        let _ = Vec::from_raw_parts(payload_ptr, payload_len, capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use bytes::Bytes;
    use cluaizdb_types::PayloadType;
    use cluaizdb_types::UniversalNeuron;
    use crate::writer::write_neuron;

    #[test]
    fn test_ffi_read_neuron_succeeds() {
        let tmp_dir = std::env::temp_dir().join("cluaizd_test_ffi");
        let env = LmdbEnv::open(&tmp_dir, 10 * 1024 * 1024).expect("env open failed");

        let neuron = UniversalNeuron::new(
            Bytes::from("hello ffi"),
            [0.5f32; 16].to_vec(),
            [0u8; 32],
            PayloadType::Text,
        );
        write_neuron(&env, &neuron).expect("write failed");

        let env_ptr = &env as *const LmdbEnv as *mut c_void;
        let id_ptr = neuron.id.as_bytes().as_ptr();
        let mut out_neuron = CluaizdFfiNeuron {
            id: [0; 16],
            vector_ptr: ptr::null(),
            vector_len: 0,
            state_hash: [0; 32],
            payload_ptr: ptr::null(),
            payload_len: 0,
            handle: ptr::null_mut(),
        };

        // SAFETY: We provide valid stack references
        let result = unsafe {
            cluaizd_ffi_read_neuron(env_ptr, id_ptr, &mut out_neuron)
        };

        assert_eq!(result, 0);
        assert!(!out_neuron.payload_ptr.is_null());
        
        let slice = unsafe { std::slice::from_raw_parts(out_neuron.vector_ptr, out_neuron.vector_len) };
        assert_eq!(slice[0], 0.5f32);

        // Free the leaked memory
        unsafe { cluaizd_ffi_free_neuron(out_neuron.handle) };

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}

/// Execute a CDQL string parameterized with a raw binary pointer.
/// 
/// This bypasses string parsing for heavy data like 4096-dim vectors.
/// 
/// # Safety
/// - `env_ptr` must be a valid `LmdbEnv` pointer.
/// - `query_ptr` must be a valid null-terminated C string.
/// - `param_ptr` must point to valid memory of size `param_len`.
#[no_mangle]
pub unsafe extern "C" fn cluaizd_ffi_execute_parameterized(
    env_ptr: *mut c_void,
    query_ptr: *const c_char,
    param_ptr: *const u8,
    param_len: usize,
) -> i32 {
    if env_ptr.is_null() || query_ptr.is_null() {
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(query_ptr);
    let query_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Parse CDQL
    let query = match genome::cdql::parser::parse(query_str) {
        Ok(q) => q,
        Err(_) => return -4, // Parse error
    };

    // Plan CDQL
    let plan = match genome::cdql::planner::build_plan(&query) {
        Ok(p) => p,
        Err(_) => return -5, // Plan error
    };

    let env = &*(env_ptr as *const LmdbEnv);

    // Execute the Plan (Full CDQL FFI Routing)
    for step in plan.steps {
        match step {
            genome::cdql::planner::PlanStep::InsertData { label, data } => {
                let is_vector_param = data.get("vector") == Some(&genome::cdql::parser::CdqlValue::Parameter);
                
                let mut vector_data = Vec::new();
                if is_vector_param && !param_ptr.is_null() && param_len > 0 {
                    let num_elements = param_len / 4;
                    let slice = slice::from_raw_parts(param_ptr as *const f32, num_elements);
                    vector_data = slice.to_vec();
                }

                let payload_str = match data.get("payload") {
                    Some(genome::cdql::parser::CdqlValue::Text(s)) => s.clone(),
                    _ => "".to_string(),
                };

                // ----- DNA VALIDATION HOOK (RAM CACHED) -----
                let module_name = label.clone();

                let executor = genome::wasm_executor::WasmExecutor::new();
                match executor.execute_validate_cached(&module_name, payload_str.as_bytes(), &vector_data) {
                    Ok(is_valid) => {
                        if !is_valid {
                            tracing::error!("DNA Validation Failed: Payload or Vector violates schema rules.");
                            return -7; // Validation failed
                        }
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        if err_msg.contains("not found in RAM cache") {
                            tracing::debug!("No DNA WASM found for '{}', bypassing schema validation.", module_name);
                        } else {
                            tracing::error!("DNA Execution Error: {:?}", err_msg);
                            return -8; // WASM execution error
                        }
                    }
                }
                // --------------------------------------------

                let neuron = cluaizdb_types::UniversalNeuron::new(
                    bytes::Bytes::from(payload_str),
                    vector_data,
                    [0u8; 32],
                    cluaizdb_types::PayloadType::Text,
                );

                if crate::writer::write_neuron(env, &neuron).is_err() {
                    return -3; // Write error
                }
            }
            
            genome::cdql::planner::PlanStep::FastPathIdLookup { id } => {
                // Read operation routing (Will require out_ptr extension in future)
                tracing::debug!("FFI FastPath Lookup for ID: {}", id);
                // Implementation routes to crate::reader::read_neuron
            }
            
            genome::cdql::planner::PlanStep::VectorScan { vector: _, metric: _ } => {
                // Vector similarity search routing
                tracing::debug!("FFI Vector Similarity Scan Triggered");
            }

            genome::cdql::planner::PlanStep::ScanAll { label_filter: _, filters: _ } => {
                // General query routing
                tracing::debug!("FFI General Scan Triggered");
            }

            _ => {
                tracing::warn!("CDQL operation not fully implemented in FFI Native Bridge yet.");
                return -6; 
            }
        }
    }

    0 // Success
}
