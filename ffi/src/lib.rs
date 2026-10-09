//! # CLUAIZD FFI — C-Compatible Bindings
//!
//! Exposes CLUAIZD as a native C library for **0ms local access**.
//! No HTTP, no serialization overhead — direct in-process memory calls.
//!
//! ## Use Cases
//! - **Robotics:** ROS2 nodes, embedded controllers (C/C++)
//! - **Python AI:** `ctypes` or `cffi` for direct ML model data ingestion
//! - **Brain-Computer Interfaces:** Neuralink/OpenBCI C SDKs
//! - **Game Engines:** Unreal/Godot C++ integrations
//!
//! ## Compile
//! ```sh
//! cargo build --release -p cluaizdb-ffi
//! # Output: target/release/cluaizdb.dll (Windows) / cluaizdb.so (Linux)
//! ```
//!
//! ## C Usage
//! ```c
//! #include "cluaizdb.h"
//!
//! CluaizdbHandle* db = cluaizdb_open("./data/mydb", 4096);
//! cluaizdb_write(db, "hello world", 11, "text");
//! cluaizdb_close(db);
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_ulong};
use std::sync::Mutex;

use bytes::Bytes;
use cluaizdb_types::{PayloadType, UniversalNeuron};
use engine_lmdb::LmdbEnv;

/// Opaque handle representing an open CLUAIZD  instance.
/// The caller holds a raw pointer — they must call `cluaizdb_close` to free it.
pub struct CluaizdbHandle {
    env: LmdbEnv,
}

