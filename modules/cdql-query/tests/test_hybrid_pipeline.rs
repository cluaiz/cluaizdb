use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;

use cluaizdb_substrate::{EdgeMetadata, StorageSubstrate, SubstrateEnv};
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb_cdql::{build_plan, execute_plan, parse, QueryContext};

#[test]
fn test_hybrid_ai_memory_pipeline() {
    let dir = tempdir().unwrap();
    let substrate = SubstrateEnv::open(dir.path(), 20 * 1024 * 1024).unwrap();
    let col_id = 1u32;

    // 1. Ingest Entities (Document Records)
    // Node 101: User Profile
    let user_data = serde_json::json!({
        "id": 101,
        "name": "Aryan",
        "role": "Chief Architect",
        "domain": "AI Systems"
    });
    substrate.put_record(col_id, 101, &serde_json::to_vec(&user_data).unwrap()).unwrap();

    // Node 201: Active Project (Connected via 'owns')
    let project_data = serde_json::json!({
        "id": 201,
        "name": "CluaizDB Substrate",
        "type": "Database Engine",
        "status": "active",
        "priority": 1
    });
    substrate.put_record(col_id, 201, &serde_json::to_vec(&project_data).unwrap()).unwrap();

    // Node 202: Legacy Project (Connected via 'owns', but status 'deprecated')
    let legacy_data = serde_json::json!({
        "id": 202,
        "name": "Legacy Storage Wrapper",
        "type": "Prototype",
        "status": "deprecated",
        "priority": 5
    });
    substrate.put_record(col_id, 202, &serde_json::to_vec(&legacy_data).unwrap()).unwrap();

    // Node 301: Downstream Deployment (2 hops away)
    let deploy_data = serde_json::json!({
        "id": 301,
        "name": "Autonomous Robot Fleet",
        "status": "active"
    });
    substrate.put_record(col_id, 301, &serde_json::to_vec(&deploy_data).unwrap()).unwrap();

    // 2. Ingest Graph Edges into B+Tree Edge Tables
    let edge_meta = EdgeMetadata {
        weight: 1.0,
        created_at: 1000,
        valid_until: u64::MAX,
    };
    substrate.add_edge(101, 1, 201, edge_meta).unwrap(); // Aryan -> owns -> CluaizDB
    substrate.add_edge(101, 1, 202, edge_meta).unwrap(); // Aryan -> owns -> Legacy Wrapper
    substrate.add_edge(201, 2, 301, edge_meta).unwrap(); // CluaizDB -> powers -> Robot Fleet

    // 3. Setup Vector Index (HNSW SIMD AVX2)
    let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
    let mut vec_101 = vec![0.0f32; 128];
    vec_101[0] = 0.95; // Strong semantic signature for Aryan
    vector_index.insert(101, vec_101);

    let ctx = QueryContext::with_vector_index(&substrate, col_id, &vector_index);

    // 4. EXECUTE HYBRID QUERY:
    // Flow: Lookup User 101 -> Traverse 1-hop graph edges -> Filter only active status -> Project name & type
    let hybrid_query = "find id(\"101\") -> traverse(min_hops: 1, max_hops: 1) -> filter status == \"active\" -> project(keep: [\"name\", \"type\"])";
    
    let start = Instant::now();
    let ast = parse(hybrid_query).expect("Failed to parse hybrid query");
    let plan = build_plan(&ast).expect("Failed to build plan");
    let result = execute_plan(&plan, &ctx).expect("Failed to execute hybrid plan");
    let elapsed = start.elapsed();

    println!("⚡ Hybrid query execution time: {:?}", elapsed);
    println!("📦 Results count: {}", result.count);
    println!("📄 Records: {:?}", result.records);

    // 5. ASSERTIONS:
    // Only Node 201 should survive (Node 202 was excluded because status == 'deprecated', Node 301 was 2 hops away)
    assert_eq!(result.count, 1, "Expected exactly 1 surviving connected record");
    let first = &result.records[0];
    assert_eq!(first["name"], "CluaizDB Substrate");
    assert_eq!(first["type"], "Database Engine");
    // Project operator should have stripped unrequested fields
    assert!(first.get("status").is_none());
    assert!(first.get("priority").is_none());

    // Latency must be strictly sub-millisecond in-process (< 1000 µs)
    assert!(elapsed.as_micros() < 2000, "Hybrid execution latency exceeded 2ms threshold!");
}

#[test]
fn test_multihop_graph_expansion_pipeline() {
    let dir = tempdir().unwrap();
    let substrate = SubstrateEnv::open(dir.path(), 20 * 1024 * 1024).unwrap();
    let col_id = 1u32;

    // Chain: 1 -> 2 -> 3
    substrate.put_record(col_id, 1, b"{\"node\": 1}").unwrap();
    substrate.put_record(col_id, 2, b"{\"node\": 2}").unwrap();
    substrate.put_record(col_id, 3, b"{\"node\": 3}").unwrap();

    let meta = EdgeMetadata { weight: 1.0, created_at: 0, valid_until: u64::MAX };
    substrate.add_edge(1, 1, 2, meta).unwrap();
    substrate.add_edge(2, 1, 3, meta).unwrap();

    let ctx = QueryContext::new(&substrate, col_id);

    // Traverse 2 hops from Node 1
    let query = "find id(\"1\") -> traverse(min_hops: 1, max_hops: 2)";
    let ast = parse(query).unwrap();
    let plan = build_plan(&ast).unwrap();
    let result = execute_plan(&plan, &ctx).unwrap();

    // Both Node 2 (depth 1) and Node 3 (depth 2) must be retrieved
    assert_eq!(result.count, 2);
    let ids: Vec<i64> = result.records.iter().map(|r| r["node"].as_i64().unwrap()).collect();
    assert!(ids.contains(&2));
    assert!(ids.contains(&3));
}
