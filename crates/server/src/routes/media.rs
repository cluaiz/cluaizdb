use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap, header},
    response::IntoResponse,
};
use axum::body::Body;
use uuid::Uuid;

use cluaizdb_types::NeuronId;
use crate::routes::AppState;

/// `GET /stream/{id}` — Zero-Copy Media Streaming.
/// Supports HTTP `Range` headers for partial content.
/// Configurable chunk size through DNA hooks.
pub async fn handle_media_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
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

    let neuron = match engine_lmdb::read_neuron(&shard.env, neuron_id, None) {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Neuron not found").into_response(),
    };

    let total_len = neuron.raw_payload.len() as u64;
    
    // Check DNA for chunk_size_kb (default to 1MB if not set)
    let mut chunk_size = 1024 * 1024;
    if let Some(dna) = &neuron.dna {
        if let Some(val) = dna.parameters.get("chunk_size_kb") {
            if let Some(kb) = val.as_u64() {
                chunk_size = kb * 1024;
            }
        }
    }

    let range_header = headers.get(header::RANGE).and_then(|h| h.to_str().ok());

    let (start, end) = if let Some(range) = range_header {
        if range.starts_with("bytes=") {
            let parts: Vec<&str> = range["bytes=".len()..].split('-').collect();
            let start: u64 = parts[0].parse().unwrap_or(0);
            let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                parts[1].parse().unwrap_or(total_len - 1)
            } else {
                std::cmp::min(start + chunk_size - 1, total_len - 1)
            };
            (start, std::cmp::min(end, total_len - 1))
        } else {
            (0, total_len - 1)
        }
    } else {
        (0, std::cmp::min(chunk_size - 1, total_len - 1))
    };

    if start > end || start >= total_len {
        return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{}", total_len))],
            "Range not satisfiable",
        ).into_response();
    }

    let chunk_len = end - start + 1;
    let chunk_data = neuron.raw_payload.slice(start as usize..=end as usize);

    let content_range = format!("bytes {}-{}/{}", start, end, total_len);

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    response_headers.insert(header::CONTENT_RANGE, content_range.parse().unwrap());
    response_headers.insert(header::CONTENT_LENGTH, chunk_len.to_string().parse().unwrap());
    response_headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());

    (StatusCode::PARTIAL_CONTENT, response_headers, Body::from(chunk_data)).into_response()
}
