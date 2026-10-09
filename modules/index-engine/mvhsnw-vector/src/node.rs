use parking_lot::RwLock;

/// A single node in the HNSW graph representing a vector.
#[derive(Debug)]
pub struct HnswNode {
    pub id: u128,
    pub vector: Vec<f32>,
    pub connections: Vec<RwLock<Vec<usize>>>,
}

impl HnswNode {
    pub fn new(id: u128, vector: Vec<f32>, max_level: usize) -> Self {
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
