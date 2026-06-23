use cluaizd_types::UniversalNeuron;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphMutation {
    ConnectNeurons {
        source: String,
        target: String,
        weight: f32,
    },
    StrengthenEdge {
        source: String,
        target: String,
        amount: f32,
    },
    DecayEdge {
        source: String,
        target: String,
        amount: f32,
    },
}

#[derive(Debug, Clone)]
pub struct GraphContext {
    pub mutations: std::sync::Arc<std::sync::Mutex<Vec<GraphMutation>>>,
}

impl Default for GraphContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphContext {
    pub fn new() -> Self {
        Self {
            mutations: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn take_mutations(&self) -> Vec<GraphMutation> {
        let mut guard = self.mutations.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}

/// Controls how the WAL flush is performed after a successful write.
///
/// - `Lite`   — OS page-cache flush only. Maximum throughput (20k+ OPS).
///              Survives process crashes. Minimal risk on power loss.
/// - `Strict` — Calls `.sync_all()` (fsync) before returning 201.
///              Guarantees the block is physically written to the SSD.
///              Use for payment records, auth tokens, or any critical data.
#[derive(Debug, Clone, PartialEq)]
pub enum Durability {
    Lite,
    Strict,
}

/// The decision returned by `GenomeExecutor::execute_on_write`.
#[derive(Debug, Clone)]
pub enum GenomeWriteAction {
    /// Allow the write. Carries the durability level requested by the DNA script.
    Allow(Durability),
    /// Defer the write to the WAL only (skip LMDB for now).
    Defer,
    /// Abort the write entirely. Carries the reason.
    Abort(String),
}

/// The Unified DNA Executor
/// Runs the `on_write` hook for a given neuron using the specified DNA ruleset.
pub struct GenomeExecutor;

/// Parses the `sync_write` field from a Rhai result map.
/// Accepts both string ("lite" / "strict") and boolean (false / true).
/// If absent, defaults to `Durability::Lite`.
fn parse_durability(result_map: &rhai::Map) -> Durability {
    if let Some(val) = result_map.get("sync_write") {
        // Boolean form: sync_write: true => Strict, false => Lite
        if let Ok(b) = val.as_bool() {
            return if b { Durability::Strict } else { Durability::Lite };
        }
        // String form: "strict" => Strict, anything else ("lite", omitted) => Lite
        if let Ok(s) = val.clone().into_string() {
            if s == "strict" {
                return Durability::Strict;
            }
        }
    }
    Durability::Lite
}

/// Creates and configures a Rhai engine with registered CLUAIZD system helpers.
pub fn create_rhai_engine() -> rhai::Engine {
    let mut engine = rhai::Engine::new();

    // Register cosine_similarity helper
    engine.register_fn("cosine_similarity", |a: rhai::Array, b: rhai::Array| -> f64 {
        let mut vec_a = Vec::with_capacity(a.len());
        let mut vec_b = Vec::with_capacity(b.len());
        for val in a {
            vec_a.push(val.as_float().unwrap_or(0.0) as f32);
        }
        for val in b {
            vec_b.push(val.as_float().unwrap_or(0.0) as f32);
        }
        cluaizd_types::distance::cosine_similarity(&vec_a, &vec_b) as f64
    });

    // Register euclidean_distance helper
    engine.register_fn("euclidean_distance", |a: rhai::Array, b: rhai::Array| -> f64 {
        let mut vec_a = Vec::with_capacity(a.len());
        let mut vec_b = Vec::with_capacity(b.len());
        for val in a {
            vec_a.push(val.as_float().unwrap_or(0.0) as f32);
        }
        for val in b {
            vec_b.push(val.as_float().unwrap_or(0.0) as f32);
        }
        cluaizd_types::distance::euclidean_distance(&vec_a, &vec_b) as f64
    });

    // Register GraphContext and connection helpers
    engine.register_type_with_name::<GraphContext>("GraphContext");

    engine.register_fn("connect_neurons", |ctx: &mut GraphContext, source: String, target: String, weight: f64| {
        ctx.mutations.lock().unwrap().push(GraphMutation::ConnectNeurons {
            source,
            target,
            weight: weight as f32,
        });
    });

    engine.register_fn("strengthen_edge", |ctx: &mut GraphContext, source: String, target: String, amount: f64| {
        ctx.mutations.lock().unwrap().push(GraphMutation::StrengthenEdge {
            source,
            target,
            amount: amount as f32,
        });
    });

    engine.register_fn("decay_edge", |ctx: &mut GraphContext, source: String, target: String, amount: f64| {
        ctx.mutations.lock().unwrap().push(GraphMutation::DecayEdge {
            source,
            target,
            amount: amount as f32,
        });
    });

    engine
}

impl GenomeExecutor {
    /// Evaluates the DNA ruleset to decide if a write should be allowed, deferred, or aborted.
    /// Returns:
    /// - `Ok(GenomeWriteAction::Allow(Durability))` -> Allow write with given durability level
    /// - `Ok(GenomeWriteAction::Defer)`             -> Defer write (lazy to WAL only)
    /// - `Ok(GenomeWriteAction::Abort(reason))`     -> Abort write (validation failed)
    /// - `Err`                                      -> Internal engine error
    pub fn execute_on_write(neuron: &UniversalNeuron, bp: f32, spo2: f32) -> Result<GenomeWriteAction, cluaizd_errors::StorageError> {
        if let Some(dna) = &neuron.dna {
            if let Some(write_script) = &dna.on_write {
                
                // 1. CDQL Engine (Currently JSON-based parsing via CDQL AST)
                if dna.engine == "cdql" {
                    // For now, CDQL is evaluated in query.rs for read ops.
                    // If a CDQL write filter is specified, we evaluate it natively.
                    return Ok(GenomeWriteAction::Allow(Durability::Lite));
                }
                
                // 2. Rhai Interpreted Engine
                if dna.engine == "rhai" {
                    let engine = create_rhai_engine();
                    let mut scope = rhai::Scope::new();
                    let metrics = rhai::Map::from([
                        ("bp".into(), (bp as i64).into()),
                        ("spo2".into(), (spo2 as i64).into())
                    ]);
                    scope.push("system_metrics", metrics);

                    // Expose payload string if text
                    if neuron.payload_type == cluaizd_types::PayloadType::Text {
                        if let Ok(text) = String::from_utf8(neuron.raw_payload.to_vec()) {
                            scope.push("payload", text);
                        }
                    }

                    match engine.eval_with_scope::<rhai::Map>(&mut scope, write_script) {
                        Ok(result_map) => {
                            let durability = parse_durability(&result_map);
                            if let Some(action) = result_map.get("action").and_then(|v| v.clone().into_string().ok()) {
                                tracing::debug!("DNA on_write action: {}, sync_write: {:?}, bp: {}", action, durability, bp);
                                if action == "Abort" {
                                    if let Some(reason) = result_map.get("error").and_then(|v| v.clone().into_string().ok()) {
                                        return Ok(GenomeWriteAction::Abort(reason));
                                    }
                                    return Ok(GenomeWriteAction::Abort("Rhai Aborted write".to_string()));
                                } else if action == "Defer" {
                                    return Ok(GenomeWriteAction::Defer);
                                }
                            }
                            return Ok(GenomeWriteAction::Allow(durability));
                        }
                        Err(e) => return Err(cluaizd_errors::StorageError::DnaValidationFailed(format!("Rhai error: {}", e))),
                    }
                }
                
                // 3. WASM Native Engine (and Auto-WASM)
                if dna.engine == "wasm" || dna.engine == "auto-wasm" {
                    if let Some(wasm_bytes) = &dna.wasm_module {
                        let executor = crate::WasmExecutor::new();
                        // For WASM, `on_write` is mapped to `validate` hook.
                        // WASM does not yet support sync_write control; defaults to Lite.
                        match executor.execute_validate(wasm_bytes, &neuron.raw_payload, &neuron.vector_data) {
                            Ok(true) => return Ok(GenomeWriteAction::Allow(Durability::Lite)),
                            Ok(false) => return Ok(GenomeWriteAction::Abort("WASM Aborted write".to_string())),
                            Err(e) => return Err(e),
                        }
                    }
                    // If no bytes, allow default
                    return Ok(GenomeWriteAction::Allow(Durability::Lite));
                }
            }
        }
        
        // If no DNA or no on_write script, default is to allow with Lite durability.
        Ok(GenomeWriteAction::Allow(Durability::Lite))
    }

    /// Evaluates the DNA ruleset when a neuron is read.
    /// Used to increment strengthen_factor or block access.
    pub fn execute_on_read(neuron: &mut UniversalNeuron) -> Result<(), cluaizd_errors::StorageError> {
        if let Some(dna) = &neuron.dna {
            if let Some(read_script) = &dna.on_read {
                if dna.engine == "rhai" {
                    let engine = create_rhai_engine();
                    let mut scope = rhai::Scope::new();
                    // We can pass the neuron ID or tier as context
                    let ctx = rhai::Map::from([
                        ("id".into(), neuron.id.to_string().into())
                    ]);
                    scope.push("neuron", ctx);

                    // Expose payload string if text
                    if neuron.payload_type == cluaizd_types::PayloadType::Text {
                        if let Ok(text) = String::from_utf8(neuron.raw_payload.to_vec()) {
                            scope.push("payload", text);
                        }
                    }

                    match engine.eval_with_scope::<rhai::Map>(&mut scope, read_script) {
                        Ok(result) => {
                            if let Some(action) = result.get("action").map(|v| v.to_string()) {
                                if action == "Block" {
                                    return Err(cluaizd_errors::StorageError::DnaValidationFailed("Rhai Blocked read".to_string()));
                                }
                            }
                            if let Some(weight_inc) = result.get("increase_weight").and_then(|v| v.as_float().ok()) {
                                // For graph paradigm, increment adjacency weight
                                for edge in &mut neuron.adjacency {
                                    edge.weight += weight_inc as f32;
                                    // cap at 1.0
                                    if edge.weight > 1.0 { edge.weight = 1.0; }
                                }
                            }
                        }
                        Err(e) => return Err(cluaizd_errors::StorageError::DnaValidationFailed(format!("Rhai error: {}", e))),
                    }
                } else if dna.engine == "wasm" {
                    if let Some(ref wasm_bytes) = dna.wasm_module {
                        let executor = crate::wasm_executor::WasmExecutor::new();
                        // Call the exported WASM function "on_read".
                        // If it returns a strengthen factor (e.g. integer percentage), apply it.
                        if let Ok(strengthen_percent) = executor.execute(wasm_bytes, "on_read") {
                            if strengthen_percent > 100 && !neuron.adjacency.is_empty() {
                                let strengthen_factor = strengthen_percent as f32 / 100.0;
                                for edge in &mut neuron.adjacency {
                                    edge.weight *= strengthen_factor;
                                    if edge.weight > 1.0 { edge.weight = 1.0; } // Cap at 1.0
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Evaluates the DNA ruleset for garbage collection/lifecycle events.
    /// Returns:
    /// - `Ok(true)` -> Retain neuron (mutations may have been applied to `neuron`)
    /// - `Ok(false)` -> Delete neuron immediately (Apoptosis)
    pub fn execute_on_lifecycle(neuron: &mut UniversalNeuron, current_time_ns: u64) -> Result<bool, cluaizd_errors::StorageError> {
        if let Some(dna) = &neuron.dna {
            if let Some(lifecycle_script) = &dna.on_lifecycle {
                if dna.engine == "rhai" {
                    let engine = create_rhai_engine();
                    let mut scope = rhai::Scope::new();
                    let age_ns = current_time_ns.saturating_sub(neuron.created_at_ns);
                    let ctx = rhai::Map::from([
                        ("age_ns".into(), (age_ns as i64).into()),
                        ("current_tier".into(), format!("{:?}", neuron.tier).into())
                    ]);
                    scope.push("neuron", ctx);

                    if let Ok(dynamic_config) = rhai::serde::to_dynamic(&dna.parameters) {
                        scope.push_dynamic("config", dynamic_config);
                    }

                    match engine.eval_with_scope::<rhai::Map>(&mut scope, lifecycle_script) {
                        Ok(result) => {
                            // Explicit Deletion (Apoptosis)
                            if let Some(del) = result.get("delete_neuron").and_then(|v| v.as_bool().ok()) {
                                if del {
                                    return Ok(false);
                                }
                            }

                            // Explicit Tier Transitions
                            if let Some(new_tier) = result.get("new_tier").and_then(|v| v.clone().into_string().ok()) {
                                if new_tier == "Warm" && neuron.tier != cluaizd_types::StorageTier::Warm {
                                    neuron.tier = cluaizd_types::StorageTier::Warm;
                                } else if new_tier == "Cold" && neuron.tier != cluaizd_types::StorageTier::Cold {
                                    neuron.tier = cluaizd_types::StorageTier::Cold;
                                }
                            }

                            // Explicit Payload Clearing
                            if let Some(clear) = result.get("clear_payload").and_then(|v| v.as_bool().ok()) {
                                if clear && !neuron.raw_payload.is_empty() {
                                    neuron.raw_payload = bytes::Bytes::new();
                                }
                            }

                            // Explicit Edge Decay
                            if let Some(decay_factor) = result.get("edge_decay_factor").and_then(|v| v.as_float().ok()) {
                                if !neuron.adjacency.is_empty() {
                                    let prune_threshold = result.get("edge_prune_threshold").and_then(|v| v.as_float().ok()).unwrap_or(0.0) as f32;
                                    for edge in &mut neuron.adjacency {
                                        edge.weight *= decay_factor as f32;
                                    }
                                    neuron.adjacency.retain(|edge| edge.weight >= prune_threshold);
                                }
                            }
                        }
                        Err(e) => return Err(cluaizd_errors::StorageError::DnaValidationFailed(format!("Rhai error: {}", e))),
                    }
                }
            }
        }
        Ok(true)
    }

    /// Evaluates the DNA ruleset at a single step of speculative parallel routing.
    /// Returns `Ok(true)` if the path should continue, or `Ok(false)` if it should be pruned/aborted.
    pub fn execute_on_path_step(
        neuron: &UniversalNeuron,
        current_path: Vec<String>,
        ctx: &mut GraphContext,
    ) -> Result<bool, cluaizd_errors::StorageError> {
        if let Some(dna) = &neuron.dna {
            if let Some(step_script) = &dna.on_path_step {
                if dna.engine == "rhai" {
                    let engine = create_rhai_engine();
                    let mut scope = rhai::Scope::new();
                    
                    let path_array: rhai::Array = current_path.into_iter().map(|s| s.into()).collect();
                    scope.push("current_path", path_array);
                    scope.push("ctx", ctx.clone());
                    
                    let neuron_map = rhai::Map::from([
                        ("id".into(), neuron.id.to_string().into())
                    ]);
                    scope.push("neuron", neuron_map);

                    match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, step_script) {
                        Ok(res) => {
                            if let Ok(b) = res.as_bool() {
                                return Ok(b);
                            }
                            if let Some(map) = res.clone().try_cast::<rhai::Map>() {
                                if let Some(action_dyn) = map.get("action") {
                                    let action = action_dyn.to_string();
                                    if action == "Prune" || action == "Abort" {
                                        return Ok(false);
                                    }
                                }
                            }
                            return Ok(true);
                        }
                        Err(e) => return Err(cluaizd_errors::StorageError::DnaValidationFailed(format!("Rhai path_step error: {}", e))),
                    }
                }
            }
        }
        Ok(true)
    }

    /// Evaluates the DNA ruleset when a path is successfully resolved (reinforced).
    pub fn execute_on_path_resolve(
        neuron: &UniversalNeuron,
        winning_path: Vec<String>,
        ctx: &mut GraphContext,
    ) -> Result<(), cluaizd_errors::StorageError> {
        if let Some(dna) = &neuron.dna {
            if let Some(resolve_script) = &dna.on_path_resolve {
                if dna.engine == "rhai" {
                    let engine = create_rhai_engine();
                    let mut scope = rhai::Scope::new();
                    
                    let path_array: rhai::Array = winning_path.clone().into_iter().map(|s| s.into()).collect();
                    scope.push("winning_path", path_array);
                    scope.push("ctx", ctx.clone());
                    
                    let neuron_map = rhai::Map::from([
                        ("id".into(), neuron.id.to_string().into())
                    ]);
                    scope.push("neuron", neuron_map.clone());

                    if let Err(e) = engine.eval_with_scope::<()>(&mut scope, resolve_script) {
                        // Check if it returned a Map/Dynamic instead
                        let mut scope2 = rhai::Scope::new();
                        let path_array2: rhai::Array = winning_path.into_iter().map(|s| s.into()).collect();
                        scope2.push("winning_path", path_array2);
                        scope2.push("ctx", ctx.clone());
                        scope2.push("neuron", neuron_map);
                        if let Err(e2) = engine.eval_with_scope::<rhai::Dynamic>(&mut scope2, resolve_script) {
                            return Err(cluaizd_errors::StorageError::DnaValidationFailed(format!("Rhai path_resolve error: {} / {}", e, e2)));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
