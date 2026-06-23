use std::sync::Arc;
use axum::{Router, routing::{get, post}};

mod write;
mod read;
mod stream;
mod ws;
mod state;
mod shard_manager;
mod validate;
pub mod transit;

mod query;
mod graph;
mod media;
pub mod juju;
mod crispr;
pub mod booster;
pub mod dna;

#[cfg(test)]
mod shard_manager_tests;

pub use state::AppState;
pub use shard_manager::ShardManager;

/// Build the main Axum router with all CLUAIZD API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check for brain status
        .route("/health", get(|| async { axum::http::StatusCode::OK }))
        // Write a neuron (insert or update)
        .route("/neuron", post(write::handle_write))
        // Read a neuron by ID
        .route("/neuron/:id", get(read::handle_read))
        // Zero-copy media stream
        .route("/stream/:id", get(media::handle_media_stream))
        // Deep Graph Traversal
        .route("/graph/:id/traverse", get(graph::handle_traverse))
        .route("/graph/search/speculative", post(graph::handle_speculative_search))
        // JUJU Live Spatial State
        .route("/juju/state", get(juju::handle_juju_state))
        // CRISPR Surgery API
        .route("/crispr/clamp/:id", post(crispr::handle_clamp))
        .route("/crispr/clamp-vector/:id", post(crispr::handle_clamp_vector))
        .route("/crispr/force/:id", post(crispr::handle_force_edge))
        // WASM Booster Config
        .route("/booster/upload", post(booster::handle_upload_booster))
        .route("/booster/mode/:mode", post(booster::handle_change_mode))
        // DNA Execution Setup API
        .route("/dna/setup", post(dna::handle_dna_setup))
        // Query the database via on_index
        .route("/query", post(query::handle_query))
        // Ingest a raw voltage/sensor stream into the sensory shard
        .route("/ingest/stream", post(stream::handle_stream))
        // Validate proposed mutations in Deep Archer Sandbox
        .route("/sandbox/validate", post(validate::handle_validate))
        // WebSocket live telemetry & control pipeline
        .route("/ws/telemetry", get(ws::handle_ws))
        .with_state(state)
}

