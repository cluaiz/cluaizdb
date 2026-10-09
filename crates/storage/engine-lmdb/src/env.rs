use std::path::Path;

use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn};
use tracing::info;

use cluaizdb_errors::StorageError;

use heed::types::*;
use cluaizdb_types::NeuronId;
use crate::codecs::UniversalNeuronCodec;

/// The LMDB environment wrapper.
/// This struct owns the connection to the physical `.mdb` file on disk.
/// One `LmdbEnv` per database shard.
#[derive(Clone)]
pub struct LmdbEnv {
    pub(crate) env: Env,
    /// The primary neuron key-value store within this environment.
    /// Key: NeuronId (16 bytes, zero-copy bincode)
    /// Value: UniversalNeuron (zero-copy bincode via custom codec)
    pub(crate) db: Database<SerdeBincode<NeuronId>, UniversalNeuronCodec>,
    /// In-memory vector index for rapid similarity search.
    pub hnsw_index: std::sync::Arc<cluaizdb_index_mvhsnw::HnswIndex<cluaizdb_index_mvhsnw::CosineDistance>>,
}

impl LmdbEnv {
    /// Open (or create) an LMDB environment at the given directory path.
    ///
    /// # Arguments
    /// * `path` — Directory where `data.mdb` and `lock.mdb` will be created.
    /// * `map_size_bytes` — Maximum memory-mapped size for this environment.
    ///
    /// # Errors
    /// Returns `StorageError::EnvOpenFailed` if the path is invalid or permissions fail.
    pub fn open(path: &Path, map_size_bytes: usize) -> Result<Self, StorageError> {
        std::fs::create_dir_all(path).map_err(|e| StorageError::EnvOpenFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        let env = unsafe {
            let mut options = EnvOpenOptions::new();
            options.map_size(map_size_bytes);
            options.max_dbs(8);
            options.flags(heed::EnvFlags::NO_SYNC | heed::EnvFlags::NO_META_SYNC);
            options.open(path)
                .map_err(|e| StorageError::EnvOpenFailed {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?
        };

        let mut wtxn = env.write_txn().map_err(|e: heed::Error| StorageError::WriteTxnFailed(e.to_string()))?;
        let db = env
            .create_database(&mut wtxn, Some("neurons"))
            .map_err(|e: heed::Error| StorageError::WriteTxnFailed(e.to_string()))?;
        wtxn.commit().map_err(|e: heed::Error| StorageError::WriteTxnFailed(e.to_string()))?;

        info!(path = %path.display(), "LMDB environment opened successfully");

        let hnsw_index = std::sync::Arc::new(cluaizdb_index_mvhsnw::HnswIndex::new());

        Ok(Self { env, db, hnsw_index })
    }

    /// Begin a read-only transaction.
    pub fn read_txn(&self) -> Result<RoTxn<'_>, StorageError> {
        self.env
            .read_txn()
            .map_err(|e| StorageError::ReadTxnFailed(e.to_string()))
    }

    /// Begin a read-write transaction.
    pub fn write_txn(&self) -> Result<RwTxn<'_>, StorageError> {
        self.env
            .write_txn()
            .map_err(|e| StorageError::WriteTxnFailed(e.to_string()))
    }
}
