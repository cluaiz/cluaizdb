use cluaizdb_index_mvhsnw::{HnswIndex, CosineDistance};
use cluaizdb_types::NeuronId;
use std::sync::Arc;

#[test]
fn test_hnsw_vector_integration() {
    let index = Arc::new(HnswIndex::<CosineDistance>::new());
    
    let id1 = NeuronId::new();
    let id2 = NeuronId::new();
    
    index.insert(id1.clone(), vec![1.0, 0.0, 0.0]);
    index.insert(id2.clone(), vec![0.0, 1.0, 0.0]);

    let results = index.search_knn(&[1.0, 0.0, 0.0], 1);
    assert_eq!(results[0].0, id1);
}
