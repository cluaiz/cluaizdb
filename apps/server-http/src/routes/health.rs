use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "engine": "cluaizdb-substrate"
    }))
}
