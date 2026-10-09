pub mod handler;
pub mod types;

pub use handler::McpHandler;
pub use types::{JsonRpcRequest, JsonRpcResponse, McpTextContent, McpTool};

#[cfg(test)]
mod tests {
    use super::*;
    use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv};
    use cluaizdb_group_commit::{GroupCommitter, GroupCommitConfig};
    use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_mcp_initialize_and_tool_list() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
        let index = Arc::new(HnswIndex::<CosineDistance>::new());

        let handler = McpHandler::new(db, committer, index);

        // 1. Initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        let init_resp = handler.handle_request(init_req).await;
        assert!(init_resp.error.is_none());
        assert_eq!(init_resp.result.unwrap()["serverInfo"]["name"], "cluaizdb-mcp-server");

        // 2. Tools List
        let tools_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };
        let tools_resp = handler.handle_request(tools_req).await;
        assert!(tools_resp.error.is_none());
        let tools = tools_resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 4);
    }

    #[tokio::test]
    async fn test_mcp_ingest_and_query_dispatch() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
        let index = Arc::new(HnswIndex::<CosineDistance>::new());

        let handler = McpHandler::new(db.clone(), committer, Arc::clone(&index));

        // 1. Ingest Memory via MCP
        let ingest_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(101)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "db_ingest",
                "arguments": {
                    "collection_id": 1,
                    "record_id": "9999",
                    "payload": "{\"thought\": \"remember user preference\"}"
                }
            })),
        };
        let ingest_resp = handler.handle_request(ingest_req).await;
        assert!(ingest_resp.error.is_none());

        // 2. Index Term in Substrate and Inspect via MCP
        db.index_term(1, b"pref:memory", 9999).unwrap();
        let inspect_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(102)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "db_inspect_node",
                "arguments": {
                    "collection_id": 1,
                    "term": "pref:memory"
                }
            })),
        };
        let inspect_resp = handler.handle_request(inspect_req).await;
        assert!(inspect_resp.error.is_none());
        let content_text = &inspect_resp.result.unwrap()["content"][0]["text"];
        assert!(content_text.as_str().unwrap().contains("9999"));

        // 3. Query Vector via MCP
        index.insert(9999, vec![1.0, 0.0, 0.0]);
        let vector_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(103)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "db_query_vector",
                "arguments": {
                    "query": [0.95, 0.05, 0.0],
                    "k": 1
                }
            })),
        };
        let vector_resp = handler.handle_request(vector_req).await;
        assert!(vector_resp.error.is_none());
        let vec_content = &vector_resp.result.unwrap()["content"][0]["text"];
        assert!(vec_content.as_str().unwrap().contains("9999"));
    }
}
