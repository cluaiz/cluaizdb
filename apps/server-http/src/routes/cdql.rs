use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::state::AppState;
use cluaizdb_cdql::{build_plan, execute_plan, parse, QueryContext};

#[derive(Deserialize)]
pub struct CdqlQueryRequest {
    pub query: String,
    #[serde(default = "default_collection")]
    pub collection_id: u32,
}

fn default_collection() -> u32 {
    1
}

pub async fn handle_cdql_query(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CdqlQueryRequest>,
) -> impl IntoResponse {
    let ast = match parse(&payload.query) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("CDQL Parse Error: {}", e) })),
            );
        }
    };

    let plan = match build_plan(&ast) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("CDQL Plan Error: {}", e) })),
            );
        }
    };

    let ctx = QueryContext::with_vector_index(&state.db, payload.collection_id, &state.vector_index);
    match execute_plan(&plan, &ctx) {
        Ok(result) => (StatusCode::OK, Json(json!(result))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("CDQL Execution Error: {}", e) })),
        ),
    }
}
