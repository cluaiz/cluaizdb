use crate::distance::DistanceMetric;
use crate::node::HnswNode;
use parking_lot::RwLock;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

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

/// The core HNSW Graph structure for Approximate Nearest Neighbor vector search.
pub struct HnswIndex<D: DistanceMetric> {
    mutation_lock: parking_lot::Mutex<()>,
    nodes: RwLock<Vec<Arc<HnswNode>>>,
    entry_point: RwLock<Option<usize>>,
    max_level: RwLock<usize>,
    _m: usize,
    m_max: usize,
    m_max_0: usize,
    level_mult: f64,
    _marker: std::marker::PhantomData<D>,
}

impl<D: DistanceMetric> Default for HnswIndex<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: DistanceMetric> HnswIndex<D> {
    pub fn new() -> Self {
        let m = DEFAULT_M;
        Self {
            mutation_lock: parking_lot::Mutex::new(()),
            nodes: RwLock::new(Vec::new()),
            entry_point: RwLock::new(None),
            max_level: RwLock::new(0),
            _m: m,
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

    /// Inserts a new vector with an abstract `u128` record ID into the HNSW graph.
    pub fn insert(&self, id: u128, vector: Vec<f32>) {
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
            // First node in the graph
            *self.entry_point.write() = Some(new_node_idx);
            *self.max_level.write() = level;
            return;
        }

        let mut curr_obj = entry_point_idx.unwrap();

        // Search Phase 1: From max_level down to level+1, find nearest element via steepest descent
        let nodes = self.nodes.read();
        for lc in (level + 1..=max_level).rev() {
            loop {
                let mut changed = false;
                let Some(curr_node) = nodes.get(curr_obj) else { break; };
                let mut dist = D::calculate(&vector, &curr_node.vector);
                let mut best_neighbor = curr_obj;

                let neighbors = curr_node.connections[lc].read();
                for &neighbor in neighbors.iter() {
                    let Some(neighbor_node) = nodes.get(neighbor) else { continue; };
                    let n_dist = D::calculate(&vector, &neighbor_node.vector);
                    if n_dist < dist {
                        dist = n_dist;
                        best_neighbor = neighbor;
                        changed = true;
                    }
                }
                if changed {
                    curr_obj = best_neighbor;
                } else {
                    break;
                }
            }
        }

        // Search Phase 2: Insert into levels from `level` down to 0
        for lc in (0..=std::cmp::min(level, max_level)).rev() {
            let mut w = self.search_layer(&vector, curr_obj, DEFAULT_EF_CONSTRUCTION, lc, &nodes);
            let neighbors = self.select_neighbors(&mut w, if lc == 0 { self.m_max_0 } else { self.m_max });

            // Add bidirectional connections with safe bounds checks
            for &neighbor_idx in &neighbors {
                if let Some(new_node) = nodes.get(new_node_idx) {
                    new_node.connections[lc].write().push(neighbor_idx);
                }
                if let Some(neighbor_node) = nodes.get(neighbor_idx) {
                    let mut n_connections = neighbor_node.connections[lc].write();
                    let max_links = if lc == 0 { self.m_max_0 } else { self.m_max };
                    if n_connections.len() < max_links {
                        n_connections.push(new_node_idx);
                    } else {
                        // Edge pruning: replace furthest neighbor if new_node is closer
                        let mut furthest_idx = 0;
                        let mut max_dist = f32::MIN;
                        for (i, &exist_idx) in n_connections.iter().enumerate() {
                            if let Some(en) = nodes.get(exist_idx) {
                                let d = D::calculate(&neighbor_node.vector, &en.vector);
                                if d > max_dist {
                                    max_dist = d;
                                    furthest_idx = i;
                                }
                            }
                        }
                        let new_dist = D::calculate(&neighbor_node.vector, &vector);
                        if new_dist < max_dist {
                            n_connections[furthest_idx] = new_node_idx;
                        }
                    }
                }
            }

            // Descend: update entry point for next lower layer to the closest neighbor
            if let Some(&closest) = neighbors.first() {
                curr_obj = closest;
            }
        }

        if level > max_level {
            *self.max_level.write() = level;
            *self.entry_point.write() = Some(new_node_idx);
        }
    }

    /// Searches the graph for the K nearest neighbors.
    /// Returns vector of `(record_id: u128, distance: f32)` sorted by proximity.
    pub fn search_knn(&self, query: &[f32], k: usize) -> Vec<(u128, f32)> {
        let ep = *self.entry_point.read();
        if ep.is_none() {
            return Vec::new();
        }

        let mut curr_obj = ep.unwrap();
        let max_level = *self.max_level.read();
        let nodes = self.nodes.read();

        // Drop down layers to level 0 via steepest descent
        for lc in (1..=max_level).rev() {
            loop {
                let mut changed = false;
                let Some(curr_node) = nodes.get(curr_obj) else { break; };
                let mut dist = D::calculate(query, &curr_node.vector);
                let mut best_neighbor = curr_obj;

                let neighbors = curr_node.connections[lc].read();
                for &neighbor in neighbors.iter() {
                    let Some(neighbor_node) = nodes.get(neighbor) else { continue; };
                    let n_dist = D::calculate(query, &neighbor_node.vector);
                    if n_dist < dist {
                        dist = n_dist;
                        best_neighbor = neighbor;
                        changed = true;
                    }
                }
                if changed {
                    curr_obj = best_neighbor;
                } else {
                    break;
                }
            }
        }

        // Search level 0
        let ef = std::cmp::max(k, DEFAULT_EF_CONSTRUCTION);
        let mut results = self.search_layer(query, curr_obj, ef, 0, &nodes);

        let mut final_results: Vec<(u128, f32)> = Vec::new();
        while let Some(el) = results.pop() {
            if let Some(node) = nodes.get(el.index) {
                let actual_dist = D::calculate(query, &node.vector);
                final_results.push((node.id, actual_dist));
            }
        }
        final_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        final_results.truncate(k);
        final_results
    }

    fn search_layer(
        &self,
        query: &[f32],
        ep: usize,
        ef: usize,
        lc: usize,
        nodes: &[Arc<HnswNode>],
    ) -> BinaryHeap<Element> {
        let mut v = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut w = BinaryHeap::new();

        let Some(ep_node) = nodes.get(ep) else { return w; };
        let dist = D::calculate(query, &ep_node.vector);
        v.insert(ep);
        candidates.push(Element { distance: dist, index: ep });
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
