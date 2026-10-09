use tracing::debug;

use cluaizdb_errors::StorageError;
use cluaizdb_types::UniversalNeuron;

use crate::env::LmdbEnv;

/// Write a `UniversalNeuron` to the LMDB database.
///
/// Serializes the neuron to JSON bytes and stores it under its `NeuronId` key.
/// In a future phase, this will use zero-copy binary serialization (rkyv/flatbuffers).
///
/// # Errors
/// Returns `StorageError` if the write transaction or serialization fails.
pub fn write_neuron(env: &LmdbEnv, neuron: &UniversalNeuron) -> Result<(), StorageError> {
    if !neuron.vector_data.is_empty() && !neuron.vector_data.iter().all(|&x| x == 0.0) {
        env.hnsw_index.insert(neuron.id, neuron.vector_data.clone());
    }
    let mut wtxn = env.write_txn()?;

    env.db
        .put(&mut wtxn, &neuron.id, neuron)
        .map_err(|e: heed::Error| StorageError::WriteTxnFailed(e.to_string()))?;

    wtxn.commit().map_err(|e: heed::Error| StorageError::WriteTxnFailed(e.to_string()))?;

    debug!(neuron_id = %neuron.id, "Neuron written to LMDB");
    Ok(())
}



#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use cluaizdb_types::PayloadType;

    use super::*;
    use crate::env::LmdbEnv;

    #[test]
    fn test_write_neuron_succeeds() {
        let tmp_dir = std::env::temp_dir().join("cluaizd_test_write");
        let env = LmdbEnv::open(&tmp_dir, 10 * 1024 * 1024).expect("env open failed");

        let neuron = UniversalNeuron::new(
            Bytes::from("test payload"),
            vec![0.1f32; 16],
            [0u8; 32],
            PayloadType::Text,
        );

        write_neuron(&env, &neuron).expect("write failed");
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
