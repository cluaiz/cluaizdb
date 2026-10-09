use cluaizdb_types::NeuronId;

/// All errors that can occur in the physical storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Failed to open LMDB environment at path '{path}': {reason}")]
    EnvOpenFailed { path: String, reason: String },

    #[error("Write transaction failed: {0}")]
    WriteTxnFailed(String),

    #[error("Read transaction failed: {0}")]
    ReadTxnFailed(String),

    #[error("Neuron not found: {0}")]
    NeuronNotFound(NeuronId),

    #[error("Database migration failed: {0}")]
    MigrationFailed(String),
    #[error("WASM Execution failed: {0}")]
    WasmExecutionFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("WAL append failed: {0}")]
    WalAppendFailed(String),

    #[error("WAL recovery failed: {0}")]
    WalRecoveryFailed(String),

    #[error("Sensory shard write failed: {0}")]
    SensoryShardFailed(String),

    #[error("DNA Validation failed: {0}")]
    DnaValidationFailed(String),
}
