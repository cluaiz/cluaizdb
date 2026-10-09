pub mod routes;
pub mod state;

pub use routes::create_router;
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use cluaizdb_substrate::SubstrateEnv;
    use cluaizdb_group_commit::{GroupCommitter, GroupCommitConfig};
    use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
    use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};
    use serde_json::json;
    use tempfile::tempdir;
    use tower::ServiceExt;
    use http_body_util::BodyExt;

    fn setup_test_app() -> axum::Router {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
        let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
        let wasm_runner = Arc::new(WasmGenomeRunner::new(SandboxConfig::default()).unwrap());

        let state = Arc::new(AppState::new(db, committer, vector_index, wasm_runner));
        create_router(state)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = setup_test_app();

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["status"], "ok");
        assert_eq!(body_json["version"], "0.2.0");
    }

    #[tokio::test]
    async fn test_records_put_and_get() {
        let app = setup_test_app();

        // 1. Put record
        let put_payload = json!({
            "collection_id": 1,
            "record_id": "42",
            "payload": "{\"memory\": \"remember this context\"}"
        });

        let put_req = Request::builder()
            .method("POST")
            .uri("/v1/records")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&put_payload).unwrap()))
            .unwrap();

        let put_resp = app.clone().oneshot(put_req).await.unwrap();
        assert_eq!(put_resp.status(), StatusCode::OK);

        // 2. Get record (< 5 µs recall)
        let get_req = Request::builder()
            .uri("/v1/records/1/42")
            .body(Body::empty())
            .unwrap();

        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let body_bytes = get_resp.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body_json["record_id"], "42");
        assert!(body_json["payload"].as_str().unwrap().contains("remember this context"));
    }

    #[tokio::test]
    async fn test_mcp_json_rpc_endpoint() {
        let app = setup_test_app();

        let mcp_req_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&mcp_req_body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(body_json["result"]["tools"].as_array().is_some());
        let tools = body_json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }
}
