pub mod distance;
pub mod graph;
pub mod node;

pub use distance::{CosineDistance, DistanceMetric, L2Distance};
pub use graph::HnswIndex;
pub use node::HnswNode;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_cosine_vector_ann_search() {
        let index: HnswIndex<CosineDistance> = HnswIndex::new();

        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let v3 = vec![0.9, 0.1, 0.0]; // Close to v1

        index.insert(1, v1);
        index.insert(2, v2);
        index.insert(3, v3);

        // Query vector close to v1 and v3
        let query = vec![1.0, 0.05, 0.0];
        let results = index.search_knn(&query, 2);

        assert_eq!(results.len(), 2);
        // Result 0 should be either 1 or 3 (closest)
        let top_ids: Vec<u128> = results.iter().map(|(id, _)| *id).collect();
        assert!(top_ids.contains(&1));
        assert!(top_ids.contains(&3));
    }

    #[test]
    fn test_concurrent_vector_insertion() {
        let index = Arc::new(HnswIndex::<L2Distance>::new());
        let mut handles = Vec::new();

        for i in 0..64 {
            let idx = Arc::clone(&index);
            handles.push(std::thread::spawn(move || {
                let vec = vec![i as f32, (i * 2) as f32, (i * 3) as f32];
                idx.insert(i as u128, vec);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let query = vec![10.0, 20.0, 30.0];
        let results = index.search_knn(&query, 5);
        assert_eq!(results.len(), 5);
        // Node 10 should be closest (distance ~ 0.0)
        assert_eq!(results[0].0, 10);
        assert!(results[0].1 < 0.001);
    }
}
