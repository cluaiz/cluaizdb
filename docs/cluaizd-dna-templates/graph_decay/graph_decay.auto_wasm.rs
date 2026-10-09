// ==============================================================================
// 🕸️ Graph Edge Decay (Auto-WASM Rust Edition)
// ==============================================================================

use cluaizdb_dna_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct DecayConfig {
    decay_factor: f32,
    pruning_threshold: f32,
}

#[dna_hook(on_lifecycle)]
pub fn prune_edges(neuron: &Neuron, _time: u64, config_json: &str) -> LifecycleDecision {
    let config: DecayConfig = serde_json::from_str(config_json).unwrap();
    
    for edge in neuron.edges() {
        let new_weight = edge.weight * config.decay_factor;
        
        if new_weight < config.pruning_threshold {
            ctx::query().disconnect(neuron.id(), edge.target_id).execute();
        } else {
            ctx::query().update_edge_weight(neuron.id(), edge.target_id, new_weight).execute();
        }
    }

    LifecycleDecision::None
}
