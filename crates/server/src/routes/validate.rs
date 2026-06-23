use std::sync::Arc;
use axum::{Json, http::{StatusCode, HeaderMap}, response::IntoResponse, extract::State};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use cluaizd_types::{PayloadType, NeuronId, UniversalNeuron, NeuronDna, NeuronEdge};
use engine_lmdb::DeepArcherSandbox;
use crate::routes::AppState;
use crate::routes::write::WriteNeuronEdgeRequest;

/// Request payload for sandbox mutation validation.
#[derive(Debug, Deserialize)]
pub struct ValidateNeuronRequest {
    /// The original NeuronId to overwrite/simulate mutation on.
    pub original_id: String,
    /// Proposed raw payload.
    pub proposed_payload: String,
    /// Proposed dynamic coordinates footprint.
    pub proposed_vector_data: Vec<f32>,
    /// Hex model hash generating the proposed vector.
    pub model_creator_hash: String,
    /// proposed type payload.
    pub payload_type: String,
    /// Proposed dynamic rules.
    pub dna: Option<NeuronDna>,
    /// Proposed dynamic weighted adjacency edges.
    pub adjacency: Option<Vec<WriteNeuronEdgeRequest>>,
}

/// Response returned on validation.
#[derive(Debug, Serialize)]
pub struct ValidateNeuronResponse {
    pub original_id: String,
    pub is_safe: bool,
    pub message: &'static str,
}

/// `POST /sandbox/validate` — Ephemerally validates proposed edits before WAL persistence.
pub async fn handle_validate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ValidateNeuronRequest>,
) -> impl IntoResponse {
    // 1. Validate UUID structure
    let uuid = match Uuid::parse_str(&body.original_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid original_id format (expected UUID v7)" })),
            ).into_response();
        }
    };
    let original_id = NeuronId::from_bytes(*uuid.as_bytes());

    // 2. Resolve Shard Env
    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, tenant = %tenant_id, "Failed to resolve shard environment");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to open tenant shard: {}", e) })),
            ).into_response();
        }
    };

    // 3. Setup Volatile Deep Archer Sandbox
    let sandbox = match DeepArcherSandbox::new(Arc::clone(&shard.env)) {
        Ok(sb) => sb,
        Err(e) => {
            warn!(error = %e, "Failed to instantiate Deep Archer sandbox");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Sandbox initialization failed: {}", e) })),
            ).into_response();
        }
    };

    // 4. Parse model creator hash
    let model_hash_bytes = match hex::decode(&body.model_creator_hash) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "model_creator_hash must be a 64-char hex string (SHA-256)" })),
            ).into_response();
        }
    };

    let payload_type = match body.payload_type.as_str() {
        "text"           => PayloadType::Text,
        "audio"          => PayloadType::Audio,
        "video"          => PayloadType::Video,
        "code"           => PayloadType::Code,
        "voltage_stream" => PayloadType::VoltageStream,
        _                => PayloadType::Binary,
    };

    // Create proposed neuron utilizing original id coordinates
    let mut proposed_neuron = UniversalNeuron::new(
        Bytes::from(body.proposed_payload.into_bytes()),
        body.proposed_vector_data,
        model_hash_bytes,
        payload_type,
    );
    proposed_neuron.id = original_id;
    proposed_neuron.dna = body.dna;

    if let Some(req_edges) = body.adjacency {
        let mut edges = Vec::new();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        for e in req_edges {
            if let Ok(uuid) = uuid::Uuid::parse_str(&e.target_id) {
                edges.push(NeuronEdge {
                    target_id: NeuronId::from_bytes(*uuid.as_bytes()),
                    weight: e.weight,
                    last_accessed_ns: now_ns,
                });
            }
        }
        proposed_neuron.adjacency = edges;
    }

    // 5. Run simulation safety check
    match sandbox.simulate_mutation(original_id, &proposed_neuron) {
        Ok(true) => (
            StatusCode::OK,
            Json(ValidateNeuronResponse {
                original_id: body.original_id,
                is_safe: true,
                message: "Synaptic weight configuration safe to commit.",
            }),
        ).into_response(),
        Ok(false) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ValidateNeuronResponse {
                original_id: body.original_id,
                is_safe: false,
                message: "Deep Archer Blocked Mutation: Structural Weight Crash or unbalanced coordinates.",
            }),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Sandbox simulation execution error: {}", e) })),
        ).into_response(),
    }
}
