use cluaizd_errors::StorageError;
use wasmtime::{Engine, Instance, Module, Store};
use dashmap::DashMap;
use notify::{Watcher, RecursiveMode, Event};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    /// Global shared WASM Engine to avoid creation cost
    pub static ref WASM_ENGINE: Engine = Engine::default();
    
    /// Global RAM Cache for WASM Modules (Hot-Reloadable)
    /// Maps DNA Label -> wasmtime::Module
    pub static ref WASM_CACHE: DashMap<String, Module> = DashMap::new();
}

/// A highly optimized WASM Executor for CLUAIZD.
/// Uses a global Engine and RAM Cache for microsecond execution.
pub struct WasmExecutor;

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Preload all WASM files from the active_dnas directory into RAM cache
    pub fn preload_cache(active_dnas_dir: &Path) {
        if !active_dnas_dir.exists() {
            tracing::info!("active_dnas directory not found, skipping WASM preload.");
            return;
        }

        if let Ok(entries) = std::fs::read_dir(active_dnas_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "wasm") {
                    Self::load_module_to_cache(&path);
                }
            }
        }
    }

    /// Internal helper to read bytes and compile to WASM module
    fn load_module_to_cache(path: &Path) {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            match std::fs::read(path) {
                Ok(bytes) => {
                    match Module::from_binary(&WASM_ENGINE, &bytes) {
                        Ok(module) => {
                            WASM_CACHE.insert(file_stem.to_string(), module);
                            tracing::info!("🧬 Loaded DNA into RAM Cache: {}", file_stem);
                        }
                        Err(e) => tracing::error!("Failed to compile WASM DNA {}: {}", file_stem, e),
                    }
                }
                Err(e) => tracing::error!("Failed to read WASM file {:?}: {}", path, e),
            }
        }
    }

    /// Execute a WASM module directly from raw bytes (no RAM cache).
    /// Useful for legacy neurons that embed their own WASM modules.
    pub fn execute_from_bytes(&self, wasm_bytes: &[u8], function_name: &str) -> Result<i32, StorageError> {
        let module = Module::from_binary(&WASM_ENGINE, wasm_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let func = instance.get_typed_func::<(), i32>(&mut store, function_name)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let result = func.call(&mut store, ())
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        Ok(result)
    }

    /// Execute a WASM module's exported function from RAM cache.
    pub fn execute_cached(&self, module_name: &str, function_name: &str) -> Result<i32, StorageError> {
        let module = WASM_CACHE.get(module_name)
            .ok_or_else(|| StorageError::WasmExecutionFailed(format!("WASM module '{}' not found in RAM cache", module_name)))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &*module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let func = instance.get_typed_func::<(), i32>(&mut store, function_name)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let result = func.call(&mut store, ())
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        Ok(result)
    }

    /// Execute a WASM module's exported function.
    pub fn execute(&self, wasm_bytes: &[u8], function_name: &str) -> Result<i32, StorageError> {
        let module = Module::from_binary(&WASM_ENGINE, wasm_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let func = instance.get_typed_func::<(), i32>(&mut store, function_name)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let result = func.call(&mut store, ())
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        Ok(result)
    }

    /// Execute a Booster WASM module, passing telemetry data.
    pub fn execute_booster(&self, wasm_bytes: &[u8], bp: u32, spo2: u32, process_bp: u32, process_spo2: u32, gpu: u32, ssd: u32, mode: u32) -> Result<u32, StorageError> {
        let module = Module::from_binary(&WASM_ENGINE, wasm_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let func = instance.get_typed_func::<(u32, u32, u32, u32, u32, u32, u32), u32>(&mut store, "should_suspend")
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let result = func.call(&mut store, (bp, spo2, process_bp, process_spo2, gpu, ssd, mode))
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        Ok(result)
    }

    /// Execute a dynamic query against a neuron's payload using WASM DNA.
    pub fn execute_query(&self, wasm_bytes: &[u8], query: &str, payload: &[u8]) -> Result<i32, StorageError> {
        let module = Module::from_binary(&WASM_ENGINE, wasm_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| StorageError::WasmExecutionFailed("No memory exported".to_string()))?;
            
        let alloc_func = instance.get_typed_func::<u32, i32>(&mut store, "allocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No allocate: {}", e)))?;
            
        let dealloc_func = instance.get_typed_func::<(i32, u32), ()>(&mut store, "deallocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No deallocate: {}", e)))?;
            
        // Allocate and write query
        let q_len = query.len() as u32;
        let q_ptr = alloc_func.call(&mut store, q_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        memory.write(&mut store, q_ptr as usize, query.as_bytes())
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Allocate and write payload
        let p_len = payload.len() as u32;
        let p_ptr = alloc_func.call(&mut store, p_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        memory.write(&mut store, p_ptr as usize, payload)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Execute query
        let exec_func = instance.get_typed_func::<(i32, u32, i32, u32), i32>(&mut store, "execute_query")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No execute_query: {}", e)))?;
            
        let result = exec_func.call(&mut store, (q_ptr, q_len, p_ptr, p_len))
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Cleanup memory
        let _ = dealloc_func.call(&mut store, (q_ptr, q_len));
        let _ = dealloc_func.call(&mut store, (p_ptr, p_len));
        
        Ok(result)
    }

    /// Execute a validation hook on a payload and vector using WASM DNA (from bytes).
    pub fn execute_validate(&self, wasm_bytes: &[u8], payload: &[u8], vector: &[f32]) -> Result<bool, StorageError> {
        let module = Module::from_binary(&WASM_ENGINE, wasm_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(format!("Invalid DNA WASM: {}", e)))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| StorageError::WasmExecutionFailed("No memory exported by DNA".to_string()))?;
            
        let alloc_func = instance.get_typed_func::<u32, i32>(&mut store, "allocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No allocate: {}", e)))?;
            
        let dealloc_func = instance.get_typed_func::<(i32, u32), ()>(&mut store, "deallocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No deallocate: {}", e)))?;
            
        // Allocate and write payload
        let p_len = payload.len() as u32;
        let p_ptr = alloc_func.call(&mut store, p_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        memory.write(&mut store, p_ptr as usize, payload)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Allocate and write vector (vector.len() * 4 bytes)
        let v_len = (vector.len() * 4) as u32;
        let v_ptr = alloc_func.call(&mut store, v_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        let vector_bytes = unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, v_len as usize) };
        memory.write(&mut store, v_ptr as usize, vector_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Execute validation
        let validate_func = instance.get_typed_func::<(i32, u32, i32), i32>(&mut store, "validate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("DNA is missing validate hook: {}", e)))?;
            
        let result = validate_func.call(&mut store, (p_ptr, p_len, v_ptr))
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Cleanup memory
        let _ = dealloc_func.call(&mut store, (p_ptr, p_len));
        let _ = dealloc_func.call(&mut store, (v_ptr, v_len));
        
        Ok(result == 1)
    }

    /// Execute a validation hook on a payload and vector using RAM cached WASM DNA.
    pub fn execute_validate_cached(&self, module_name: &str, payload: &[u8], vector: &[f32]) -> Result<bool, StorageError> {
        let module = WASM_CACHE.get(module_name)
            .ok_or_else(|| StorageError::WasmExecutionFailed(format!("WASM DNA '{}' not found in RAM cache", module_name)))?;
        
        let mut store = Store::new(&WASM_ENGINE, ());
        let instance = Instance::new(&mut store, &*module, &[])
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| StorageError::WasmExecutionFailed("No memory exported by DNA".to_string()))?;
            
        let alloc_func = instance.get_typed_func::<u32, i32>(&mut store, "allocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No allocate: {}", e)))?;
            
        let dealloc_func = instance.get_typed_func::<(i32, u32), ()>(&mut store, "deallocate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("No deallocate: {}", e)))?;
            
        // Allocate and write payload
        let p_len = payload.len() as u32;
        let p_ptr = alloc_func.call(&mut store, p_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        memory.write(&mut store, p_ptr as usize, payload)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Allocate and write vector (vector.len() * 4 bytes)
        let v_len = (vector.len() * 4) as u32;
        let v_ptr = alloc_func.call(&mut store, v_len)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
        let vector_bytes = unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, v_len as usize) };
        memory.write(&mut store, v_ptr as usize, vector_bytes)
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Execute validation
        let validate_func = instance.get_typed_func::<(i32, u32, i32), i32>(&mut store, "validate")
            .map_err(|e| StorageError::WasmExecutionFailed(format!("DNA is missing validate hook: {}", e)))?;
            
        let result = validate_func.call(&mut store, (p_ptr, p_len, v_ptr))
            .map_err(|e| StorageError::WasmExecutionFailed(e.to_string()))?;
            
        // Cleanup memory
        let _ = dealloc_func.call(&mut store, (p_ptr, p_len));
        let _ = dealloc_func.call(&mut store, (v_ptr, v_len));
        
        Ok(result == 1)
    }
}

/// Spawns a background task to watch the `active_dnas/` directory and hot-reload WASM files
pub async fn start_dna_watcher(active_dnas_dir: PathBuf) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    
    // Create the notify watcher
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            let _ = tx.blocking_send(event);
        }
    }).unwrap();

    // Start watching the directory
    if let Err(e) = watcher.watch(&active_dnas_dir, RecursiveMode::NonRecursive) {
        tracing::error!("Failed to watch DNA directory: {:?}", e);
        return;
    }

    tracing::info!("🧬 DNA Hot-Reload Watcher started on {:?}", active_dnas_dir);

    // Keep the watcher alive in the background and process events
    tokio::spawn(async move {
        // We move `watcher` in here to keep it alive
        let _keep_alive = watcher;
        
        while let Some(event) = rx.recv().await {
            // If a file is created or modified
            if event.kind.is_modify() || event.kind.is_create() {
                for path in event.paths {
                    if path.extension().map_or(false, |ext| ext == "wasm") {
                        tracing::info!("🔄 DNA Change Detected: {:?}", path.file_name().unwrap());
                        WasmExecutor::load_module_to_cache(&path);
                    }
                }
            }
        }
    });
}
