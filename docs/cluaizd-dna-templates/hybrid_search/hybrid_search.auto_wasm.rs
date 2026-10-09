// ==============================================================================
// 🔍 Hybrid Search (Auto-WASM Rust Edition)
// ==============================================================================

use cluaizdb_dna_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct RrfConfig {
    vector_weight: f32,
    text_weight: f32,
    rrf_k: f32,
}

#[dna_hook(on_index)]
pub fn calculate_score(eval: &SearchEvaluation, config_json: &str) -> f32 {
    let config: RrfConfig = serde_json::from_str(config_json).unwrap();
    
    let v_score = 1.0 / (config.rrf_k + eval.vector_rank() as f32);
    let t_score = 1.0 / (config.rrf_k + eval.text_rank() as f32);
    
    (v_score * config.vector_weight) + (t_score * config.text_weight)
}
