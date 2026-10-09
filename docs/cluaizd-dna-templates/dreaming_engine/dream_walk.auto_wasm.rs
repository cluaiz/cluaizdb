// ==============================================================================
// 🧠 Dreaming Engine (Auto-WASM Rust Edition)
// ==============================================================================

use cluaizdb_dna_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct DreamConfig {
    walk_depth: u32,
    eureka_threshold: f32,
}

#[dna_hook(on_dream)]
pub fn dream_cycle(context: &IdleContext, config_json: &str) -> DreamDecision {
    if context.idle_budget_ms < 500 {
        return DreamDecision::Skipped;
    }

    let config: DreamConfig = serde_json::from_str(config_json).unwrap();

    let anchor = ctx::query().find_all().sort_by("last_accessed").desc().limit(1).execute().first();
    let anchor = match anchor {
        Some(node) => node,
        None => return DreamDecision::Skipped,
    };

    let destination = ctx::query()
        .traverse(anchor.id(), config.walk_depth, TraverseStrategy::Stochastic)
        .last();

    if anchor.id() == destination.id() {
        return DreamDecision::NoInsight;
    }

    let sim = ctx::math::cosine(anchor.vector_data(), destination.vector_data());
    
    if sim > config.eureka_threshold {
        ctx::query().connect(anchor.id(), destination.id(), sim, "hypothesis").execute();
        ctx::log::audit("EUREKA", &format!("Linked {} and {}", anchor.id(), destination.id()));
        return DreamDecision::InsightGenerated;
    }

    DreamDecision::Complete
}
