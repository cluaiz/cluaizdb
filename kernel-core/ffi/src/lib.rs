use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;

use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv};

pub const CLUAIZDB_OK: i32 = 0;
pub const CLUAIZDB_ERR_NULL_HANDLE: i32 = -1;
pub const CLUAIZDB_ERR_NOT_FOUND: i32 = -2;
pub const CLUAIZDB_ERR_BUFFER_TOO_SMALL: i32 = -3;
pub const CLUAIZDB_ERR_STORAGE: i32 = -4;
pub const CLUAIZDB_ERR_PANIC: i32 = -5;
pub const CLUAIZDB_ERR_PARSE: i32 = -6;

#[inline]
fn to_u128(hi: u64, lo: u64) -> u128 {
    ((hi as u128) << 64) | (lo as u128)
}

/// Opens or creates a physical Substrate environment.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_open(path: *const c_char, map_size: usize) -> *mut SubstrateEnv {
    if path.is_null() {
        return ptr::null_mut();
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let c_str = CStr::from_ptr(path);
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        match SubstrateEnv::open(path_str, map_size) {
            Ok(env) => Box::into_raw(Box::new(env)),
            Err(_) => ptr::null_mut(),
        }
    }));

    result.unwrap_or(ptr::null_mut())
}

/// Safely frees an open Substrate instance.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_close(handle: *mut SubstrateEnv) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(Box::from_raw(handle));
        }));
    }
}

/// Puts a raw byte record into Substrate.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_put(
    handle: *mut SubstrateEnv,
    col_id: u32,
    id_hi: u64,
    id_lo: u64,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    if handle.is_null() || payload.is_null() {
        return CLUAIZDB_ERR_NULL_HANDLE;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let env = &*handle;
        let record_id = to_u128(id_hi, id_lo);
        let bytes = slice::from_raw_parts(payload, payload_len);

        match env.put_record(col_id, record_id, bytes) {
            Ok(()) => CLUAIZDB_OK,
            Err(_) => CLUAIZDB_ERR_STORAGE,
        }
    }));

    result.unwrap_or(CLUAIZDB_ERR_PANIC)
}

/// Retrieves a raw byte record by ID into a caller-supplied buffer (< 5 µs recall).
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_get(
    handle: *mut SubstrateEnv,
    col_id: u32,
    id_hi: u64,
    id_lo: u64,
    out_buf: *mut u8,
    max_buf_len: usize,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || out_buf.is_null() || out_len.is_null() {
        return CLUAIZDB_ERR_NULL_HANDLE;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let env = &*handle;
        let record_id = to_u128(id_hi, id_lo);

        match env.get_record(col_id, record_id) {
            Ok(Some(data)) => {
                if data.len() > max_buf_len {
                    *out_len = data.len();
                    return CLUAIZDB_ERR_BUFFER_TOO_SMALL;
                }
                ptr::copy_nonoverlapping(data.as_ptr(), out_buf, data.len());
                *out_len = data.len();
                CLUAIZDB_OK
            }
            Ok(None) => CLUAIZDB_ERR_NOT_FOUND,
            Err(_) => CLUAIZDB_ERR_STORAGE,
        }
    }));

    result.unwrap_or(CLUAIZDB_ERR_PANIC)
}

/// Deletes a record by ID.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_delete(
    handle: *mut SubstrateEnv,
    col_id: u32,
    id_hi: u64,
    id_lo: u64,
) -> i32 {
    if handle.is_null() {
        return CLUAIZDB_ERR_NULL_HANDLE;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let env = &*handle;
        let record_id = to_u128(id_hi, id_lo);

        match env.delete_record(col_id, record_id) {
            Ok(true) => CLUAIZDB_OK,
            Ok(false) => CLUAIZDB_ERR_NOT_FOUND,
            Err(_) => CLUAIZDB_ERR_STORAGE,
        }
    }));

    result.unwrap_or(CLUAIZDB_ERR_PANIC)
}

/// Forces a physical sync barrier on the environment.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_sync(handle: *mut SubstrateEnv) -> i32 {
    if handle.is_null() {
        return CLUAIZDB_ERR_NULL_HANDLE;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let env = &*handle;
        match env.sync() {
            Ok(()) => CLUAIZDB_OK,
            Err(_) => CLUAIZDB_ERR_STORAGE,
        }
    }));

    result.unwrap_or(CLUAIZDB_ERR_PANIC)
}

