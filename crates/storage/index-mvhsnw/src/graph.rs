use crate::node::HnswNode;
use crate::distance::DistanceMetric;
use cluaizdb_types::NeuronId;
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;
use rand::Rng;

const DEFAULT_M: usize = 16;
const DEFAULT_EF_CONSTRUCTION: usize = 100;

#[derive(Clone, Copy)]
struct Element {
    distance: f32,
    index: usize,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.index == other.index
    }
}
impl Eq for Element {}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Min-heap behavior for BinaryHeap (reverse standard cmp)
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// The core HNSW Graph structure.
pub struct HnswIndex<D: DistanceMetric> {
    mutation_lock: parking_lot::Mutex<()>,
    nodes: RwLock<Vec<Arc<HnswNode>>>,
    entry_point: RwLock<Option<usize>>,
    max_level: RwLock<usize>,
    m: usize,
    m_max: usize,
    m_max_0: usize,
    level_mult: f64,
    _marker: std::marker::PhantomData<D>,
}

impl<D: DistanceMetric> HnswIndex<D> {
    pub fn new() -> Self {
        let m = DEFAULT_M;
        Self {
            mutation_lock: parking_lot::Mutex::new(()),
            nodes: RwLock::new(Vec::new()),
            entry_point: RwLock::new(None),
            max_level: RwLock::new(0),
            m,
            m_max: m,
            m_max_0: m * 2,
            level_mult: 1.0 / (m as f64).ln(),
            _marker: std::marker::PhantomData,
        }
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let unif: f64 = rng.gen_range(0.0..1.0);
        (-unif.ln() * self.level_mult) as usize
    }

    /// Inserts a new vector into the HNSW graph.
    /// Serializes topological mutations via `mutation_lock` to prevent partially initialized graph reads.
    pub fn insert(&self, id: NeuronId, vector: Vec<f32>) {
        let _mutation_guard = self.mutation_lock.lock();

        let level = self.random_level();
        let new_node_idx;
        let entry_point_idx;
        let max_level;

        {
            let mut nodes = self.nodes.write();
            new_node_idx = nodes.len();
            nodes.push(Arc::new(HnswNode::new(id, vector.clone(), level)));
        }

        {
            let ep = self.entry_point.read();
            entry_point_idx = *ep;
            max_level = *self.max_level.read();
        }

        if entry_point_idx.is_none() {
            // First node
            *self.entry_point.write() = Some(new_node_idx);
            *self.max_level.write() = level;
            return;
        }

        let mut curr_obj = entry_point_idx.unwrap();
        
        // Search Phase 1: From max_level down to level+1, find the nearest element
        let nodes = self.nodes.read();
        for lc in (level + 1..=max_level).rev() {
            loop {
                let mut changed = false;
                let Some(curr_node) = nodes.get(curr_obj) else { break; };
                let dist = D::calculate(&vector, &curr_node.vector);
                
                let neighbors = curr_node.connections[lc].read();
                for &neighbor in neighbors.iter() {
                    let Some(neighbor_node) = nodes.get(neighbor) else { continue; };
                    let n_dist = D::calculate(&vector, &neighbor_node.vector);
                    if n_dist < dist {
                        curr_obj = neighbor;
                        changed = true;
                        break;
                    }
                }
                if !changed { break; }
            }
        }

        // Search Phase 2: Insert into levels from `level` down to 0
        for lc in (0..=std::cmp::min(level, max_level)).rev() {
            let mut w = self.search_layer(&vector, curr_obj, DEFAULT_EF_CONSTRUCTION, lc, &nodes);
            let neighbors = self.select_neighbors(&mut w, if lc == 0 { self.m_max_0 } else { self.m_max });
            
            // Add bidirectional connections with bounds check
            for &neighbor_idx in &neighbors {
                if let Some(new_node) = nodes.get(new_node_idx) {
                    new_node.connections[lc].write().push(neighbor_idx);
                }
                if let Some(neighbor_node) = nodes.get(neighbor_idx) {
                    let mut n_connections = neighbor_node.connections[lc].write();
                    let max_links = if lc == 0 { self.m_max_0 } else { self.m_max };
                    if n_connections.len() < max_links {
                        n_connections.push(new_node_idx);
                    }
                }
            }
        }

        if level > max_level {
            *self.max_level.write() = level;
            *self.entry_point.write() = Some(new_node_idx);
        }
    }

