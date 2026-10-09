# 🤖 api/mcp-spec: Native Model Context Protocol (MCP) Server
> **Protocol:** Anthropic Model Context Protocol (MCP) Standard  
> **Interface:** Stdio / HTTP JSON-RPC 2.0  
> **Compatibility:** Claude Desktop, Claude on Vertex AI, Agentic Workflows

---

## 🎯 1. Purpose
`mcp-spec` exposes CluaizDB as a first-class Model Context Protocol (MCP) Server. Instead of pasting large document dumps into prompts, Claude and LLM agents dynamically discover and invoke CluaizDB tools to inspect knowledge nodes, query vectors, and ingest working memories in sub-millisecond execution loops.

---

## 🛠️ 2. Exposed Native Tools

| Tool Name | Aliases | Parameters | Purpose |
| :--- | :--- | :--- | :--- |
| **`db_query_vector`** | `query_vector`, `cluaiz_query_vector` | `query: float[]`, `k: int` | Sub-millisecond semantic ANN vector recall. |
| **`db_inspect_node`** | `inspect_node`, `cluaiz_inspect_nodes` | `collection_id: int`, `term: string` | Composite B+Tree prefix lookup (< 5 µs). |
| **`db_ingest`** | `ingest`, `cluaiz_ingest_memory` | `collection_id: int`, `record_id: string`, `payload: string` | Durable memory storage backed by Group Commit. |
| **`db_graph_trace`** | `graph_trace`, `cluaiz_graph_trace` | `node_id: string`, `direction: "outbound" \| "inbound"` | $O(\log N)$ semantic relation traversal. |

---

## 🏛️ 3. Domain Boundaries
- **CAN:** Validate JSON-RPC 2.0 frames, serialize tool results, bridge into `cluaizdb-substrate`.
- **CANNOT:** Mutate raw LMDB internals directly without going through substrate transactions.
