use std::sync::Arc;
use axum::{Json, extract::{Path, State}, http::{StatusCode, HeaderMap}, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use cluaizdb_types::NeuronId;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ClampRequest {
    pub key: String,
    pub value: f64,
}

#[derive(Deserialize)]
pub struct ForceEdgeRequest {
    pub target_id: String,
    pub weight: f32,
}

#[derive(Serialize)]
pub struct CrisprResponse {
    pub status: String,
    pub message: String,
}

/// `POST /crispr/clamp/{id}` — Manually lock a DNA parameter (e.g. Risk Coefficient) to a strict value.
pub async fn handle_clamp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(payload): Json<ClampRequest>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid neuron ID format").into_response(),
    };

    let neuron_id = NeuronId::from_bytes(*uuid.as_bytes());

    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open shard").into_response(),
    };

    let mut neuron = match engine_lmdb::read_neuron(&shard.env, neuron_id, None) {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Neuron not found").into_response(),
    };

    if let Some(mut dna) = neuron.dna.take() {
        if let Some(params) = dna.parameters.as_object_mut() {
            params.insert(payload.key.clone(), serde_json::json!(payload.value));
        } else {
            let mut new_params = serde_json::Map::new();
            new_params.insert(payload.key.clone(), serde_json::json!(payload.value));
            dna.parameters = serde_json::Value::Object(new_params);
        }
        neuron.dna = Some(dna);
        
        if let Err(e) = engine_lmdb::writer::write_neuron(&shard.env, &neuron) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save clamped neuron: {}", e)).into_response();
        }

        (StatusCode::OK, Json(CrisprResponse {
            status: "success".to_string(),
            message: format!("Clamped parameter '{}' to {}", payload.key, payload.value),
        })).into_response()
    } else {
        (StatusCode::BAD_REQUEST, "Neuron has no DNA attached to clamp").into_response()
    }
}

/// `POST /crispr/force/{id}` — Inject an un-deletable edge into a neuron's adjacency list.
pub async fn handle_force_edge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(payload): Json<ForceEdgeRequest>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid neuron ID format").into_response(),
    };
    
    let target_uuid = match Uuid::parse_str(&payload.target_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid target ID format").into_response(),
    };

    let neuron_id = NeuronId::from_bytes(*uuid.as_bytes());
    let target_id = NeuronId::from_bytes(*target_uuid.as_bytes());

    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open shard").into_response(),
    };

    let mut neuron = match engine_lmdb::read_neuron(&shard.env, neuron_id, None) {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Neuron not found").into_response(),
    };

    // Prevent duplicates
    if !neuron.adjacency.iter().any(|e| e.target_id == target_id) {
        neuron.adjacency.push(cluaizdb_types::NeuronEdge {
            target_id,
            weight: payload.weight,
            last_accessed_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        });
        
        if let Err(e) = engine_lmdb::writer::write_neuron(&shard.env, &neuron) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save forced edge: {}", e)).into_response();
        }
        
        // Update Spatial Map
        let mut map = state.spatial_map.write().await;
        let edges: Vec<String> = neuron.adjacency.iter().map(|e| e.target_id.to_string()).collect();
        map.edges.insert(neuron.id.to_string(), edges);
    }

    (StatusCode::OK, Json(CrisprResponse {
        status: "success".to_string(),
        message: format!("Forced synaptic edge to target '{}'", payload.target_id),
    })).into_response()
}

#[derive(Deserialize)]
pub struct ClampVectorRequest {
    /// The index of the 16-dimensional tensor to clamp to 0.0
    pub index: usize,
}

/// `POST /crispr/clamp-vector/{id}` — Manually lock a structural vector tensor to 0.0 (Trait Clamping).
/// This instantly lobotomizes specific operational affordances (e.g., removing a robot's ability to punch).
pub async fn handle_clamp_vector(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(payload): Json<ClampVectorRequest>,
) -> impl IntoResponse {
    if payload.index >= 16 {
        return (StatusCode::BAD_REQUEST, "Vector index out of bounds. Must be 0-15.").into_response();
    }

    let uuid = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid neuron ID format").into_response(),
    };

    let neuron_id = NeuronId::from_bytes(*uuid.as_bytes());

    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open shard").into_response(),
    };

    let mut neuron = match engine_lmdb::read_neuron(&shard.env, neuron_id, None) {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Neuron not found").into_response(),
    };

    // Lobotomize the specific trait tensor by clamping it to 0.0
    neuron.vector_data[payload.index] = 0.0;
    
    // Write directly to LMDB bypassing any schema validation rules
    if let Err(e) = engine_lmdb::write_neuron(&shard.env, &neuron) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save vector clamped neuron: {}", e)).into_response();
    }

    (StatusCode::OK, Json(CrisprResponse {
        status: "success".to_string(),
        message: format!("Clamped tensor index {} to 0.0", payload.index),
    })).into_response()
}
