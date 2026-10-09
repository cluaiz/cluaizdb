use wasmtime::*;
use crate::config::SandboxConfig;
use crate::error::SandboxError;

/// High-performance, fuel-metered WASM execution runtime.
pub struct WasmGenomeRunner {
    engine: Engine,
    config: SandboxConfig,
}

impl WasmGenomeRunner {
    /// Creates a new runner with the provided configuration.
    pub fn new(config: SandboxConfig) -> Result<Self, SandboxError> {
        let mut wasm_config = Config::new();
        if config.enable_fuel {
            wasm_config.consume_fuel(true);
        }

        let engine = Engine::new(&wasm_config)
            .map_err(|e| SandboxError::CompilationFailed(e.to_string()))?;

        Ok(Self { engine, config })
    }

    /// Compiles raw WASM bytecode into a reusable, in-memory `Module`.
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<Module, SandboxError> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| SandboxError::CompilationFailed(e.to_string()))
    }

    /// Returns a reference to the active sandbox configuration.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Executes an exported function that accepts scalar inputs and returns scalar results.
    pub fn call_scalar(
        &self,
        module: &Module,
        func_name: &str,
        params: &[Val],
    ) -> Result<Vec<Val>, SandboxError> {
        let mut store = Store::new(&self.engine, ());
        if self.config.enable_fuel {
            store
                .set_fuel(self.config.fuel_limit)
                .map_err(|e| SandboxError::ExecutionTrapped(e.to_string()))?;
        }

        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| SandboxError::InstantiationFailed(e.to_string()))?;

        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| SandboxError::FunctionNotFound(func_name.to_string()))?;

        let func_ty = func.ty(&store);
        let param_count = func_ty.params().len();
        let result_count = func_ty.results().len();
        let mut results = vec![Val::I32(0); result_count];

        if params.len() != param_count {
            return Err(SandboxError::InvalidOutput(format!(
                "Parameter count mismatch: expected {}, got {}",
                param_count,
                params.len()
            )));
        }

        func.call(&mut store, params, &mut results)
            .map_err(|err| self.map_trap(err))?;

        Ok(results)
    }

    /// Executes an exported function using a linear memory staging buffer.
    ///
    /// Protocol:
    /// - `func(input_offset: i32, input_len: i32) -> (output_offset: i32, output_len: i32)` (or packed i64)
    /// - Guarantees "Reads Never Write": The genome writes outputs exclusively into linear memory,
    ///   leaving storage locks completely unaffected.
    pub fn execute_memory_transform(
        &self,
        module: &Module,
        func_name: &str,
        input_data: &[u8],
    ) -> Result<Vec<u8>, SandboxError> {
        let mut store = Store::new(&self.engine, ());
        if self.config.enable_fuel {
            store
                .set_fuel(self.config.fuel_limit)
                .map_err(|e| SandboxError::ExecutionTrapped(e.to_string()))?;
        }

        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| SandboxError::InstantiationFailed(e.to_string()))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| SandboxError::MemoryOutOfBounds("No exported 'memory' found in WASM module".to_string()))?;

        let input_offset = 0usize;
        let input_len = input_data.len();

        // Ensure linear memory has sufficient capacity for input staging buffer
        let mem_size = memory.data_size(&store);
        if input_offset + input_len > mem_size {
            let needed_pages = ((input_offset + input_len - mem_size + 65535) / 65536) as u64;
            memory
                .grow(&mut store, needed_pages)
                .map_err(|e| SandboxError::MemoryOutOfBounds(format!("Failed to grow memory: {}", e)))?;
        }

        // Write input staging buffer into linear memory
        memory
            .write(&mut store, input_offset, input_data)
            .map_err(|e| SandboxError::MemoryOutOfBounds(e.to_string()))?;

        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| SandboxError::FunctionNotFound(func_name.to_string()))?;

        let mut results = vec![Val::I32(0); 2];
        let params = [Val::I32(input_offset as i32), Val::I32(input_len as i32)];

        func.call(&mut store, &params, &mut results)
            .map_err(|err| self.map_trap(err))?;

        let out_offset = match results[0] {
            Val::I32(o) => o as usize,
            _ => return Err(SandboxError::InvalidOutput("Output offset must be i32".to_string())),
        };
        let out_len = match results[1] {
            Val::I32(l) => l as usize,
            _ => return Err(SandboxError::InvalidOutput("Output length must be i32".to_string())),
        };

        if out_offset + out_len > memory.data_size(&store) {
            return Err(SandboxError::MemoryOutOfBounds(format!(
                "Output buffer bounds [{}..{}] exceed linear memory capacity {}",
                out_offset,
                out_offset + out_len,
                memory.data_size(&store)
            )));
        }

        let mut output_bytes = vec![0u8; out_len];
        memory
            .read(&store, out_offset, &mut output_bytes)
            .map_err(|e| SandboxError::MemoryOutOfBounds(e.to_string()))?;

        Ok(output_bytes)
    }

    fn map_trap(&self, err: wasmtime::Error) -> SandboxError {
        if let Some(trap) = err.downcast_ref::<Trap>() {
            if *trap == Trap::OutOfFuel {
                return SandboxError::OutOfFuel;
            }
        }

        let mut source = err.source();
        while let Some(e) = source {
            if let Some(trap) = e.downcast_ref::<Trap>() {
                if *trap == Trap::OutOfFuel {
                    return SandboxError::OutOfFuel;
                }
            }
            if e.to_string().to_lowercase().contains("fuel") {
                return SandboxError::OutOfFuel;
            }
            source = e.source();
        }

        let err_str = err.to_string();
        if err_str.to_lowercase().contains("fuel") {
            SandboxError::OutOfFuel
        } else {
            SandboxError::ExecutionTrapped(err_str)
        }
    }
}
