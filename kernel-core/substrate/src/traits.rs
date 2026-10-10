use crate::error::Result;
use crate::graph::{GraphTraversal, TraversalConfig, TraversalNode};
use crate::schema::{EdgeMetadata, StatsMetadata};

/// The fundamental, zero-opinion storage trait for CluaizDB.
/// Abstracting this interface satisfies the Ironclad DRY principle, allowing
/// hot-swapping or mocking without modifying dependent runtimes.
pub trait StorageSubstrate: Send + Sync {
    // Primary Record I/O
    fn get_record(&self, col_id: u32, record_id: u128) -> Result<Option<Vec<u8>>>;
    fn put_record(&self, col_id: u32, record_id: u128, payload: &[u8]) -> Result<()>;
    fn delete_record(&self, col_id: u32, record_id: u128) -> Result<bool>;

    // Secondary B+Tree Composite Indexing
    fn index_term(&self, col_id: u32, term: &[u8], record_id: u128) -> Result<()>;
    fn remove_index_term(&self, col_id: u32, term: &[u8], record_id: u128) -> Result<()>;
    fn seek_index(&self, col_id: u32, term: &[u8]) -> Result<Vec<u128>>;

    // Bi-directional Graph Relations (O(log N))
    fn add_edge(&self, src_id: u128, type_id: u32, dst_id: u128, meta: EdgeMetadata) -> Result<()>;
    fn delete_edge(&self, src_id: u128, type_id: u32, dst_id: u128) -> Result<bool>;
    fn get_outbound_edges(&self, src_id: u128, type_id: Option<u32>) -> Result<Vec<(u128, EdgeMetadata)>>;
    fn get_inbound_edges(&self, dst_id: u128, type_id: Option<u32>) -> Result<Vec<u128>>;

    // Multi-hop Graph Traversal Algorithms
    fn traverse_graph(&self, start_id: u128, config: &TraversalConfig) -> Result<Vec<TraversalNode>> {
        GraphTraversal::bfs(self, start_id, config)
    }

    fn find_shortest_path(
        &self,
        start_id: u128,
        target_id: u128,
        config: &TraversalConfig,
    ) -> Result<Option<Vec<u128>>> {
        GraphTraversal::shortest_path(self, start_id, target_id, config)
    }

    // Sandboxed WASM Genomes
    fn put_genome(&self, wasm_id: u64, bytecode: &[u8]) -> Result<()>;
    fn get_genome(&self, wasm_id: u64) -> Result<Option<Vec<u8>>>;

    // Telemetry & Batched Stats
    fn update_stats(&self, record_id: u128, stats: StatsMetadata) -> Result<()>;
    fn get_stats(&self, record_id: u128) -> Result<Option<StatsMetadata>>;

    // Physical durability sync barrier
    fn sync(&self) -> Result<()>;
}
