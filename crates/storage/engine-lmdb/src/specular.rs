use std::collections::HashSet;
use cluaizdb_types::{NeuronId, UniversalNeuron};
use crate::{LmdbEnv, read_neuron};

/// Specular Graph Traversal (The "What-If" Matrix)
/// 
/// Traditional graph traversals use loops to find mutual connections (O(N^2)).
/// The Specular Graph computes a bitwise structural signature of a neuron's 
/// adjacency list and uses logical AND/XOR gates to find relationships in nanoseconds.
pub struct SpecularGraph;

impl SpecularGraph {
    /// Generates a 64-bit spatial signature (Bloom-filter style) representing the outgoing edges.
    pub fn generate_spatial_signature(neuron: &UniversalNeuron) -> u64 {
        let mut signature = 0u64;
        for edge in &neuron.adjacency {
            // Hash the target_id to a bit position (0-63)
            let mut hash = 0usize;
            for byte in edge.target_id.as_bytes() {
                hash = hash.wrapping_add(*byte as usize);
            }
            let bit_pos = hash % 64;
            signature |= 1 << bit_pos;
        }
        signature
    }

    /// Performs a nanosecond Specular Bitwise Intersection between two neurons.
    /// Returns true if they *likely* share a mutual connection based on bitwise AND.
    /// This bypasses LMDB deserialization of target nodes entirely.
    pub fn fast_bitwise_intersection(env: &LmdbEnv, a_id: NeuronId, b_id: NeuronId) -> anyhow::Result<bool> {
        let a = read_neuron(env, a_id, None)?;
        let b = read_neuron(env, b_id, None)?;

        let sig_a = Self::generate_spatial_signature(&a);
        let sig_b = Self::generate_spatial_signature(&b);

        // Bitwise AND directly on the signatures (Nanosecond execution)
        let mutual_signature = sig_a & sig_b;

        // If mutual_signature > 0, they likely share a connection
        Ok(mutual_signature > 0)
    }

    /// Finds exact mutual connections (Intersection) between two neurons.
    pub fn exact_intersection(env: &LmdbEnv, a_id: NeuronId, b_id: NeuronId) -> anyhow::Result<Vec<NeuronId>> {
        // Fast-path bitwise reject
        if !Self::fast_bitwise_intersection(env, a_id, b_id)? {
            return Ok(Vec::new());
        }

        // Slow-path exact match (only executed if fast-path succeeds)
        let a = read_neuron(env, a_id, None)?;
        let b = read_neuron(env, b_id, None)?;

        let set_a: HashSet<NeuronId> = a.adjacency.iter().map(|e| e.target_id).collect();
        let mut mutuals = Vec::new();

        for edge in b.adjacency {
            if set_a.contains(&edge.target_id) {
                mutuals.push(edge.target_id);
            }
        }

        Ok(mutuals)
    }
}
