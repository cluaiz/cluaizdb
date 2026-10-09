use cluaizdb_types::NeuronId;
use parking_lot::RwLock;
use std::sync::Arc;

/// A single node in the HNSW graph representing a stored vector.
#[derive(Debug)]
pub struct HnswNode {
    pub id: NeuronId,
    /// The actual vector data. In a production engine, this might just be a pointer 
    /// or index into LMDB, but we hold it here for fast distance computations.
    pub vector: Vec<f32>,
    /// Connections at each level. Index 0 is the base layer (densest).
    /// Index N is the top layer (sparsest).
    pub connections: Vec<RwLock<Vec<usize>>>,
}

impl HnswNode {
    pub fn new(id: NeuronId, vector: Vec<f32>, max_level: usize) -> Self {
        let mut connections = Vec::with_capacity(max_level + 1);
        for _ in 0..=max_level {
            connections.push(RwLock::new(Vec::new()));
        }
        
        Self {
            id,
            vector,
            connections,
        }
    }
}
