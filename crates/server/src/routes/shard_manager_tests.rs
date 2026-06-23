use bytes::Bytes;
use cluaizd_types::{PayloadType, UniversalNeuron, NeuronDna};
use tempfile::tempdir;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::shard_manager::ShardManager;

#[tokio::test]
async fn test_shard_isolation_and_routing() {
    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let telemetry = Arc::new(RwLock::new(heart::Telemetry::default()));
    let manager = ShardManager::new(tmp_dir.path(), telemetry, "dashmap".into(), "json".into());

    // Open tenant_A and tenant_B
    let shard_a = manager.get_or_open_shard("tenant_A").await.expect("Failed to open shard A");
    let shard_b = manager.get_or_open_shard("tenant_B").await.expect("Failed to open shard B");

    // Write a neuron to tenant_A
    let model_hash = [42u8; 32];
    let neuron = UniversalNeuron::new(
        Bytes::from("tenant A specific message"),
        [0.5f32; 16].to_vec(),
        model_hash,
        PayloadType::Text,
    );
    let neuron_id = neuron.id;

    engine_lmdb::write_neuron(&shard_a.env, &neuron).expect("Failed to write to shard A");

    // Retrieve the neuron from tenant_A - should succeed
    let fetched_a = engine_lmdb::read_neuron(&shard_a.env, neuron_id, Some(model_hash))
        .expect("Failed to read from shard A");
    assert_eq!(fetched_a.id, neuron_id);

    // Retrieve from tenant_B - should fail with NeuronNotFound
    let fetched_b = engine_lmdb::read_neuron(&shard_b.env, neuron_id, Some(model_hash));
    assert!(fetched_b.is_err());
}

#[tokio::test]
async fn test_shard_wal_recovery_on_opening() {
    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let tenant_id = "recovery_tenant";

    // 1. Manually write a neuron to the WAL without writing it to LMDB.
    let wal_dir = tmp_dir.path().join(tenant_id).join("wal");
    std::fs::create_dir_all(&wal_dir).expect("Failed to create wal dir");

    let model_hash = [100u8; 32];
    let neuron = UniversalNeuron::new(
        Bytes::from("lost transaction to recover"),
        [0.9f32; 16].to_vec(),
        model_hash,
        PayloadType::Text,
    );
    let neuron_id = neuron.id;

    {
        let mut writer = wal::WalWriter::open(&wal_dir).expect("Failed to open WalWriter");
        writer.append_write(&neuron).expect("Failed to write to WAL");
    }

    // 2. Open shard using ShardManager. It should automatically run recovery.
    let telemetry = Arc::new(RwLock::new(heart::Telemetry::default()));
    let manager = ShardManager::new(tmp_dir.path(), telemetry, "dashmap".into(), "json".into());
    let shard = manager.get_or_open_shard(tenant_id).await.expect("Failed to open shard");

    // 3. Verify the neuron was successfully recovered and populated in LMDB.
    let recovered = engine_lmdb::read_neuron(&shard.env, neuron_id, Some(model_hash))
        .expect("Failed to read recovered neuron from LMDB");
    assert_eq!(recovered.id, neuron_id);
    assert_eq!(recovered.raw_payload, Bytes::from("lost transaction to recover"));
}

#[tokio::test]
async fn test_biological_gc_ttl_decay() {
    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let telemetry = Arc::new(RwLock::new(heart::Telemetry::default()));
    let manager = ShardManager::new(tmp_dir.path(), telemetry, "dashmap".into(), "json".into());
    let tenant_id = "ttl_decay_tenant";

    let shard = manager.get_or_open_shard(tenant_id).await.expect("Failed to open shard");

    // Write a neuron with a 1ms TTL (so it expires immediately)
    let model_hash = [100u8; 32];
    let mut neuron = UniversalNeuron::new(
        Bytes::from("temporary sensory data"),
        [0.8f32; 16].to_vec(),
        model_hash,
        PayloadType::Text,
    );
    neuron.dna = Some(NeuronDna {
        on_write: Some("if payload != \"test\" { throw \"Invalid payload\"; }".to_string()),
        on_read: None,
        on_index: None,
        on_traverse: None,
        on_dream: None,
        on_lifecycle: Some("let res = #{}; if neuron.age_ns > 1000000 { res.new_tier = \"Warm\"; res.clear_payload = true; } res".to_string()),
        on_path_step: None,
        on_path_resolve: None,
        wasm_module: None,
        wasm_module_path: None,
        parameters: serde_json::json!({}),
        engine: "rhai".to_string(),
    });
    neuron.created_at_ns = 0; // force it to be created far in the past so it is expired

    let neuron_id = neuron.id;

    engine_lmdb::write_neuron(&shard.env, &neuron).expect("Failed to write neuron");

    // Run GC Sweep manually
    engine_lmdb::run_gc_sweep(&shard.env, 95).expect("GC Sweep failed");

    // Retrieve the neuron back - it should have StorageTier::Warm and empty raw_payload,
    // but the vector data is preserved (Shadow State).
    let fetched = engine_lmdb::read_neuron(&shard.env, neuron_id, Some(model_hash))
        .expect("Failed to read neuron after GC");

    assert_eq!(fetched.tier, cluaizd_types::StorageTier::Warm);
    assert!(fetched.raw_payload.is_empty());
    assert_eq!(fetched.vector_data, vec![0.8f32; 16]);
}

