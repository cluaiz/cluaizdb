use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::{NeuronId, PayloadType};

/// Represents a weighted relationship edge between two neurons.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeuronEdge {
    /// The target neuron's unique identifier.
    pub target_id: NeuronId,
    /// The weight (strength) of the connection, typically between 0.0 and 1.0.
    pub weight: f32,
    /// Timestamp in Unix nanoseconds of when this connection was last traversed/accessed.
    pub last_accessed_ns: u64,
}

/// The Living DNA Sequence attached to a node, separated into 4 operational hooks.
/// If `None`, the neuron defaults to the static raw "Static Heap" storage mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NeuronDna {
    /// Executed before writing. Used for Strict Schema validation (SQL) or Tokenization (Search).
    pub on_write: Option<String>,

    /// Executed during querying. Used for JSON filtering (NoSQL) or Graph BFS Traversal.
    pub on_read: Option<String>,

    /// Executed for indexing. Used for Vector Cosine Math or Geospatial Radius.
    pub on_index: Option<String>,

    /// Executed by the deep graph traversal API to determine if an edge should be followed.
    pub on_traverse: Option<String>,

    /// Executed by the background Dreaming Engine to forge new semantic connections.
    pub on_dream: Option<String>,

    /// Executed by the GC. Used for Edge Decay, TTL, and Tier transitions.
    pub on_lifecycle: Option<String>,

    /// Executed at each step of speculative path search to validate path conditions.
    pub on_path_step: Option<String>,

    /// Executed on path resolution (reinforcement) for winning paths.
    pub on_path_resolve: Option<String>,

    /// Pre-compiled WASM bytecode module for native machine execution.
    #[serde(skip)] // Don't serialize the raw bytes
    pub wasm_module: Option<Vec<u8>>,

    /// Path to a .wasm file on disk. Loaded by GenomeRegistry on startup.
    pub wasm_module_path: Option<String>,

    /// Dynamic parameters injected into the DNA (controlled via Cluaizdb-JUJU UI).
    pub parameters: serde_json::Value,

    /// The type of DNA engine to execute these sequences (e.g. "rhai", "wasm").
    pub engine: String,
}

/// Defines the 3-Tier Biological Storage State of a Neuron
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageTier {
    /// Tier 1: Fully uncompressed, 0ms latency.
    Hot,
    /// Tier 2: Payload deleted, only vector and graph retained (Shadow Intuition).
    Warm,
    /// Tier 3: Entire neuron compressed with ZSTD.
    Cold,
}

/// The fundamental atomic unit of storage in Cluaizdb CLUAIZD.
///
/// Every piece of data — text, audio, video, or raw voltage streams —
/// is stored as a `UniversalNeuron`. Fields are co-located on disk
/// for single-read access (no joins, no foreign keys).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalNeuron {
    /// Unique time-sortable identifier for this neuron.
    pub id: NeuronId,

    /// The raw original payload — text bytes, audio bytes, video chunks, etc.
    /// Stored zero-copy using `Bytes`. Heavy payload subject to Neural TTL.
    pub raw_payload: Bytes,

    /// The 16-dimensional hardware footprint bitmask of this neuron.
    /// Generated externally by an AI model or hardware transducer — NOT by the DB.
    /// Stored alongside the raw payload (no separate index table).
    pub vector_data: Vec<f32>,

    /// SHA-256 hash of the AI model that generated `vector_data`.
    /// On query, if the requesting model's hash does not match this value,
    /// a `VectorMismatchError` is returned immediately.
    /// This prevents cross-model vector incompatibility issues.
    pub model_creator_hash: [u8; 32],

    /// The type of data stored in `raw_payload`.
    /// Controls routing logic in the transducer and storage engine.
    pub payload_type: PayloadType,

    /// Creation timestamp in Unix nanoseconds.
    pub created_at_ns: u64,

    /// Adjacency list representing directed weighted edges to related neurons.
    pub adjacency: Vec<NeuronEdge>,

    /// The biological storage state (Hot, Warm, Cold).
    pub tier: StorageTier,

    /// Optional DNA sequence attached to this neuron.
    /// If `None`, the neuron resides in the static "Static Heap" default mode.
    pub dna: Option<NeuronDna>,
}

impl UniversalNeuron {
    /// Create a new neuron with required fields.
    /// `dna` defaults to `None` (Static Heap Mode) and `adjacency` starts empty.
    pub fn new(
        raw_payload: Bytes,
        vector_data: Vec<f32>,
        model_creator_hash: [u8; 32],
        payload_type: PayloadType,
    ) -> Self {
        let created_at_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos() as u64;

        Self {
            id: NeuronId::new(),
            raw_payload,
            vector_data,
            model_creator_hash,
            payload_type,
            created_at_ns,
            adjacency: Vec::new(),
            tier: StorageTier::Hot,
            dna: None,
        }
    }

    /// Note: `is_expired` logic is now delegated entirely to the DNA engine.
    /// This method is a stub for the Rust core to call the DNA interpreter if needed.
    pub fn execute_dna_lifecycle(&self) {
        // This will be invoked by the GC engine evaluating the DNA script.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_creation() {
        let payload = Bytes::from("hello world");
        let vector = vec![0.1f32; 16];
        let hash = [0u8; 32];
        let neuron = UniversalNeuron::new(payload.clone(), vector, hash, PayloadType::Text);

        assert_eq!(neuron.raw_payload, payload);
        assert!(neuron.adjacency.is_empty());
        assert_eq!(neuron.dna, None);
    }

    #[test]
    fn test_neuron_dna_attachment() {
        let payload = Bytes::from("test");
        let vector = vec![0.0f32; 16];
        let hash = [0u8; 32];
        let mut neuron = UniversalNeuron::new(payload, vector, hash, PayloadType::Text);

        // Attach DNA sequence
        neuron.dna = Some(NeuronDna {
            on_write: None,
            on_read: None,
            on_index: None,
            on_traverse: None,
            on_dream: None,
            on_lifecycle: Some("if age > 1000 { return true; } else { return false; }".to_string()),
            on_path_step: None,
            on_path_resolve: None,
            wasm_module: None,
            wasm_module_path: None,
            parameters: serde_json::json!({}),
            engine: "rhai".to_string(),
        });

        assert!(neuron.dna.is_some());
        assert_eq!(neuron.dna.unwrap().engine, "rhai");
    }
}
