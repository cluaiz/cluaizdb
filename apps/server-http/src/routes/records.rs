use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use cluaizdb_substrate::StorageSubstrate;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct PutRecordRequest {
    pub collection_id: Option<u32>,
    pub record_id: String,
    pub payload: String,
}

#[derive(Serialize)]
pub struct PutRecordResponse {
    pub status: &'static str,
    pub collection_id: u32,
    pub record_id: String,
}

/// Durable write backed by Group Commit (~121 µs physical SSD barrier).
pub async fn put_record(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PutRecordRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let col_id = body.collection_id.unwrap_or(1);
    let record_id: u128 = body
        .record_id
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid record_id: {}", e)))?;

    state
        .committer
        .write(col_id, record_id, body.payload.into_bytes())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PutRecordResponse {
        status: "committed",
        collection_id: col_id,
        record_id: body.record_id,
    }))
}

/// Physical in-process record point lookup (< 5 µs recall).
pub async fn get_record(
    State(state): State<Arc<AppState>>,
    Path((col_id, rec_id_str)): Path<(u32, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let record_id: u128 = rec_id_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid record_id: {}", e)))?;

    match state.db.get_record(col_id, record_id) {
        Ok(Some(bytes)) => {
            let payload = String::from_utf8(bytes).unwrap_or_else(|_| "<binary>".to_string());
            Ok(Json(json!({
                "collection_id": col_id,
                "record_id": rec_id_str,
                "payload": payload
            })))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Record not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Physical record deletion.
pub async fn delete_record(
    State(state): State<Arc<AppState>>,
    Path((col_id, rec_id_str)): Path<(u32, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let record_id: u128 = rec_id_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid record_id: {}", e)))?;

    match state.db.delete_record(col_id, record_id) {
        Ok(true) => Ok(Json(json!({ "status": "deleted", "record_id": rec_id_str }))),
        Ok(false) => Err((StatusCode::NOT_FOUND, "Record not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