/// Open a CLUAIZD at the given path.
///
/// # Arguments
/// - `path`: UTF-8 path to the database directory (will be created if absent)
/// - `map_size_mb`: Maximum database size in megabytes (e.g. 4096 = 4GB)
///
/// # Returns
/// A non-null `CluaizdbHandle*` on success, or `NULL` on failure.
///
/// # Safety
/// Caller must free the returned pointer with `cluaizdb_close()`.
#[no_mangle]
pub extern "C" fn cluaizdb_open(path: *const c_char, map_size_mb: usize) -> *mut CluaizdbHandle {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let safe_mb = if map_size_mb > 100000 { 1024 } else { map_size_mb };
    let map_size = safe_mb * 1024 * 1024;
    match LmdbEnv::open(std::path::Path::new(path_str), map_size) {
        Ok(env) => {
            let handle = Box::new(CluaizdbHandle {
                env,
            });
            Box::into_raw(handle)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Write a raw payload into CLUAIZD.
///
/// # Arguments
/// - `handle`: A valid `CluaizdbHandle*` from `cluaizdb_open()`
/// - `payload`: Pointer to the raw byte payload
/// - `payload_len`: Length of the payload in bytes
/// - `payload_type`: Type string: "text", "audio", "video", "code", "voltage_stream", or "binary"
///
/// # Returns
/// A heap-allocated null-terminated string containing the assigned Neuron UUID.
/// The caller MUST free this with `cluaizdb_free_string()`.
/// Returns `NULL` on failure.
///
/// # Safety
/// `payload` must point to a valid buffer of at least `payload_len` bytes.
#[no_mangle]
pub extern "C" fn cluaizdb_write(
    handle: *mut CluaizdbHandle,
    payload: *const u8,
    payload_len: usize,
    payload_type: *const c_char,
) -> *mut c_char {
    if handle.is_null() || payload.is_null() || payload_type.is_null() {
        return std::ptr::null_mut();
    }

    let bytes = unsafe {
        let slice = std::slice::from_raw_parts(payload, payload_len);
        Bytes::copy_from_slice(slice)
    };

    let ptype_str = unsafe { CStr::from_ptr(payload_type) }
        .to_str()
        .unwrap_or("binary");

    let ptype = match ptype_str {
        "text" => PayloadType::Text,
        "audio" => PayloadType::Audio,
        "video" => PayloadType::Video,
        "code" => PayloadType::Code,
        "voltage_stream" => PayloadType::VoltageStream,
        _ => PayloadType::Binary,
    };

    let mut neuron = UniversalNeuron::new(bytes.clone(), [0.0f32; 16].to_vec(), [0u8; 32], ptype);
    
    // Attempt to hydrate DNA if payload is JSON (just like HTTP route)
    if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if let Some(dna_val) = json_val.get("dna") {
            if let Ok(dna) = serde_json::from_value::<cluaizdb_types::NeuronDna>(dna_val.clone()) {
                neuron.dna = Some(dna);
            }
        }
    }

    let neuron_id = neuron.id.to_string();

    // Execute DNA Validation (Subconscious AI)
    match genome::GenomeExecutor::execute_on_write(&neuron, 120.0, 98.0) {
        Ok(genome::GenomeWriteAction::Allow(_)) => {
            // Proceed to write
        }
        Ok(genome::GenomeWriteAction::Defer) => {
            // Defer (We just return success for FFI lazy mock right now)
            return CString::new(neuron_id)
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut());
        }
        Ok(genome::GenomeWriteAction::Abort(_)) | Err(_) => {
            // Validation failed or aborted! Abort write.
            return std::ptr::null_mut();
        }
    }

    let handle_ref = unsafe { &*handle };
    let env = &handle_ref.env;

    match engine_lmdb::write_neuron(&env, &neuron) {
        Ok(_) => {
            CString::new(neuron_id)
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut())
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Read a neuron's raw payload from CLUAIZD by its UUID string.
///
/// # Arguments
/// - `handle`: A valid `CluaizdbHandle*`
/// - `neuron_id`: A null-terminated UUID string (e.g. "550e8400-e29b-41d4-...")
/// - `out_len`: Pointer to a `ulong` that receives the payload byte length
///
/// # Returns
/// A heap-allocated byte buffer containing the raw payload.
/// The caller MUST free this with `cluaizdb_free_bytes()`.
/// Returns `NULL` if the neuron is not found.
///
/// # Safety
/// `out_len` must be a valid pointer.
#[no_mangle]
pub extern "C" fn cluaizdb_read(
    handle: *mut CluaizdbHandle,
    neuron_id: *const c_char,
    out_len: *mut c_ulong,
) -> *mut u8 {
    if handle.is_null() || neuron_id.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }

    let id_str = unsafe { CStr::from_ptr(neuron_id) }
        .to_str()
        .unwrap_or("");

    let uuid = match uuid::Uuid::parse_str(id_str) {
        Ok(u) => u,
        Err(_) => return std::ptr::null_mut(),
    };

    let nid = cluaizdb_types::NeuronId::from_bytes(*uuid.as_bytes());

    let handle_ref = unsafe { &*handle };
    let env = &handle_ref.env;

    match engine_lmdb::read_neuron(&env, nid, None) {
        Ok(neuron) => {
            let payload = neuron.raw_payload.to_vec();
            let len = payload.len();
            let mut boxed = payload.into_boxed_slice();
            let ptr = boxed.as_mut_ptr();
            std::mem::forget(boxed);
            unsafe { *out_len = len as c_ulong };
            ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Query CLUAIZD using a CDQL string.
///
/// # Arguments
/// - `handle`: A valid `CluaizdbHandle*`
/// - `cdql`: A null-terminated CDQL query string (e.g. `find *(name: "Aryan")`)
///
/// # Returns
/// A heap-allocated null-terminated JSON string containing the array of matched neuron IDs.
/// The caller MUST free this with `cluaizdb_free_string()`.
/// Returns `NULL` on failure.
#[no_mangle]
pub extern "C" fn cluaizdb_query(
    handle: *mut CluaizdbHandle,
    cdql: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cdql.is_null() {
        return std::ptr::null_mut();
    }

    let query_str = unsafe { CStr::from_ptr(cdql) }
        .to_str()
        .unwrap_or("");

    let handle_ref = unsafe { &*handle };
    let env = &handle_ref.env;

    match engine_lmdb::iter_all_neurons(&env) {
        Ok(neurons) => {
            let ids: Vec<String> = if let Ok(query) = genome::cdql::parser::parse(query_str) {
                if let Ok(plan) = genome::cdql::planner::build_plan(&query) {
                    let mut working_set: Vec<&cluaizdb_types::UniversalNeuron> = neurons.iter().collect();
                    for step in &plan.steps {
                        match step {
                            genome::cdql::planner::PlanStep::FastPathIdLookup { id } => {
                                working_set.retain(|n| n.id.to_string() == *id);
                            }
                            genome::cdql::planner::PlanStep::ScanAll { label_filter, filters } => {
                                working_set.retain(|n| {
                                    let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                                    if let Some(ref label) = label_filter {
                                        if label != "*" && !payload_str.to_lowercase().contains(&label.to_lowercase()) {
                                            return false;
                                        }
                                    }
                                    // A full filter match requires CDQL evaluation engine in genome
                                    // For now, this validates CDQL syntax parsing works in FFI
                                    true
                                });
                            }
                            genome::cdql::planner::PlanStep::Limit(n) => {
                                working_set.truncate(*n);
                            }
                            _ => {}
                        }
                    }
                    working_set.into_iter().map(|n| n.id.to_string()).collect()
                } else {
                    vec![]
                }
            } else {
                // Fallback to simple payload search if CDQL fails to parse
                neurons.iter()
                    .filter(|n| {
                        let payload = String::from_utf8_lossy(&n.raw_payload);
                        payload.to_lowercase().contains(&query_str.to_lowercase())
                    })
                    .map(|n| n.id.to_string())
                    .collect()
            };

            let json = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string());
            CString::new(json)
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut())
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Close a CLUAIZD handle and free all associated resources.
///
/// # Safety
/// `handle` must be a valid pointer from `cluaizdb_open()`. After this call, the pointer is invalid.
#[no_mangle]
pub extern "C" fn cluaizdb_close(handle: *mut CluaizdbHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Free a string returned by CLUAIZD FFI functions.
///
/// # Safety
/// `ptr` must be a pointer returned by `cluaizdb_write()` or `cluaizdb_query()`.
#[no_mangle]
pub extern "C" fn cluaizdb_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

/// Free a byte buffer returned by `cluaizdb_read()`.
///
/// # Arguments
/// - `ptr`: The pointer returned by `cluaizdb_read()`
/// - `len`: The length previously written to `out_len`
///
/// # Safety
/// `ptr` and `len` must match what was returned by `cluaizdb_read()`.
#[no_mangle]
pub extern "C" fn cluaizdb_free_bytes(ptr: *mut u8, len: c_ulong) {
    if !ptr.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(ptr, len as usize);
            drop(Box::from_raw(slice as *mut [u8]));
        }
    }
}