/// Executes a CDQL query directly against the in-process substrate and writes serialized JSON results into out_buf.
#[no_mangle]
pub unsafe extern "C" fn cluaizdb_query_cdql(
    handle: *mut SubstrateEnv,
    col_id: u32,
    query_ptr: *const c_char,
    out_buf: *mut u8,
    max_buf_len: usize,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || query_ptr.is_null() || out_buf.is_null() || out_len.is_null() {
        return CLUAIZDB_ERR_NULL_HANDLE;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let env = &*handle;
        let c_str = CStr::from_ptr(query_ptr);
        let query_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return CLUAIZDB_ERR_PARSE,
        };

        let ast = match cluaizdb_cdql::parse(query_str) {
            Ok(a) => a,
            Err(_) => return CLUAIZDB_ERR_PARSE,
        };

        let plan = match cluaizdb_cdql::build_plan(&ast) {
            Ok(p) => p,
            Err(_) => return CLUAIZDB_ERR_PARSE,
        };

        let ctx = cluaizdb_cdql::QueryContext::new(env, col_id);
        let query_result = match cluaizdb_cdql::execute_plan(&plan, &ctx) {
            Ok(r) => r,
            Err(_) => return CLUAIZDB_ERR_STORAGE,
        };

        let json_bytes = match serde_json::to_vec(&query_result) {
            Ok(b) => b,
            Err(_) => return CLUAIZDB_ERR_STORAGE,
        };

        *out_len = json_bytes.len();
        if json_bytes.len() > max_buf_len {
            return CLUAIZDB_ERR_BUFFER_TOO_SMALL;
        }

        ptr::copy_nonoverlapping(json_bytes.as_ptr(), out_buf, json_bytes.len());
        CLUAIZDB_OK
    }));

    result.unwrap_or(CLUAIZDB_ERR_PANIC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use tempfile::tempdir;

    #[test]
    fn test_c_ffi_crud_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let c_path = CString::new(path).unwrap();

        unsafe {
            // Open
            let handle = cluaizdb_open(c_path.as_ptr(), 10 * 1024 * 1024);
            assert!(!handle.is_null());

            let col_id = 1u32;
            let id_hi = 0u64;
            let id_lo = 42u64;
            let payload = b"hello from clean c ffi bridge";

            // Put
            let rc = cluaizdb_put(handle, col_id, id_hi, id_lo, payload.as_ptr(), payload.len());
            assert_eq!(rc, CLUAIZDB_OK);

            // Sync
            assert_eq!(cluaizdb_sync(handle), CLUAIZDB_OK);

            // Get
            let mut out_buf = [0u8; 128];
            let mut out_len = 0usize;
            let rc = cluaizdb_get(
                handle,
                col_id,
                id_hi,
                id_lo,
                out_buf.as_mut_ptr(),
                out_buf.len(),
                &mut out_len as *mut usize,
            );
            assert_eq!(rc, CLUAIZDB_OK);
            assert_eq!(&out_buf[..out_len], payload);

            // Delete
            assert_eq!(cluaizdb_delete(handle, col_id, id_hi, id_lo), CLUAIZDB_OK);

            // Verify Not Found
            let rc = cluaizdb_get(
                handle,
                col_id,
                id_hi,
                id_lo,
                out_buf.as_mut_ptr(),
                out_buf.len(),
                &mut out_len as *mut usize,
            );
            assert_eq!(rc, CLUAIZDB_ERR_NOT_FOUND);

            // Close
            cluaizdb_close(handle);
        }
    }

    #[test]
    fn test_c_ffi_query_cdql_execution() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let c_path = CString::new(path).unwrap();

        unsafe {
            let handle = cluaizdb_open(c_path.as_ptr(), 10 * 1024 * 1024);
            assert!(!handle.is_null());

            let col_id = 1u32;
            let id_hi = 0u64;
            let id_lo = 100u64;
            let payload = br#"{"title":"Architectural Excellence","score":99}"#;

            // Put record into substrate
            let rc = cluaizdb_put(handle, col_id, id_hi, id_lo, payload.as_ptr(), payload.len());
            assert_eq!(rc, CLUAIZDB_OK);

            // Query via CDQL FFI
            let query_str = CString::new("find id(\"100\")").unwrap();
            let mut out_buf = [0u8; 512];
            let mut out_len = 0usize;

            let rc = cluaizdb_query_cdql(
                handle,
                col_id,
                query_str.as_ptr(),
                out_buf.as_mut_ptr(),
                out_buf.len(),
                &mut out_len as *mut usize,
            );
            assert_eq!(rc, CLUAIZDB_OK);
            assert!(out_len > 0);

            let json_res: serde_json::Value = serde_json::from_slice(&out_buf[..out_len]).unwrap();
            assert_eq!(json_res["count"], 1);
            assert_eq!(json_res["records"][0]["title"], "Architectural Excellence");
            assert_eq!(json_res["records"][0]["score"], 99);

            cluaizdb_close(handle);
        }
    }
}
