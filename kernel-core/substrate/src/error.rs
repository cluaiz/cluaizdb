use thiserror::Error;

#[derive(Error, Debug)]
pub enum SubstrateError {
    #[error("Storage environment error: {0}")]
    Heed(#[from] heed::Error),

    #[error("Record not found for id: {0}")]
    NotFound(u128),

    #[error("Collection not registered: {0}")]
    CollectionNotFound(u32),

    #[error("Serialization / Key Encoding error: {0}")]
    Codec(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transaction aborted or conflict: {0}")]
    TxConflict(String),
}

pub type Result<T> = std::result::Result<T, SubstrateError>;
