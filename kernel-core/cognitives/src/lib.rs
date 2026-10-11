//! CluaizDB Cognitive Governance Subsystem.
//!
//! Provides template-based Genomes, composable DNA modules, taxonomic hypergeometric topology,
//! action-bounded inference interceptors, and asymmetric double-queue write pipelines.

pub mod async_queue;
pub mod error;
pub mod genome;
pub mod interceptor;
pub mod signal;
pub mod topology;

pub use async_queue::{AsymmetricWriteQueue, QueueCommand};
pub use error::{CognitiveError, Result};
pub use genome::{AgentGenome, DnaModule, GenomeId, PolicyDna, PreferenceDna, WarmWasmDna};
pub use interceptor::{ActionInterceptor, CompactContextPacket, ContextSerializer};
pub use signal::{PostEvaluationResult, Signal};
pub use topology::{DimensionCoordinates, HypergeometricManager};

#[cfg(test)]
mod tests {
    use super::*;
    use cluaizdb_substrate::traits::StorageSubstrate;
    use cluaizdb_substrate::SubstrateEnv;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_context_serializer_strict_bounds() {
        let facts = vec![
            "Brand:BMW",
            "Color:Red",
            "Model:M3",
            "UserPref:Performance",
            "MaxRefund:10000",
        ];
        let packet = ContextSerializer::compress_into_active_compact_state(&facts);
        assert!(packet.tokens_serialized.starts_with("[SYSTEM MEMORY:"));
        assert!(packet.tokens_serialized.ends_with(']'));
        assert!(packet.tokens_serialized.len() <= 190);

        // Extreme length scenario
        let many_facts = vec!["A_VERY_LONG_STRING_THAT_REPRESENTS_EXTENSIVE_DETAILS_OF_A_USER_PREFERENCE_THAT_SHOULD_BE_TRUNCATED"; 5];
        let overflow_packet = ContextSerializer::compress_into_active_compact_state(&many_facts);
        assert!(overflow_packet.tokens_serialized.len() <= 190);
        assert!(overflow_packet.tokens_serialized.ends_with("...]"));
    }

    #[test]
    fn test_genome_and_dna_preflight_gating() {
        let mut genome = AgentGenome::new(101);
        genome.plug_dna(Box::new(PolicyDna::new(vec!["unauthorized_bypass".into()])));
        genome.plug_dna(Box::new(
            PreferenceDna::new()
                .with_association("car", "UserPref:BMW-Red")
                .with_association("payment", "Policy:CheckManagerApproval"),
        ));

        // 1. Harmless intent -> Memory association
        let signal_car = Signal::new("User wants to generate car image");
        let facts = genome.execute_evaluations(&signal_car).unwrap();
        assert_eq!(facts, vec!["UserPref:BMW-Red"]);

        // 2. Unsafe intent -> Policy rejection
        let signal_bad = Signal::new("Attempting unauthorized_bypass of storage invariants");
        let result = genome.execute_evaluations(&signal_bad);
        assert!(matches!(result, Err(CognitiveError::PolicyViolation(_))));

        // 3. Write gate check
        let bad_payload = b"{\"sensitive_identity_token\": \"secret_xyz_123\"}";
        let write_result = genome.dispatch_preflight_write(b"key1", bad_payload);
        assert!(matches!(write_result, Err(CognitiveError::PolicyViolation(_))));
    }

    #[test]
    fn test_action_interceptor_end_to_end() {
        let dir = tempdir().unwrap();
        let db = Arc::new(SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap());

        let mut genome = AgentGenome::new(202);
        genome.plug_dna(Box::new(PolicyDna::new(vec!["drop_tables".into()])));
        genome.plug_dna(Box::new(
            PreferenceDna::new().with_association("vehicle", "VehiclePref:BMW-M3-Red"),
        ));

        let interceptor = ActionInterceptor::new(Arc::clone(&db), Arc::new(genome));

        // Test Permit
        let signal_ok = Signal::new("Calculate simple arithmetic sum");
        assert_eq!(
            interceptor.evaluate_and_route_intent(&signal_ok),
            PostEvaluationResult::Permit
        );

        // Test Intervene (< 50 tokens compact memory injection)
        let signal_vehicle = Signal::new("Render my favorite vehicle");
        match interceptor.evaluate_and_route_intent(&signal_vehicle) {
            PostEvaluationResult::Intervene { context } => {
                assert!(context.contains("VehiclePref:BMW-M3-Red"));
                assert!(context.len() <= 190);
            }
            other => panic!("Expected Intervene, got {:?}", other),
        }

        // Test Reject
        let signal_blocked = Signal::new("Execute drop_tables on system database");
        match interceptor.evaluate_and_route_intent(&signal_blocked) {
            PostEvaluationResult::Reject { reason } => {
                assert!(reason.contains("drop_tables"));
            }
            other => panic!("Expected Reject, got {:?}", other),
        }
    }

    #[test]
    fn test_topology_taxonomic_hierarchy_and_discrete_keys() {
        let dir = tempdir().unwrap();
        let db = Arc::new(SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap());

        let relation_taxonomic = 1u32;
        let topology = HypergeometricManager::new(Arc::clone(&db), relation_taxonomic);

        let parent_id = 1000u128; // Vehicle
        let child_bmw = 1001u128; // Brand: BMW
        let child_audi = 1002u128; // Brand: Audi

        // Insert coordinates
        let coords_parent = DimensionCoordinates::new(1.0, 0.5);
        let coords_child = DimensionCoordinates::new(2.0, 0.75);

        topology.insert_topology_node(1, parent_id, coords_parent).unwrap();
        topology.insert_topology_node(1, child_bmw, coords_child).unwrap();
        topology.insert_topology_node(1, child_audi, coords_child).unwrap();

        // Link hierarchical edges
        topology.link_taxonomic_hierarchy(parent_id, child_bmw, 1.0).unwrap();
        topology.link_taxonomic_hierarchy(parent_id, child_audi, 0.9).unwrap();

        // Query taxonomic cluster
        let cluster = topology.lookup_adjacent_taxonomic_cluster(parent_id).unwrap();
        assert_eq!(cluster.len(), 2);
        assert!(cluster.contains(&child_bmw));
        assert!(cluster.contains(&child_audi));
    }

    #[test]
    fn test_asymmetric_write_queue_persistence() {
        let dir = tempdir().unwrap();
        let db = Arc::new(SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap());

        let queue = AsymmetricWriteQueue::new(Arc::clone(&db));

        let col_id = 5u32;
        let rec_id = 9999u128;
        let payload = b"{\"event\": \"async_learning_experience_recorded\"}";

        queue
            .dispatch(QueueCommand::PersistPayload {
                col: col_id,
                id: rec_id,
                data: payload.to_vec(),
            })
            .unwrap();

        // Shut down the queue to guarantee all pending queue commands are drained and committed
        queue.shutdown();

        let read_back = db.get_record(col_id, rec_id).unwrap();
        assert_eq!(read_back, Some(payload.to_vec()));
    }
}