    /// Searches the graph for the K nearest neighbors
    pub fn search_knn(&self, query: &[f32], k: usize) -> Vec<(NeuronId, f32)> {
        let ep = *self.entry_point.read();
        if ep.is_none() {
            return Vec::new();
        }
        
        let mut curr_obj = ep.unwrap();
        let max_level = *self.max_level.read();
        let nodes = self.nodes.read();

        // Drop down to level 0
        for lc in (1..=max_level).rev() {
            loop {
                let mut changed = false;
                let Some(curr_node) = nodes.get(curr_obj) else { break; };
                let dist = D::calculate(query, &curr_node.vector);
                
                let neighbors = curr_node.connections[lc].read();
                for &neighbor in neighbors.iter() {
                    let Some(neighbor_node) = nodes.get(neighbor) else { continue; };
                    let n_dist = D::calculate(query, &neighbor_node.vector);
                    if n_dist < dist {
                        curr_obj = neighbor;
                        changed = true;
                        break;
                    }
                }
                if !changed { break; }
            }
        }

        // Search level 0 with EF = max(k, EF_CONSTRUCTION)
        let ef = std::cmp::max(k, DEFAULT_EF_CONSTRUCTION);
        let mut results = self.search_layer(query, curr_obj, ef, 0, &nodes);
        
        let mut final_results = Vec::new();
        while let Some(el) = results.pop() {
            if let Some(node) = nodes.get(el.index) {
                final_results.push((node.id, el.distance));
            }
            if final_results.len() == k {
                break;
            }
        }
        final_results
    }

    fn search_layer(&self, query: &[f32], ep: usize, ef: usize, lc: usize, nodes: &[Arc<HnswNode>]) -> BinaryHeap<Element> {
        let mut v = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut w = BinaryHeap::new(); // Results, reverse ord (max-heap behavior for popping farthest)

        let Some(ep_node) = nodes.get(ep) else { return w; };
        let dist = D::calculate(query, &ep_node.vector);
        v.insert(ep);
        candidates.push(Element { distance: dist, index: ep });
        // Max-heap wrapper (inverse distance)
        w.push(Element { distance: -dist, index: ep });

        while let Some(c) = candidates.pop() {
            let f = w.peek().unwrap();
            if c.distance > -f.distance {
                break;
            }

            let Some(cand_node) = nodes.get(c.index) else { continue; };
            let neighbors = cand_node.connections[lc].read();
            for &e in neighbors.iter() {
                if !v.contains(&e) {
                    v.insert(e);
                    let Some(neighbor_node) = nodes.get(e) else { continue; };
                    let e_dist = D::calculate(query, &neighbor_node.vector);
                    let f = w.peek().unwrap();
                    
                    if e_dist < -f.distance || w.len() < ef {
                        candidates.push(Element { distance: e_dist, index: e });
                        w.push(Element { distance: -e_dist, index: e });
                        if w.len() > ef {
                            w.pop();
                        }
                    }
                }
            }
        }

        // Convert back to min-heap of actual distances
        let mut res = BinaryHeap::new();
        for e in w {
            res.push(Element { distance: -e.distance, index: e.index });
        }
        res
    }

    fn select_neighbors(&self, candidates: &mut BinaryHeap<Element>, m: usize) -> Vec<usize> {
        let mut res = Vec::new();
        while let Some(c) = candidates.pop() {
            res.push(c.index);
            if res.len() == m {
                break;
            }
        }
        res
    }
}
