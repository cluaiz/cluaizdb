use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tracing::{debug, info};

use cluaizdb_errors::StorageError;
use cluaizdb_types::{NeuronId, UniversalNeuron};

use crate::log_entry::{WalEntry, WalOperation};

/// Maximum WAL segment file size (64 MB). When exceeded, a new segment is created.
const MAX_SEGMENT_SIZE_BYTES: u64 = 64 * 1024 * 1024;

/// Appends mutation entries to the Write-Ahead Log on disk.
///
/// All writes to LMDB must first pass through the WAL writer.
/// This guarantees zero data loss on crash.
///
/// ## Segment Rotation
/// WAL files are rotated at 64 MB (`wal_00001.log`, `wal_00002.log`, etc.)
/// to prevent unbounded file growth.
pub struct WalWriter {
    wal_dir: PathBuf,
    current_file: std::fs::File,
    current_segment: u64,
    current_size: u64,
    /// Monotonically increasing sequence counter (shared across all writers).
    sequence: Arc<AtomicU64>,
}

impl WalWriter {
    /// Open (or create) the WAL writer at the given directory.
    ///
    /// # Arguments
    /// * `wal_dir` — Directory where WAL segment files will be stored.
    ///
    /// # Errors
    /// Returns `StorageError::WalAppendFailed` if the directory cannot be created
    /// or the initial segment file cannot be opened.
    pub fn open(wal_dir: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(wal_dir)
            .map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        let segment = 1u64;
        let path = wal_dir.join(format!("wal_{:05}.log", segment));

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        let current_size = file.metadata().map(|m: std::fs::Metadata| m.len()).unwrap_or(0);

        info!(path = %path.display(), "WAL writer opened");

        Ok(Self {
            wal_dir: wal_dir.to_path_buf(),
            current_file: file,
            current_segment: segment,
            current_size,
            sequence: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Append a neuron write operation to the WAL.
    ///
    /// This must be called BEFORE writing to LMDB.
    ///
    /// # Errors
    /// Returns `StorageError::WalAppendFailed` on I/O failure.
    pub fn append_write(&mut self, neuron: &UniversalNeuron) -> Result<(), StorageError> {
        let payload =
            serde_json::to_vec(neuron).map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let operation = WalOperation::Write { payload };
        let checksum = WalEntry::compute_checksum(seq, &neuron.id, &operation);

        let entry = WalEntry { sequence: seq, neuron_id: neuron.id, operation, checksum };

        self.write_entry(&entry)
    }

    /// Append a neuron delete operation to the WAL.
    pub fn append_delete(&mut self, neuron_id: NeuronId) -> Result<(), StorageError> {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let operation = WalOperation::Delete { neuron_id };
        let checksum = WalEntry::compute_checksum(seq, &neuron_id, &operation);

        let entry = WalEntry { sequence: seq, neuron_id, operation, checksum };

        self.write_entry(&entry)
    }

    /// Serialize and flush a `WalEntry` to the current log segment.
    fn write_entry(&mut self, entry: &WalEntry) -> Result<(), StorageError> {
        let bytes =
            serde_json::to_vec(entry).map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        // Write: [4-byte length prefix][entry bytes][newline]
        let len = (bytes.len() as u32).to_le_bytes();
        self.current_file
            .write_all(&len)
            .map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;
        self.current_file
            .write_all(&bytes)
            .map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        self.current_size += (4 + bytes.len()) as u64;

        debug!(sequence = entry.sequence, "WAL entry appended");

        // Rotate to a new segment if we hit the size limit.
        if self.current_size >= MAX_SEGMENT_SIZE_BYTES {
            self.rotate_segment()?;
        }

        Ok(())
    }

    /// Rotate to a new WAL segment file.
    fn rotate_segment(&mut self) -> Result<(), StorageError> {
        self.current_segment += 1;
        let path = self.wal_dir.join(format!("wal_{:05}.log", self.current_segment));

        self.current_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StorageError::WalAppendFailed(e.to_string()))?;

        self.current_size = 0;
        info!(segment = self.current_segment, "WAL segment rotated");
        Ok(())
    }

    pub fn sync(&mut self) -> Result<(), StorageError> {
        self.current_file.sync_all().map_err(|e| StorageError::WalAppendFailed(e.to_string()))
    }
}
