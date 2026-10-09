use cluaizdb_graph_engine::{GraphEngine, NeuronFetcher, TraversalConfig};
use cluaizdb_types::{NeuronId, UniversalNeuron, NeuronEdge, PayloadType};
use bytes::Bytes;
use std::sync::Arc;
use std::collections::HashMap;

struct MockFetcher {
    neurons: HashMap<NeuronId, UniversalNeuron>,
}
impl NeuronFetcher for MockFetcher {
    fn fetch(&self, id: &NeuronId) -> Option<UniversalNeuron> {
        self.neurons.get(id).cloned()
    }
}

#[test]
fn test_graph_traversal_integration() {
    let mut fetcher = MockFetcher { neurons: HashMap::new() };
    
    let root = NeuronId::new();
    let leaf = NeuronId::new();

    let mut root_neuron = UniversalNeuron::new(Bytes::new(), [0.0; 16].to_vec(), [0; 32], PayloadType::Text);
    root_neuron.id = root.clone();
    root_neuron.adjacency = vec![NeuronEdge { target_id: leaf.clone(), weight: 1.0, last_accessed_ns: 0 }];
    
    fetcher.neurons.insert(root.clone(), root_neuron);

    let engine = GraphEngine::new(Arc::new(fetcher));
    let config = TraversalConfig { max_depth: 1, min_weight: 0.0, limit: 10 };

    let res = engine.bfs_traverse(root, &config).unwrap();
    assert_eq!(res.len(), 2);
}
