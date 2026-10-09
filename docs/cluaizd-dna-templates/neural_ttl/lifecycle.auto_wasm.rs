// ==============================================================================
// 🕰️ Biological Forgetting (Auto-WASM Rust Edition)
// ==============================================================================

use cluaizdb_dna_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct LifecycleConfig {
    warm_threshold_days: u64,
    cold_threshold_days: u64,
}

#[dna_hook(on_lifecycle)]
pub fn check_lifecycle(neuron: &Neuron, current_time: u64, config_json: &str) -> LifecycleDecision {
    let config: LifecycleConfig = serde_json::from_str(config_json).unwrap();
    let age_ms = current_time.saturating_sub(neuron.last_accessed());
    let days_idle = age_ms / (1000 * 60 * 60 * 24);

    if days_idle > config.cold_threshold_days {
        return LifecycleDecision::Compress(CompressionAlgo::Zstd, 19);
    }

    if days_idle > config.warm_threshold_days {
        return LifecycleDecision::PurgePayloadKeepVector;
    }

    if days_idle < 1 {
        return LifecycleDecision::ReinforceEdges(1.05);
    }

    LifecycleDecision::None
}
