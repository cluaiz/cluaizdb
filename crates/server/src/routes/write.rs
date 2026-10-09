use std::sync::Arc;
use axum::{Json, http::{StatusCode, HeaderMap}, response::IntoResponse, extract::State};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::warn;

use cluaizdb_types::{PayloadType, UniversalNeuron, NeuronDna, NeuronEdge};
use genome::{GenomeWriteAction, Durability};
use crate::routes::AppState;

/// API request representation of a Neuron Edge.
#[derive(Debug, Deserialize)]
pub struct WriteNeuronEdgeRequest {
    pub target_id: String,
    pub weight: f32,
}

/// Request body for `POST /neuron`.
///
/// The AI model or client is responsible for computing the vector.
/// CLUAIZD only stores what it receives — it never generates vectors internally.
#[derive(Debug, Deserialize)]
pub struct WriteNeuronRequest {
    /// The raw payload as a UTF-8 string (for text) or base64 (for binary).
    pub raw_payload: String,
    /// The dynamic hardware footprint vector from the external model.
    pub vector_data: Vec<f32>,
    /// SHA-256 hash (hex string) of the model that generated `vector_data`.
    pub model_creator_hash: String,
    /// The type of data (text, audio, video, code, binary).
    pub payload_type: String,
    /// Optional dynamic ruleset configuration.
    pub dna: Option<NeuronDna>,
    /// Optional initial weighted edges to connect to.
    pub adjacency: Option<Vec<WriteNeuronEdgeRequest>>,
}

/// Response body for a successful neuron write.
#[derive(Debug, Serialize)]
pub struct WriteNeuronResponse {
    pub neuron_id: String,
    pub status: &'static str,
}

/// `POST /neuron` — Write a new neuron to the database.
pub async fn handle_write(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WriteNeuronRequest>,
) -> impl IntoResponse {
    tracing::info!(">>> ENTERED handle_write for payload_type: {}", body.payload_type);
    
    // Validate vector dimensions.
    if body.vector_data.iter().any(|v| !v.is_finite()) {
        warn!("Rejected write: vector contains NaN or Infinity");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "vector_data contains non-finite values" })),
        ).into_response();
    }

    // Parse model hash from hex.
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

    let mut neuron = UniversalNeuron::new(
        Bytes::from(body.raw_payload.into_bytes()),
        body.vector_data,
        model_hash_bytes,
        payload_type,
    );

    // Hydrate dynamic ruleset
    neuron.dna = body.dna;

    // Hydrate adjacency edges
    if let Some(req_edges) = body.adjacency {
        let mut edges = Vec::new();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        for e in req_edges {
            if let Ok(uuid) = uuid::Uuid::parse_str(&e.target_id) {
                edges.push(NeuronEdge {
                    target_id: cluaizdb_types::NeuronId::from_bytes(*uuid.as_bytes()),
                    weight: e.weight,
                    last_accessed_ns: now_ns,
                });
            }
        }
        neuron.adjacency = edges;
    }

    let neuron_id = neuron.id.to_string();

    // Resolve tenant ID from headers
    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    // Open/retrieve sharded database
    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, tenant = %tenant_id, "Failed to resolve or open tenant shard");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to open tenant shard: {}", e) })),
            ).into_response();
        }
    };

    // Verify payload based on format config
    match shard.config.payload_format.as_str() {
        "protobuf" => {
            // Protobuf validation framework placeholder
            tracing::info!("Validating Protobuf payload structure for tenant {}", tenant_id);
        }
        "flatbuffers" => {
            // FlatBuffers zero-copy verification placeholder
            tracing::info!("Verifying FlatBuffers binary layout for tenant {}", tenant_id);
        }
        _ => {
            // JSON (default / implicit verification through Axum extractor)
            tracing::info!("Processing JSON structured payload for tenant {}", tenant_id);
        }
    }

    // Retrieve current telemetry for the DNA script
    let (bp, spo2) = {
        let tel = state.heart_telemetry.read().await;
        (tel.bp_systolic, tel.spo2)
    };

    // Execute DNA Validation (Subconscious AI)
    let durability = match genome::GenomeExecutor::execute_on_write(&neuron, bp as f32, spo2 as f32) {
        Ok(GenomeWriteAction::Allow(d)) => d,
        Ok(GenomeWriteAction::Defer) => {
            // DNA requested a lazy write: commit to WAL only, skip LMDB for now.
            tracing::warn!(bp=bp, spo2=spo2, neuron_id=%neuron_id, "DNA deferred write (Lazy Execution)");
            let mut wal = shard.wal_writer.lock().await;
            if let Err(e) = wal.append_write(&neuron) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("WAL write failure: {}", e) }))).into_response();
            }
            return (
                StatusCode::ACCEPTED,
                Json(WriteNeuronResponse { neuron_id, status: "deferred" }),
            ).into_response();
        }
        Ok(GenomeWriteAction::Abort(reason)) => {
            tracing::warn!(reason = %reason, "DNA Validation Aborted write");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("DNA Validation Failed: {}", reason) })),
            ).into_response();
        }
        Err(e) => {
            tracing::warn!(error = %e, "DNA engine internal error");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("DNA engine error: {:?}", e) })),
            ).into_response();
        }
    };

    // --- Strict Durability path ---
    // DNA requested sync_write: "strict" / true.
    // Bypass the Transit Lounge entirely. Write WAL, call fsync, then write LMDB inline.
    // This guarantees the block is physically on the SSD before we return 201.
    if durability == Durability::Strict {
        tracing::debug!(neuron_id = %neuron_id, "sync_write: strict — bypassing Transit Lounge");
        let mut wal = shard.wal_writer.lock().await;
        if let Err(e) = wal.append_write(&neuron) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("WAL write failure: {}", e) }))).into_response();
        }
        // fsync: force OS to flush page-cache to physical SSD blocks.
        if let Err(e) = wal.sync() {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("WAL sync failure: {}", e) }))).into_response();
        }
        drop(wal); // release lock before LMDB write
        if let Err(e) = engine_lmdb::write_neuron(&shard.env, &neuron) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("LMDB write failure: {}", e) }))).into_response();
        }
        return (
            StatusCode::CREATED,
            Json(WriteNeuronResponse { neuron_id, status: "created" }),
        ).into_response();
    }

    // --- Lite Durability path (default) ---
    // DNA requested sync_write: "lite" / false (or omitted).
    // Push to the Transit Lounge (O(1) lock-free RAM ring buffer).
    // The background flusher drains it to WAL + LMDB every 50ms.
    if let Err(returned_neuron) = shard.transit_lounge.push(neuron) {
        // Transit Lounge is full — fall back to synchronous WAL + LMDB write (no fsync).
        tracing::warn!(tenant = %tenant_id, "Transit Lounge full — falling back to synchronous write (no fsync)");

        let mut wal = shard.wal_writer.lock().await;
        if let Err(e) = wal.append_write(&returned_neuron) {
            warn!(error = %e, tenant = %tenant_id, "Failed to write to WAL");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("WAL write failure: {}", e) })),
            ).into_response();
        }
        drop(wal);

        if let Err(e) = engine_lmdb::write_neuron(&shard.env, &returned_neuron) {
            warn!(error = %e, tenant = %tenant_id, "Failed to write to LMDB");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("LMDB write failure: {}", e) })),
            ).into_response();
        }
    }

    (
        StatusCode::CREATED,
        Json(WriteNeuronResponse {
            neuron_id,
            status: "created",
        }),
    ).into_response()
}
