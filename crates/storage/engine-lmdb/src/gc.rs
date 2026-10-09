use bytes::Bytes;
use cluaizdb_types::{StorageTier, UniversalNeuron};
use rhai::{Engine, Map, Scope};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::info;
use zstd::stream::encode_all;

use crate::LmdbEnv;

/// Spawns the low-priority biological GC thread (The Dreamer Engine Tier Shifter).
pub fn spawn_biological_gc(env: Arc<LmdbEnv>, telemetry: Arc<RwLock<heart::Telemetry>>) {
    tokio::spawn(async move {
        // Run every 10 seconds to respond quickly to RAM pressure
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            // Read current SpO2 (Available RAM percentage)
            let spo2 = {
                let tel = telemetry.read().await;
                tel.spo2
            };

            let env_clone = env.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = run_gc_sweep(&env_clone, spo2);
            })
            .await;
        }
    });
}

/// Run a single GC sweep cycle to degrade expired or aged neurons dynamically based on node rules and RAM pressure.
pub fn run_gc_sweep(env: &LmdbEnv, spo2: u8) -> anyhow::Result<()> {
    let now_ns =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
            as u64;

    // Collect targets first to avoid write lock contention during iteration.
    let mut targets_to_update = Vec::new(); // Vec<(UniversalNeuron, i64)>
    let mut targets_to_delete = Vec::new();

    {
        let rtxn = env.env.read_txn()?;
        let iter = env.db.iter(&rtxn)?;
        for result in iter {
            let (key, mut neuron) = result?;

            let mut changed = false;
            let mut custom_compress_lvl = 3; // Default fallback

            // Evaluate Neuron-level DNA sequence via Unified GenomeExecutor
            let original_tier = neuron.tier;
            let original_payload_len = neuron.raw_payload.len();
            let original_adj_len = neuron.adjacency.len();

            match genome::GenomeExecutor::execute_on_lifecycle(&mut neuron, now_ns) {
                Ok(retain) => {
                    tracing::info!("GC: neuron {} retain: {}", neuron.id, retain);
                    if !retain {
                        targets_to_delete.push(neuron.id);
                        continue;
                    }
                    if neuron.tier != original_tier
                        || neuron.raw_payload.len() != original_payload_len
                        || neuron.adjacency.len() != original_adj_len
                    {
                        changed = true;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to execute lifecycle for {}: {}", neuron.id, e);
                }
            }

            // Global RAM Pressure Demotions (The Dreamer Autonomic Forgetfulness)
            if spo2 < 20 && neuron.tier == StorageTier::Hot {
                // Warning threshold: Drop payload, enter Shadow State (Warm)
                neuron.tier = StorageTier::Warm;
                neuron.raw_payload = Bytes::new();
                changed = true;
                tracing::debug!("Dreamer Engine: Low RAM (SpO2 {}%). Demoting neuron {} to WARM (payload purged).", spo2, neuron.id);
            } else if spo2 < 5 && neuron.tier == StorageTier::Warm {
                // Critical threshold: ZSTD compress the shell into Cold state
                neuron.tier = StorageTier::Cold;
                changed = true;
                tracing::debug!(
                    "Dreamer Engine: CRITICAL RAM (SpO2 {}%). Demoting neuron {} to COLD.",
                    spo2,
                    neuron.id
                );
            }

            if changed {
                targets_to_update.push((neuron, custom_compress_lvl));
            }
        }
    }

    if !targets_to_update.is_empty() || !targets_to_delete.is_empty() {
        let mut wtxn = env.env.write_txn()?;

        for (neuron, _compress_lvl) in &targets_to_update {
            env.db.put(&mut wtxn, &neuron.id, neuron)?;
        }

        for id in targets_to_delete.iter() {
            env.db.delete(&mut wtxn, id)?;
        }

        wtxn.commit()?;
        tracing::info!(
            "GC Sweep complete: {} updated, {} deleted",
            targets_to_update.len(),
            targets_to_delete.len()
        );
    }

    Ok(())
}
