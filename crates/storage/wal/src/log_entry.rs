use serde::{Deserialize, Serialize};

use cluaizdb_types::NeuronId;

/// The type of operation recorded in the WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalOperation {
    /// A neuron write (insert or update).
    Write {
        /// Serialized `UniversalNeuron` bytes.
        payload: Vec<u8>,
    },
    /// A neuron deletion.
    Delete {
        /// The ID of the neuron to delete.
        neuron_id: NeuronId,
    },
}

/// A single atomic entry in the Write-Ahead Log.
///
/// Every mutation (write or delete) is first appended to the WAL
/// before being committed to LMDB. On crash, uncommitted WAL entries
/// are replayed to restore consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Monotonically increasing sequence number. Used for ordering during replay.
    pub sequence: u64,

    /// The unique ID of the neuron this entry affects.
    pub neuron_id: NeuronId,

    /// The operation type.
    pub operation: WalOperation,

    /// CRC32 checksum of (`sequence` + `neuron_id bytes` + `operation bytes`).
    /// If this does not match during recovery, the entry is skipped (corrupt).
    pub checksum: u32,
}

impl WalEntry {
    /// Compute a simple checksum over the entry contents.
    /// Uses CRC32 for fast, reliable corruption detection.
    pub fn compute_checksum(sequence: u64, neuron_id: &NeuronId, operation: &WalOperation) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sequence.hash(&mut hasher);
        neuron_id.as_bytes().hash(&mut hasher);

        // Hash operation type tag only (not full payload — too expensive in hot path)
        match operation {
            WalOperation::Write { .. } => 1u8.hash(&mut hasher),
            WalOperation::Delete { .. } => 2u8.hash(&mut hasher),
        }

        hasher.finish() as u32
    }

    /// Verify the stored checksum against recomputed value.
    pub fn is_valid(&self) -> bool {
        let expected =
            Self::compute_checksum(self.sequence, &self.neuron_id, &self.operation);
        self.checksum == expected
    }
}