#[tokio::test]
async fn test_artificial_pacemaker_stream_limit() {
    use axum::body::Bytes;
    use axum::http::HeaderMap;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use crate::routes::state::AppState;
    use crate::routes::stream::handle_stream;
    
    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let telemetry = Arc::new(RwLock::new(heart::Telemetry::default()));
    let shard_manager = ShardManager::new(tmp_dir.path(), Arc::clone(&telemetry), "dashmap".into(), "json".into());
    let sensory_shard = engine_lmdb::SensoryShard::open(&tmp_dir.path().join("sensory"), 10).expect("Failed to open sensory shard");
    
    let genome_registry = genome::GenomeRegistry::new();
    let heart = heart::Heart::new(tmp_dir.path());
    let state = AppState::new(
        shard_manager,
        sensory_shard,
        genome_registry,
        Arc::clone(&heart.telemetry),
        Arc::clone(&heart.booster_state),
    );
    
    // Set pacemaker write limit to 10 bytes
    state.write_rate_limit.store(10, std::sync::atomic::Ordering::SeqCst);
    
    // Ingest 5 bytes -> should succeed (returns 202 Accepted)
    let body_small = Bytes::from("12345");
    let response_small = handle_stream(State(state.clone()), HeaderMap::new(), body_small).await.into_response();
    assert_eq!(response_small.status(), axum::http::StatusCode::ACCEPTED);
    
    // Ingest 15 bytes -> should be blocked by pacemaker (returns 413 Payload Too Large)
    let body_large = Bytes::from("123456789012345");
    let response_large = handle_stream(State(state), HeaderMap::new(), body_large).await.into_response();
    assert_eq!(response_large.status(), axum::http::StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_sandbox_mutation_validation() {
    use axum::http::HeaderMap;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use crate::routes::state::AppState;
    use crate::routes::validate::{handle_validate, ValidateNeuronRequest};

    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let telemetry = Arc::new(RwLock::new(heart::Telemetry::default()));
    let shard_manager = ShardManager::new(tmp_dir.path(), Arc::clone(&telemetry), "dashmap".into(), "json".into());
    let sensory_shard = engine_lmdb::SensoryShard::open(&tmp_dir.path().join("sensory"), 10).expect("Failed to open sensory shard");
    let genome_registry = genome::GenomeRegistry::new();
    let heart = heart::Heart::new(tmp_dir.path());
    let state = AppState::new(
        shard_manager,
        sensory_shard,
        genome_registry,
        Arc::clone(&heart.telemetry),
        Arc::clone(&heart.booster_state),
    );

    let model_hash = "2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a".to_string();
    let original_id = uuid::Uuid::now_v7().to_string();

    // 1. Safe vector -> sum = 0.5 * 16 = 8.0 (< 100) -> should be approved (200 OK)
    let req_safe = axum::Json(ValidateNeuronRequest {
        original_id: original_id.clone(),
        proposed_payload: "valid update".to_string(),
        proposed_vector_data: vec![0.5f32; 16],
        model_creator_hash: model_hash.clone(),
        payload_type: "text".to_string(),
        dna: None,
        adjacency: None,
    });
    let resp_safe = handle_validate(State(state.clone()), HeaderMap::new(), req_safe).await.into_response();
    assert_eq!(resp_safe.status(), axum::http::StatusCode::OK);

    // 2. Unsafe/Explosive vector -> sum = 10.0 * 16 = 160.0 (> 100) -> should be blocked (422 Unprocessable Entity)
    let req_unsafe = axum::Json(ValidateNeuronRequest {
        original_id: original_id.clone(),
        proposed_payload: "tumor mutation".to_string(),
        proposed_vector_data: vec![10.0f32; 16],
        model_creator_hash: model_hash,
        payload_type: "text".to_string(),
        dna: None,
        adjacency: None,
    });
    let resp_unsafe = handle_validate(State(state), HeaderMap::new(), req_unsafe).await.into_response();
    assert_eq!(resp_unsafe.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}
