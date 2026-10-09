use std::sync::Arc;
use cluaizdb_substrate::SubstrateEnv;
use cluaizdb_group_commit::GroupCommitter;
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb_wasm_sandbox::WasmGenomeRunner;
use cluaizdb_mcp::McpHandler;

/// Shared Application State holding all sovereign database subsystems.
#[derive(Clone)]
pub struct AppState {
    pub db: SubstrateEnv,
    pub committer: Arc<GroupCommitter>,
    pub vector_index: Arc<HnswIndex<CosineDistance>>,
    pub wasm_runner: Arc<WasmGenomeRunner>,
    pub mcp: Arc<McpHandler>,
}

impl AppState {
    pub fn new(
        db: SubstrateEnv,
        committer: GroupCommitter,
        vector_index: Arc<HnswIndex<CosineDistance>>,
        wasm_runner: Arc<WasmGenomeRunner>,
    ) -> Self {
        let committer = Arc::new(committer);
        let mcp = Arc::new(McpHandler::new(
            db.clone(),
            (*committer).clone(),
            Arc::clone(&vector_index),
        ));

        Self {
            db,
            committer,
            vector_index,
            wasm_runner,
            mcp,
        }
    }
}
