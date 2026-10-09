use std::sync::Arc;
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use cluaizdb_mcp::JsonRpcRequest;
use crate::state::AppState;

pub async fn handle_mcp_rpc(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let resp = state.mcp.handle_request(req).await;
    Json(resp)
}
