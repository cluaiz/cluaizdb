use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use cluaizdb_substrate::StorageSubstrate;
use serde::{Deserialize, Serialize};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct VectorInsertRequest {
    pub record_id: String,
    pub vector: Vec<f32>,
}

#[derive(Deserialize)]
pub struct VectorSearchRequest {
    pub query: Vec<f32>,
    pub k: Option<usize>,
}

#[derive(Serialize)]
pub struct VectorSearchResultItem {
    pub record_id: String,
    pub distance: f32,
}

pub async fn insert_vector(
    State(state): State<Arc<AppState>>,
    Json(body): Json<VectorInsertRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rec_id: u128 = body
        .record_id
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid record_id: {}", e)))?;

    // 1. Durably persist vector to Substrate LMDB table
    state
        .db
        .put_vector(1, rec_id, &body.vector)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to persist vector: {}", e)))?;

    // 2. Insert into in-memory HNSW index
    state.vector_index.insert(rec_id, body.vector);
    Ok(Json(serde_json::json!({ "status": "indexed", "record_id": body.record_id })))
}

pub async fn search_vector(
    State(state): State<Arc<AppState>>,
    Json(body): Json<VectorSearchRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let k = body.k.unwrap_or(5);
    let results = state.vector_index.search_knn(&body.query, k);

    let items: Vec<VectorSearchResultItem> = results
        .into_iter()
        .map(|(id, dist)| VectorSearchResultItem {
            record_id: id.to_string(),
            distance: dist,
        })
        .collect();

    Ok(Json(items))
}
