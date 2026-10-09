use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Failed to compile WASM bytecode: {0}")]
    CompilationFailed(String),

    #[error("Failed to instantiate WASM module: {0}")]
    InstantiationFailed(String),

    #[error("Instruction fuel exhausted: execution exceeded fuel budget")]
    OutOfFuel,

    #[error("Linear memory out of bounds: {0}")]
    MemoryOutOfBounds(String),

    #[error("WASM execution trapped: {0}")]
    ExecutionTrapped(String),

    #[error("Exported entrypoint function '{0}' not found in WASM module")]
    FunctionNotFound(String),

    #[error("Invalid execution output: {0}")]
    InvalidOutput(String),
}
