pub mod health;
pub mod mcp;
pub mod records;
pub mod vector;

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use crate::state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health
        .route("/health", get(health::health_check))
        // Records
        .route("/v1/records", post(records::put_record))
        .route(
            "/v1/records/:col_id/:rec_id",
            get(records::get_record).delete(records::delete_record),
        )
        // Vector Index
        .route("/v1/vectors/insert", post(vector::insert_vector))
        .route("/v1/vectors/search", post(vector::search_vector))
        // Model Context Protocol (MCP)
        .route("/mcp", post(mcp::handle_mcp_rpc))
        .with_state(state)
}
