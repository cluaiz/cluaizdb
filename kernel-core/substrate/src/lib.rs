pub mod db;
pub mod error;
pub mod schema;
pub mod traits;

pub use db::SubstrateEnv;
pub use error::{Result, SubstrateError};
pub use schema::{EdgeMetadata, StatsMetadata};
pub use traits::StorageSubstrate;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::time::Instant;

    #[test]
    fn test_substrate_crud_and_latency() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();

        let col_id = 1u32;
        let record_id = 123456789u128;
        let payload = b"{\"memory\": \"user prefers dark mode and Rust\"}";

        // Put record
        db.put_record(col_id, record_id, payload).unwrap();

        // Measure Point Read Latency
        let start = Instant::now();
        let retrieved = db.get_record(col_id, record_id).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(retrieved, Some(payload.to_vec()));
        println!("Verified point read latency: {:?}", elapsed);
        // Page cache read must be extremely fast (< 100 µs on cold test harness, < 5 µs hot)
        assert!(elapsed.as_micros() < 200, "Read latency exceeded threshold");

        // Delete record
        let deleted = db.delete_record(col_id, record_id).unwrap();
        assert!(deleted);
        assert_eq!(db.get_record(col_id, record_id).unwrap(), None);
    }

    #[test]
    fn test_composite_secondary_indexing() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();

        let col_id = 1u32;
        let term = b"status:active";

        // Multiple records sharing the exact same index term
        let id1 = 101u128;
        let id2 = 102u128;
        let id3 = 103u128;

        db.index_term(col_id, term, id1).unwrap();
        db.index_term(col_id, term, id2).unwrap();
        db.index_term(col_id, term, id3).unwrap();

        // Seek index
        let matches = db.seek_index(col_id, term).unwrap();
        assert_eq!(matches.len(), 3);
        assert!(matches.contains(&id1));
        assert!(matches.contains(&id2));
        assert!(matches.contains(&id3));

        // Remove one term
        db.remove_index_term(col_id, term, id2).unwrap();
        let matches_after = db.seek_index(col_id, term).unwrap();
        assert_eq!(matches_after.len(), 2);
        assert!(!matches_after.contains(&id2));
    }

    #[test]
    fn test_bidirectional_graph_edges() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();

        let alice = 1001u128;
        let bob = 1002u128;
        let charlie = 1003u128;
        let type_friend = 1u32;

        let meta = EdgeMetadata {
            weight: 0.95,
            created_at: 1700000000,
            valid_until: 1800000000,
        };

        // Alice -> Bob
        db.add_edge(alice, type_friend, bob, meta).unwrap();
        // Charlie -> Bob
        db.add_edge(charlie, type_friend, bob, meta).unwrap();

        // 1. Forward Traversal: Outbound from Alice
        let out_alice = db.get_outbound_edges(alice, Some(type_friend)).unwrap();
        assert_eq!(out_alice.len(), 1);
        assert_eq!(out_alice[0].0, bob);
        assert_eq!(out_alice[0].1.weight, 0.95);

        // 2. Reverse Traversal: Inbound into Bob (without full table scan!)
        let in_bob = db.get_inbound_edges(bob, Some(type_friend)).unwrap();
        assert_eq!(in_bob.len(), 2);
        assert!(in_bob.contains(&alice));
        assert!(in_bob.contains(&charlie));
    }
}
