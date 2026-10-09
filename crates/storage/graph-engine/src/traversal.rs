use cluaizdb_types::{NeuronId, UniversalNeuron};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NeuronId),
    #[error("Max depth exceeded")]
    MaxDepthExceeded,
}

/// A generic abstraction for fetching neurons, 
/// allowing the graph engine to be decoupled from LMDB during testing.
pub trait NeuronFetcher {
    fn fetch(&self, id: &NeuronId) -> Option<UniversalNeuron>;
}

/// Configuration for a graph traversal query.
#[derive(Debug, Clone)]
pub struct TraversalConfig {
    /// Maximum hops allowed.
    pub max_depth: usize,
    /// Minimum weight for an edge to be traversed.
    pub min_weight: f32,
    /// Maximum number of nodes to return (limits explosive branching).
    pub limit: usize,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            min_weight: 0.0,
            limit: 1000,
        }
    }
}

pub struct GraphEngine<F: NeuronFetcher> {
    fetcher: Arc<F>,
}

impl<F: NeuronFetcher> GraphEngine<F> {
    pub fn new(fetcher: Arc<F>) -> Self {
        Self { fetcher }
    }

    /// Performs a Breadth-First Search (BFS) to find the shortest paths or discover neighbors level by level.
    pub fn bfs_traverse(
        &self,
        start_node: NeuronId,
        config: &TraversalConfig,
    ) -> Result<Vec<(NeuronId, usize)>, GraphError> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        // Queue stores (NodeId, CurrentDepth)
        queue.push_back((start_node.clone(), 0));
        visited.insert(start_node.clone());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > config.max_depth {
                break; // Because it's BFS, everything remaining in queue is deeper
            }

            if results.len() >= config.limit {
                break;
            }

            // We don't yield the start node itself as a "found neighbor" unless requested,
            // but standard BFS includes it or skips it. Let's include it.
            results.push((current_id.clone(), depth));

            if depth == config.max_depth {
                continue; // Reached max depth, don't follow edges from here
            }

            if let Some(neuron) = self.fetcher.fetch(&current_id) {
                for edge in neuron.adjacency {
                    if edge.weight >= config.min_weight && !visited.contains(&edge.target_id) {
                        visited.insert(edge.target_id.clone());
                        queue.push_back((edge.target_id, depth + 1));
                    }
                }
            } else if depth == 0 {
                return Err(GraphError::NodeNotFound(current_id));
            }
        }

        Ok(results)
    }

    /// Performs a Depth-First Search (DFS) which is useful for finding full continuous paths.
    pub fn dfs_traverse(
        &self,
        start_node: NeuronId,
        config: &TraversalConfig,
    ) -> Result<Vec<(NeuronId, usize)>, GraphError> {
        let mut visited = HashSet::new();
        let mut results = Vec::new();

        self.dfs_recursive(start_node, 0, config, &mut visited, &mut results)?;

        Ok(results)
    }

    fn dfs_recursive(
        &self,
        current_id: NeuronId,
        depth: usize,
        config: &TraversalConfig,
        visited: &mut HashSet<NeuronId>,
        results: &mut Vec<(NeuronId, usize)>,
    ) -> Result<(), GraphError> {
        if depth > config.max_depth || results.len() >= config.limit {
            return Ok(());
        }

        visited.insert(current_id.clone());
        results.push((current_id.clone(), depth));

        if depth == config.max_depth {
            return Ok(());
        }

        if let Some(neuron) = self.fetcher.fetch(&current_id) {
            for edge in neuron.adjacency {
                if edge.weight >= config.min_weight && !visited.contains(&edge.target_id) {
                    self.dfs_recursive(edge.target_id, depth + 1, config, visited, results)?;
                }
            }
        } else if depth == 0 {
            return Err(GraphError::NodeNotFound(current_id));
        }

        Ok(())
    }
}
