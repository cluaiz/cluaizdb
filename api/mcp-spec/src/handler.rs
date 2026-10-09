use std::sync::Arc;
use serde_json::{json, Value};
use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv};
use cluaizdb_group_commit::GroupCommitter;
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};

use crate::types::{JsonRpcRequest, JsonRpcResponse, McpTextContent, McpTool};

/// Central dispatcher for MCP commands originating from Claude or LLM agent hosts.
pub struct McpHandler {
    substrate: SubstrateEnv,
    committer: GroupCommitter,
    vector_index: Arc<HnswIndex<CosineDistance>>,
}

impl McpHandler {
    pub fn new(
        substrate: SubstrateEnv,
        committer: GroupCommitter,
        vector_index: Arc<HnswIndex<CosineDistance>>,
    ) -> Self {
        Self {
            substrate,
            committer,
            vector_index,
        }
    }

    /// Dispatches an incoming JSON-RPC request and returns the corresponding response frame.
    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let req_id = req.id.clone();

        match req.method.as_str() {
            "initialize" => {
                let init_result = json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "cluaizdb-mcp-server",
                        "version": "0.2.0"
                    },
                    "capabilities": {
                        "tools": {}
                    }
                });
                JsonRpcResponse::success(req_id, init_result)
            }

            "tools/list" => {
                let tools = self.list_tools();
                JsonRpcResponse::success(req_id, json!({ "tools": tools }))
            }

            "tools/call" => {
                match self.handle_tool_call(req.params).await {
                    Ok(content) => JsonRpcResponse::success(req_id, json!({ "content": content })),
                    Err(err_msg) => JsonRpcResponse::error(req_id, -32000, err_msg),
                }
            }

            _ => JsonRpcResponse::error(
                req_id,
                -32601,
                format!("Method not found: {}", req.method),
            ),
        }
    }

    /// Returns definitions of all registered tools.
    pub fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "db_query_vector".to_string(),
                description: "Sub-millisecond approximate nearest neighbor (ANN) vector search over dense embeddings.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "array", "items": { "type": "number" }, "description": "Float query vector" },
                        "k": { "type": "integer", "default": 5, "description": "Number of top candidates" }
                    },
                    "required": ["query"]
                }),
            },
            McpTool {
                name: "db_inspect_node".to_string(),
                description: "Composite B+Tree prefix index seek (< 5 µs recall).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "collection_id": { "type": "integer", "default": 1 },
                        "term": { "type": "string", "description": "Indexed search term e.g. status:active or user:alice" }
                    },
                    "required": ["term"]
                }),
            },
            McpTool {
                name: "db_ingest".to_string(),
                description: "Atomically and durably ingest a record backed by Group Commit (~121 µs).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "collection_id": { "type": "integer", "default": 1 },
                        "record_id": { "type": "string", "description": "128-bit decimal or UUID string" },
                        "payload": { "type": "string", "description": "JSON or text payload" }
                    },
                    "required": ["record_id", "payload"]
                }),
            },
            McpTool {
                name: "db_graph_trace".to_string(),
                description: "Traverse semantic graph relationships in O(log N) microsecond time.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "node_id": { "type": "string", "description": "Record ID of start node" },
                        "direction": { "type": "string", "enum": ["outbound", "inbound"], "default": "outbound" },
                        "type_id": { "type": "integer", "description": "Optional edge type filter" }
                    },
                    "required": ["node_id"]
                }),
            },
        ]
    }

    /// Dispatches tool invocations.
    async fn handle_tool_call(&self, params: Option<Value>) -> Result<Vec<McpTextContent>, String> {
        let params = params.ok_or_else(|| "Missing parameters in tools/call".to_string())?;
        let tool_name = params.get("name").and_then(|v| v.as_str()).ok_or("Missing tool name")?;
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool_name {
            "db_query_vector" | "query_vector" | "cluaiz_query_vector" => {
                let query_arr = args.get("query").and_then(|v| v.as_array()).ok_or("Missing query array")?;
                let k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                let float_vec: Vec<f32> = query_arr
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                let results = self.vector_index.search_knn(&float_vec, k);
                let json_output = serde_json::to_string(&results).map_err(|e| e.to_string())?;
                Ok(vec![McpTextContent::new(json_output)])
            }

            "db_inspect_node" | "inspect_node" | "cluaiz_inspect_nodes" => {
                let col_id = args.get("collection_id").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let term = args.get("term").and_then(|v| v.as_str()).ok_or("Missing term string")?;

                let matching_ids = self
                    .substrate
                    .seek_index(col_id, term.as_bytes())
                    .map_err(|e| e.to_string())?;

                let json_output = serde_json::to_string(&matching_ids).map_err(|e| e.to_string())?;
                Ok(vec![McpTextContent::new(json_output)])
            }

            "db_ingest" | "ingest" | "cluaiz_ingest_memory" => {
                let col_id = args.get("collection_id").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let rec_id_str = args.get("record_id").and_then(|v| v.as_str()).ok_or("Missing record_id")?;
                let payload_str = args.get("payload").and_then(|v| v.as_str()).ok_or("Missing payload")?;

                let rec_id: u128 = rec_id_str.parse().map_err(|e| format!("Invalid record_id: {}", e))?;

                self.committer
                    .write(col_id, rec_id, payload_str.as_bytes().to_vec())
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![McpTextContent::new(json!({ "status": "committed", "record_id": rec_id_str }).to_string())])
            }

            "db_graph_trace" | "graph_trace" | "cluaiz_graph_trace" => {
                let node_id_str = args.get("node_id").and_then(|v| v.as_str()).ok_or("Missing node_id")?;
                let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("outbound");
                let type_id = args.get("type_id").and_then(|v| v.as_u64()).map(|t| t as u32);

                let node_id: u128 = node_id_str.parse().map_err(|e| format!("Invalid node_id: {}", e))?;

                if direction == "inbound" {
                    let sources = self.substrate.get_inbound_edges(node_id, type_id).map_err(|e| e.to_string())?;
                    let json_output = serde_json::to_string(&sources).map_err(|e| e.to_string())?;
                    Ok(vec![McpTextContent::new(json_output)])
                } else {
                    let edges = self.substrate.get_outbound_edges(node_id, type_id).map_err(|e| e.to_string())?;
                    let targets: Vec<u128> = edges.into_iter().map(|(dst, _)| dst).collect();
                    let json_output = serde_json::to_string(&targets).map_err(|e| e.to_string())?;
                    Ok(vec![McpTextContent::new(json_output)])
                }
            }

            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }
}
