use tracing::debug;

use cluaizdb_errors::StorageError;
use cluaizdb_types::{NeuronId, UniversalNeuron};

use crate::env::LmdbEnv;

/// Read a `UniversalNeuron` from the LMDB database by its `NeuronId`.
///
/// # Arguments
/// * `env` — The open LMDB environment.
/// * `id` — The unique identifier of the neuron to fetch.
/// * `query_model_hash` — Optional SHA-256 hash of the requesting model.
///   If provided and it does not match the stored `model_creator_hash`,
///   an error is returned immediately to prevent cross-model vector misuse.
///
/// # Errors
/// Returns `StorageError::NeuronNotFound` if the ID does not exist.
/// Returns `StorageError::DeserializationFailed` if the stored bytes are corrupt.
pub fn read_neuron(
    env: &LmdbEnv,
    id: NeuronId,
    query_model_hash: Option<[u8; 32]>,
) -> Result<UniversalNeuron, StorageError> {
    let rtxn = env.read_txn()?;

    let mut neuron: UniversalNeuron = match env.db.get(&rtxn, &id)
        .map_err(|e: heed::Error| StorageError::ReadTxnFailed(e.to_string()))? {
        Some(n) => n,
        None => return Err(StorageError::NeuronNotFound(id)),
    };

    // Validate model hash if a query hash was provided.
    if let Some(query_hash) = query_model_hash {
        if neuron.model_creator_hash != query_hash {
            return Err(StorageError::WriteTxnFailed(format!(
                "Model hash mismatch: stored={:?}, query={:?}. Re-embed with current model.",
                neuron.model_creator_hash, query_hash
            )));
        }
    }

    // Execute Circulation (on_read) DNA Hook via Unified GenomeExecutor
    let mut changed = false;
    let old_adjacency = neuron.adjacency.clone();
    
    if let Err(e) = genome::GenomeExecutor::execute_on_read(&mut neuron) {
        return Err(e);
    }

    if neuron.adjacency != old_adjacency {
        changed = true;
    }

    // Drop read transaction to free locks before a potential write transaction
    drop(rtxn);

    if changed {
        // We write the updated neuron back (background logic or inline)
        let _ = crate::writer::write_neuron(env, &neuron);
        debug!(neuron_id = %id, "Neuron connections strengthened dynamically on read");
    }

    debug!(neuron_id = %id, "Neuron read from LMDB");
    Ok(neuron)
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use cluaizdb_types::PayloadType;

    use super::*;
    use crate::{env::LmdbEnv, writer::write_neuron};

    #[test]
    fn test_write_then_read_round_trip() {
        let tmp_dir = std::env::temp_dir().join("cluaizd_test_read");
        let env = LmdbEnv::open(&tmp_dir, 10 * 1024 * 1024).expect("env open failed");

        let model_hash = [42u8; 32];
        let neuron = UniversalNeuron::new(
            Bytes::from("hello cluaizd"),
            [1.0f32; 16].to_vec(),
            model_hash,
            PayloadType::Text,
        );
        let saved_id = neuron.id;

        write_neuron(&env, &neuron).expect("write failed");

        // Read back with matching model hash — should succeed
        let fetched = read_neuron(&env, saved_id, Some(model_hash)).expect("read failed");
        assert_eq!(fetched.id, saved_id);
        assert_eq!(fetched.raw_payload, Bytes::from("hello cluaizd"));

        // Read back with wrong model hash — should fail
        let wrong_hash = [0u8; 32];
        let result = read_neuron(&env, saved_id, Some(wrong_hash));
        assert!(result.is_err(), "Expected error on hash mismatch");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}

/// Iterate over all neurons in the LMDB database.
/// This reads the entire database and decodes each neuron.
pub fn iter_all_neurons(env: &crate::env::LmdbEnv) -> Result<Vec<UniversalNeuron>, StorageError> {
    let rtxn = env.read_txn()?;
    let mut neurons = Vec::new();

    let iter = env.db.iter(&rtxn).map_err(|e: heed::Error| StorageError::ReadTxnFailed(e.to_string()))?;
    for result in iter {
        let (_k, neuron) = result.map_err(|e: heed::Error| StorageError::ReadTxnFailed(e.to_string()))?;
        neurons.push(neuron);
    }

    Ok(neurons)
}

/// Scans the database and returns neurons matching a specific payload type.
pub fn scan_neurons_by_type(
    env: &crate::env::LmdbEnv,
    payload_type: cluaizdb_types::PayloadType,
) -> Result<Vec<UniversalNeuron>, StorageError> {
    let rtxn = env.read_txn()?;
    let mut neurons = Vec::new();

    let iter = env.db.iter(&rtxn).map_err(|e: heed::Error| StorageError::ReadTxnFailed(e.to_string()))?;
    for result in iter {
        let (_k, neuron) = result.map_err(|e: heed::Error| StorageError::ReadTxnFailed(e.to_string()))?;
        if neuron.payload_type == payload_type {
            neurons.push(neuron);
        }
    }

    Ok(neurons)
}
