// ==============================================================================
// 🛡️ Deep Archer Validation Matrix (Auto-WASM Rust Edition)
// ==============================================================================
// For absolute maximum performance (C-level execution speed).
// Cluaizdb will automatically compile this to WASM upon upload.
// ------------------------------------------------------------------------------

use cluaizdb_dna_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct ArcherConfig {
    enable_external_grounding: bool,
    grounding_api_url: String,
    forbidden_keywords: Vec<String>,
}

#[dna_hook(on_write)]
pub fn validate_neuron(payload: &Payload, metadata: &Metadata, config_json: &str) -> WriteDecision {
    let config: ArcherConfig = serde_json::from_str(config_json).unwrap();

    // 1. Novelty Check (Vector math is blazingly fast in WASM)
    let duplicates = ctx::query()
        .find_similar(payload.vector_data(), Cosine, 0.98)
        .limit(1)
        .execute();
        
    if !duplicates.is_empty() {
        return WriteDecision::Reject("Deep Archer: Novelty Check failed".to_string());
    }

    // 2. Alignment Check
    let text = payload.as_text().to_lowercase();
    for word in &config.forbidden_keywords {
        if text.contains(word) {
            return WriteDecision::Reject("Deep Archer: Alignment Check failed".to_string());
        }
    }

    // 3. Grounding Check
    if config.enable_external_grounding {
        let res = ctx::fetch_post(&config.grounding_api_url, payload.as_bytes());
        if res.status != 200 || res.json_get("is_hallucination") == "true" {
            return WriteDecision::Reject("Deep Archer: Grounding Check failed".to_string());
        }
    }

    WriteDecision::Approve
}
