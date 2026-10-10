use crate::error::Result;
use crate::traits::StorageSubstrate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    Outbound,
    Inbound,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalConfig {
    pub max_depth: usize,
    pub min_weight: f32,
    pub limit: usize,
    pub edge_type: Option<u32>,
    pub direction: TraversalDirection,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            min_weight: 0.0,
            limit: 1000,
            edge_type: None,
            direction: TraversalDirection::Outbound,
        }
    }
}

/// A node discovered in multi-hop traversal with depth and path weight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraversalNode {
    pub node_id: u128,
    pub depth: usize,
    pub cumulative_weight: f32,
}

/// Internal min-heap state for Dijkstra's shortest path.
#[derive(Copy, Clone, PartialEq)]
struct DijkstraState {
    cost: f32,
    node: u128,
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct GraphTraversal;

impl GraphTraversal {
    /// Multi-hop BFS Traversal with cycle protection, weight filtering, and depth bounding.
    pub fn bfs<S: StorageSubstrate + ?Sized>(
        substrate: &S,
        start_id: u128,
        config: &TraversalConfig,
    ) -> Result<Vec<TraversalNode>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(start_id);
        queue.push_back((start_id, 0usize, 0.0f32));

        while let Some((curr_id, depth, weight)) = queue.pop_front() {
            if results.len() >= config.limit {
                break;
            }

            if depth > 0 {
                results.push(TraversalNode {
                    node_id: curr_id,
                    depth,
                    cumulative_weight: weight,
                });
            }

            if depth >= config.max_depth {
                continue;
            }

            let mut neighbors: Vec<(u128, f32)> = Vec::new();

            // Outbound edges
            if matches!(config.direction, TraversalDirection::Outbound | TraversalDirection::Both) {
                if let Ok(edges) = substrate.get_outbound_edges(curr_id, config.edge_type) {
                    for (dst, meta) in edges {
                        if meta.weight >= config.min_weight {
                            neighbors.push((dst, meta.weight));
                        }
                    }
                }
            }

            // Inbound edges
            if matches!(config.direction, TraversalDirection::Inbound | TraversalDirection::Both) {
                if let Ok(srcs) = substrate.get_inbound_edges(curr_id, config.edge_type) {
                    for src in srcs {
                        neighbors.push((src, 1.0));
                    }
                }
            }

            for (next_id, edge_weight) in neighbors {
                if visited.insert(next_id) {
                    queue.push_back((next_id, depth + 1, weight + edge_weight));
                }
            }
        }

        Ok(results)
    }

    /// Shortest path between two nodes using Dijkstra's algorithm.
    pub fn shortest_path<S: StorageSubstrate + ?Sized>(
        substrate: &S,
        start_id: u128,
        target_id: u128,
        config: &TraversalConfig,
    ) -> Result<Option<Vec<u128>>> {
        if start_id == target_id {
            return Ok(Some(vec![start_id]));
        }

        let mut distances: HashMap<u128, f32> = HashMap::new();
        let mut predecessors: HashMap<u128, u128> = HashMap::new();
        let mut heap = BinaryHeap::new();

        distances.insert(start_id, 0.0);
        heap.push(DijkstraState { cost: 0.0, node: start_id });

        while let Some(DijkstraState { cost, node }) = heap.pop() {
            if node == target_id {
                let mut path = Vec::new();
                let mut curr = target_id;
                path.push(curr);
                while let Some(&pred) = predecessors.get(&curr) {
                    path.push(pred);
                    curr = pred;
                    if curr == start_id {
                        break;
                    }
                }
                path.reverse();
                return Ok(Some(path));
            }

            if let Some(&d) = distances.get(&node) {
                if cost > d {
                    continue;
                }
            }

            let edges = substrate.get_outbound_edges(node, config.edge_type).unwrap_or_default();
            for (next_node, meta) in edges {
                if meta.weight < config.min_weight {
                    continue;
                }
                let edge_cost = if meta.weight > 0.0 { 1.0 / meta.weight } else { 1.0 };
                let next_cost = cost + edge_cost;

                let is_shorter = match distances.get(&next_node) {
                    Some(&curr_d) => next_cost < curr_d,
                    None => true,
                };

                if is_shorter {
                    distances.insert(next_node, next_cost);
                    predecessors.insert(next_node, node);
                    heap.push(DijkstraState { cost: next_cost, node: next_node });
                }
            }
        }

        Ok(None)
    }
}
