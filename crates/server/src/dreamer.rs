use std::sync::Arc;
use std::time::Duration;
use tracing::{info, debug, error};
use crate::routes::AppState;


/// Spawns the background Subconscious Dreaming Engine thread.
/// This thread periodically wakes up and runs `on_dream` DNA scripts.
pub fn spawn_dreamer(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("The Dreaming Engine (Subconscious Mode) has started.");
        
        loop {
            // Wake up every 60 seconds (or configured interval)
            tokio::time::sleep(Duration::from_secs(60)).await;
            debug!("Dreaming Engine woke up.");
            
            // 1. Check Autonomic Telemetry & WASM Booster for Backpressure
            let (bp, spo2, process_bp, process_spo2, gpu_load, ssd_load) = {
                let tel = state.heart_telemetry.read().await;
                (tel.bp_systolic, tel.spo2, tel.process_bp, tel.process_spo2, tel.gpu_load, tel.ssd_load)
            };

            let mut throttle_delay: u32 = 0;

            {
                let booster_state = state.booster_state.read().await;
                if let Some(ref wasm_bytes) = booster_state.wasm_bytes {
                    let executor = genome::WasmExecutor::new();
                    match executor.execute_booster(wasm_bytes, bp as u32, spo2 as u32, process_bp as u32, process_spo2 as u32, gpu_load as u32, ssd_load as u32, booster_state.active_mode) {
                        Ok(res) => {
                            throttle_delay = res; // Returns throttle delay in milliseconds (0 means no throttling)
                        }
                        Err(e) => {
                            tracing::error!("Failed to execute Booster WASM: {}. Falling back to default limits.", e);
                            if bp > 90 || spo2 < 10 { throttle_delay = 500; }
                        }
                    }
                } else {
                    // Fallback if no WASM is loaded
                    if bp > 90 || spo2 < 10 { throttle_delay = 500; }
                }
            }

            if throttle_delay > 0 {
                tracing::warn!(bp = bp, spo2 = spo2, process_bp = process_bp, delay = throttle_delay, "System Booster: Resources under stress. Dreaming Engine throttling applied.");
            }

            let shards = state.shard_manager.get_all_shards().await;

            for shard in shards {
                // 1. Fetch all neurons to pick a random starting point
                let neurons = match engine_lmdb::reader::iter_all_neurons(&shard.env) {
                    Ok(n) => n,
                    Err(e) => {
                        error!("Dreaming Engine failed to fetch neurons: {}", e);
                        continue;
                    }
                };
                
                if neurons.is_empty() {
                    continue;
                }
                
                // --- STOCHASTIC RANDOM WALK ---
                // Pick a random starting neuron
                let start_idx = rand::random::<usize>() % neurons.len();
                let mut current_neuron = neurons[start_idx].clone();
                let walk_depth = (rand::random::<usize>() % 5) + 2; // Walk 2 to 6 steps deep

                for step in 0..walk_depth {
                    if throttle_delay > 0 {
                        tokio::time::sleep(Duration::from_millis(throttle_delay as u64)).await;
                    }

                    // If the current neuron has edges, pick one randomly to traverse
                    if !current_neuron.adjacency.is_empty() {
                        let edge_idx = rand::random::<usize>() % current_neuron.adjacency.len();
                        let target_id = current_neuron.adjacency[edge_idx].target_id;

                        // Try to fetch the target neuron
                        if let Ok(next_neuron) = engine_lmdb::read_neuron(&shard.env, target_id, None) {
                            
                            // Check if the starting neuron shares mutual connections with this target
                            // using the nanosecond Specular Graph Bitwise Intersection!
                            if let Ok(true) = engine_lmdb::SpecularGraph::fast_bitwise_intersection(&shard.env, current_neuron.id, target_id) {
                                debug!("Dreamer found deep structural alignment between {} and {} using Specular Graph!", current_neuron.id, target_id);
                            }

                            current_neuron = next_neuron;
                        }
                    }

                    let mut changed = false;
                    
                    if let Some(ref dna) = current_neuron.dna {
                        if let Some(dream_script) = &dna.on_dream {
                            if dna.engine == "rhai" {
                                let engine = rhai::Engine::new();
                                let mut scope = rhai::Scope::new();
                                
                                scope.push("total_neurons", neurons.len() as i64);
                                scope.push("walk_step", step as i64);
                                
                                if let Ok(dynamic_config) = rhai::serde::to_dynamic(&dna.parameters) {
                                    scope.push_dynamic("config", dynamic_config);
                                }

                                if let Ok(result_map) = engine.eval_with_scope::<rhai::Map>(&mut scope, dream_script) {
                                    if let Some(create_edge) = result_map.get("create_edge").and_then(|v| v.as_bool().ok()) {
                                        if create_edge {
                                            let target = result_map.get("target").and_then(|v| v.to_string().parse().ok());
                                            let weight = result_map.get("weight").and_then(|v| v.as_float().ok()).unwrap_or(0.5);
                                            
                                            if let Some(target_id) = target {
                                                current_neuron.adjacency.push(cluaizdb_types::NeuronEdge {
                                                    target_id,
                                                    weight: weight as f32,
                                                    last_accessed_ns: std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_nanos() as u64,
                                                });
                                                changed = true;
                                                debug!("Dreamer forged edge between {} and {}", current_neuron.id, target_id);
                                            }
                                        }
                                    }
                                }
                            } else if dna.engine == "wasm" {
                                if let Some(ref wasm_bytes) = dna.wasm_module {
                                    let executor = genome::WasmExecutor::new();
                                    if let Ok(res) = executor.execute(wasm_bytes, "on_dream") {
                                        if res > 0 {
                                            // TODO: implement WASM-based dream edge generation
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // 3. Save if changed
                    if changed {
                        let _ = engine_lmdb::writer::write_neuron(&shard.env, &current_neuron);
                        
                        // Update JUJU Spatial Map with new edges
                        let mut map = state.spatial_map.write().await;
                        // Add node if not exists with arbitrary default position
                        map.nodes.entry(current_neuron.id.to_string()).or_insert_with(|| crate::routes::juju::SpatialCoordinates {
                            x: rand::random::<f32>() * 1000.0,
                            y: rand::random::<f32>() * 1000.0,
                            z: rand::random::<f32>() * 1000.0,
                            tier: format!("{:?}", current_neuron.tier),
                        });
                        
                        let edges: Vec<String> = current_neuron.adjacency.iter().map(|e| e.target_id.to_string()).collect();
                        map.edges.insert(current_neuron.id.to_string(), edges);
                    }
                }
            }
        }
    });
}
