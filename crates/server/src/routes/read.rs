use std::sync::Arc;
use axum::{Json, extract::{Path, State}, http::{StatusCode, HeaderMap}, response::IntoResponse};
use uuid::Uuid;

use cluaizdb_types::NeuronId;
use crate::routes::AppState;

/// `GET /neuron/{id}` — Fetch a neuron by its ID.
///
/// Optionally, the caller can pass an `X-Model-Hash` header.
/// If provided and it doesn't match the stored hash, a 400 Bad Request or 500 status is returned.
pub async fn handle_read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    // Validate the UUID string format.
    let uuid = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid neuron ID format (expected UUID v7)" })),
            ).into_response();
        }
    };

    let neuron_id = NeuronId::from_bytes(*uuid.as_bytes());

    // Resolve tenant ID from headers
    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    // Open/retrieve sharded database
    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to open tenant shard: {}", e) })),
            ).into_response();
        }
    };

    // Extract X-Model-Hash header if present
    let mut query_model_hash = None;
    if let Some(val) = headers.get("x-model-hash") {
        let hash_str = match val.to_str() {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid x-model-hash header format" })),
                ).into_response();
            }
        };

        match hex::decode(hash_str) {
            Ok(decoded) if decoded.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&decoded);
                query_model_hash = Some(arr);
            }
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "x-model-hash must be a 64-char hex string" })),
                ).into_response();
            }
        }
    }

    match engine_lmdb::read_neuron(&shard.env, neuron_id, query_model_hash) {
        Ok(neuron) => (StatusCode::OK, Json(neuron)).into_response(),
        Err(cluaizdb_errors::StorageError::NeuronNotFound(_)) => {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("neuron {} not found", id_str) })),
            ).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ).into_response()
        }
    }
}

