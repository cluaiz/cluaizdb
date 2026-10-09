use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::{VecDeque, HashSet};
use axum::{Json, extract::{Path, State, Query}, http::{StatusCode, HeaderMap}, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;

use cluaizdb_types::{NeuronId, UniversalNeuron};
use crate::routes::AppState;
use crate::routes::shard_manager::ActiveShard;

#[derive(Deserialize)]
pub struct TraverseParams {
    pub depth: Option<u32>,
}

#[derive(Serialize)]
pub struct Subgraph {
    pub nodes: Vec<UniversalNeuron>,
}

/// `GET /graph/{id}/traverse` — Fetch a node and its network (neighbors) up to Depth N.
pub async fn handle_traverse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Query(params): Query<TraverseParams>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid neuron ID format (expected UUID v7)" })),
            ).into_response();
        }
    };

    let root_id = NeuronId::from_bytes(*uuid.as_bytes());
    let max_depth = params.depth.unwrap_or(2);

    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to open tenant shard: {}", e) })),
            ).into_response();
        }
    };

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut subgraph_nodes = Vec::new();

    queue.push_back((root_id, 0));
    visited.insert(root_id);

    while let Some((curr_id, current_depth)) = queue.pop_front() {
        if current_depth > max_depth {
            continue;
        }

        let curr_neuron = match engine_lmdb::read_neuron(&shard.env, curr_id, None) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Explore neighbors if not at max depth
        if current_depth < max_depth {
            let mut follow_edges = Vec::new();
            
            // Check DNA hook on_traverse
            let mut used_dna_hook = false;
            if let Some(ref dna) = curr_neuron.dna {
                if let Some(traverse_script) = &dna.on_traverse {
                    if dna.engine == "rhai" {
                        let mut engine = genome::create_rhai_engine();
                        let env_clone = shard.env.clone();
                        engine.register_fn("fast_bitwise_intersection", move |a_id_str: String, b_id_str: String| -> bool {
                            if let (Ok(a_uuid), Ok(b_uuid)) = (uuid::Uuid::parse_str(&a_id_str), uuid::Uuid::parse_str(&b_id_str)) {
                                let a_id = cluaizdb_types::NeuronId::from_bytes(*a_uuid.as_bytes());
                                let b_id = cluaizdb_types::NeuronId::from_bytes(*b_uuid.as_bytes());
                                engine_lmdb::SpecularGraph::fast_bitwise_intersection(&env_clone, a_id, b_id).unwrap_or(false)
                            } else {
                                false
                            }
                        });
                        
                        for edge in &curr_neuron.adjacency {
                            let mut scope = rhai::Scope::new();
                            scope.push("edge_weight", edge.weight as f64);
                            if let Ok(dynamic_config) = rhai::serde::to_dynamic(&dna.parameters) {
                                scope.push_dynamic("config", dynamic_config);
                            }
                            
                            used_dna_hook = true;
                            let mut follow = true; // default
                            
                            if let Ok(result_map) = engine.eval_with_scope::<rhai::Map>(&mut scope, traverse_script) {
                                if let Some(f) = result_map.get("follow").and_then(|v| v.as_bool().ok()) {
                                    follow = f;
                                }
                            }
                            
                            if follow {
                                follow_edges.push(edge.target_id);
                            }
                        }
                    } else if dna.engine == "wasm" {
                        if let Some(ref wasm_bytes) = dna.wasm_module {
                            let executor = genome::WasmExecutor::new();
                            for edge in &curr_neuron.adjacency {
                                used_dna_hook = true;
                                let mut follow = true;
                                // we'd pass edge weight or id to wasm here
                                if let Ok(res) = executor.execute(wasm_bytes, "on_traverse") {
                                    follow = res > 0;
                                }
                                if follow {
                                    follow_edges.push(edge.target_id);
                                }
                            }
                        }
                    }
                }
            }

            if !used_dna_hook {
                for edge in &curr_neuron.adjacency {
                    follow_edges.push(edge.target_id);
                }
            }

            for target_id in follow_edges {
                if !visited.contains(&target_id) {
                    visited.insert(target_id);
                    queue.push_back((target_id, current_depth + 1));
                }
            }
        }
        
        subgraph_nodes.push(curr_neuron);
    }

    (StatusCode::OK, Json(Subgraph { nodes: subgraph_nodes })).into_response()
}

#[derive(Deserialize)]
pub struct SpeculativeSearchRequest {
    pub start_node: String,
    pub target_node: String,
    pub max_parallel_paths: Option<usize>,
    pub max_depth: Option<usize>,
}

#[derive(Serialize)]
pub struct SpeculativeSearchResponse {
    pub success: bool,
    pub path: Option<Vec<String>>,
    pub total_cost: Option<f32>,
    pub msg: String,
}

pub async fn handle_speculative_search(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<SpeculativeSearchRequest>,
) -> impl IntoResponse {
    let start_uuid = match Uuid::parse_str(&payload.start_node) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SpeculativeSearchResponse {
                    success: false,
                    path: None,
                    total_cost: None,
                    msg: "invalid start_node ID format".to_string(),
                }),
            ).into_response();
        }
    };

    let target_uuid = match Uuid::parse_str(&payload.target_node) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SpeculativeSearchResponse {
                    success: false,
                    path: None,
                    total_cost: None,
                    msg: "invalid target_node ID format".to_string(),
                }),
            ).into_response();
        }
    };

    let start_id = NeuronId::from_bytes(*start_uuid.as_bytes());
    let target_id = NeuronId::from_bytes(*target_uuid.as_bytes());

    let tenant_id = match headers.get("x-tenant-id") {
        Some(val) => val.to_str().unwrap_or("default_sandbox"),
        None => "default_sandbox",
    };

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SpeculativeSearchResponse {
                    success: false,
                    path: None,
                    total_cost: None,
                    msg: format!("Failed to open tenant shard: {}", e),
                }),
            ).into_response();
        }
    };

    let all_shards = state.shard_manager.get_all_shards().await;
    let max_parallel_paths = payload.max_parallel_paths.unwrap_or(6).clamp(2, 20);
    let max_depth = payload.max_depth.unwrap_or(10);

    let cancelled = Arc::new(AtomicBool::new(false));
    let active_tasks = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(Vec<String>, f32, Vec<genome::GraphMutation>)>(100);
    let graph_ctx = genome::GraphContext::new();

    // Spawn the root search
    let shard_clone = Arc::clone(&shard);
    let all_shards_clone = all_shards.clone();
    let cancelled_clone = Arc::clone(&cancelled);
    let active_tasks_clone = Arc::clone(&active_tasks);
    let tx_clone = tx.clone();
    let graph_ctx_clone = graph_ctx.clone();

    tokio::spawn(async move {
        explore_path(
            start_id,
            target_id,
            shard_clone,
            all_shards_clone,
            Vec::new(),
            Vec::new(),
            0.0,
            cancelled_clone,
            active_tasks_clone,
            max_parallel_paths,
            max_depth,
            tx_clone,
            graph_ctx_clone,
        ).await;
    });

    // Wait for first winning path, or all tasks to finish/timeout
    let search_result: Option<(Vec<String>, f32, Vec<genome::GraphMutation>)> = tokio::select! {
        res = rx.recv() => res,
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            cancelled.store(true, Ordering::SeqCst);
            None
        }
    };

    if let Some((winning_path, total_cost, mutations)) = search_result {
        // Winning path resolved. Apply mutations collected during traversal
        let _ = apply_mutations(&shard, mutations);

        // Run execute_on_path_resolve for each neuron in the winning path
        let mut resolve_ctx = genome::GraphContext::new();
        for neuron_id_str in &winning_path {
            if let Ok(neuron_uuid) = Uuid::parse_str(neuron_id_str) {
                let nid = NeuronId::from_bytes(*neuron_uuid.as_bytes());
                if let Ok(neuron) = engine_lmdb::read_neuron(&shard.env, nid, None) {
                    let _ = genome::GenomeExecutor::execute_on_path_resolve(&neuron, winning_path.clone(), &mut resolve_ctx);
                }
            }
        }
        // Apply mutations generated during resolution (Hebbian reinforcement)
        let resolved_mutations = resolve_ctx.take_mutations();
        let _ = apply_mutations(&shard, resolved_mutations);

        (
            StatusCode::OK,
            Json(SpeculativeSearchResponse {
                success: true,
                path: Some(winning_path),
                total_cost: Some(total_cost),
                msg: "Speculative search succeeded".to_string(),
            }),
        ).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(SpeculativeSearchResponse {
                success: false,
                path: None,
                total_cost: None,
                msg: "No path found within depth limit or timeout exceeded".to_string(),
            }),
        ).into_response()
    }
}

fn explore_path(
    current_id: NeuronId,
    target_id: NeuronId,
    shard: Arc<ActiveShard>,
    all_shards: Vec<Arc<ActiveShard>>,
    mut visited: Vec<NeuronId>,
    mut path: Vec<String>,
    accumulated_cost: f32,
    cancelled: Arc<AtomicBool>,
    active_tasks: Arc<AtomicUsize>,
    max_parallel_paths: usize,
    max_depth: usize,
    tx: tokio::sync::mpsc::Sender<(Vec<String>, f32, Vec<genome::GraphMutation>)>,
    graph_ctx: genome::GraphContext,
) -> BoxFuture<'static, ()> {
    async move {
        // 1. Check early cancellation
        if cancelled.load(Ordering::SeqCst) {
            return;
        }

        // 2. Check depth
        if path.len() > max_depth {
            return;
        }

        // 3. Cycle / Loop Detection
        if visited.contains(&current_id) {
            tracing::debug!("Cycle detected on node {}", current_id);
            return;
        }
        visited.push(current_id);
        path.push(current_id.to_string());
        tracing::debug!("Exploring node: {}, path: {:?}", current_id, path);

        // 4. Read the current neuron.
        let mut active_shard = Arc::clone(&shard);
        let neuron = match engine_lmdb::read_neuron(&active_shard.env, current_id, None) {
            Ok(n) => n,
            Err(e) => {
                tracing::debug!("Node {} not found in current shard: {:?}", current_id, e);
                // Shard Boundary Routing: If not found in current shard, look in other active shards.
                let mut found = None;
                for other_shard in &all_shards {
                    if let Ok(n) = engine_lmdb::read_neuron(&other_shard.env, current_id, None) {
                        found = Some((n, Arc::clone(other_shard)));
                        break;
                    }
                }
                match found {
                    Some((n, new_shard)) => {
                        tracing::debug!("Node {} found in cross-shard boundary", current_id);
                        active_shard = new_shard;
                        n
                    }
                    None => {
                        tracing::debug!("Node {} not found in any shard, aborting path", current_id);
                        return; // dead-end (not found anywhere)
                    }
                }
            }
        };

        // 5. Run execute_on_path_step DNA hook
        let mut local_ctx = graph_ctx.clone();
        match genome::GenomeExecutor::execute_on_path_step(&neuron, path.clone(), &mut local_ctx) {
            Ok(continue_path) => {
                if !continue_path {
                    tracing::debug!("Node {} DNA on_path_step returned false (PRUNED)", current_id);
                    return; // DNA pruning rule hit!
                }
            }
            Err(e) => {
                tracing::error!("DNA on_path_step error for node {}: {:?}", current_id, e);
                return; // prune on script error to be safe
            }
        }

        // 6. Check if we reached target
        if current_id == target_id {
            tracing::debug!("REACHED TARGET node {}! Path: {:?}", current_id, path);
            let mutations = local_ctx.take_mutations();
            cancelled.store(true, Ordering::SeqCst);
            let _ = tx.send((path, accumulated_cost, mutations)).await;
            return;
        }

        // 7. Branching exploration
        let mut edges = neuron.adjacency;
        if edges.is_empty() {
            return; // dead-end
        }

        // Sort edges by weight descending (best path first)
        edges.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

        let mut first = true;
        for edge in edges {
            let target = edge.target_id;
            let cost = accumulated_cost + (1.0 - edge.weight);

            if first {
                first = false;
                explore_path(
                    target,
                    target_id,
                    Arc::clone(&active_shard),
                    all_shards.clone(),
                    visited.clone(),
                    path.clone(),
                    cost,
                    Arc::clone(&cancelled),
                    Arc::clone(&active_tasks),
                    max_parallel_paths,
                    max_depth,
                    tx.clone(),
                    graph_ctx.clone(),
                ).await;
            } else {
                let current_active = active_tasks.load(Ordering::SeqCst);
                if current_active < max_parallel_paths {
                    active_tasks.fetch_add(1, Ordering::SeqCst);
                    let shard_clone = Arc::clone(&active_shard);
                    let all_shards_clone = all_shards.clone();
                    let visited_clone = visited.clone();
                    let path_clone = path.clone();
                    let cancelled_clone = Arc::clone(&cancelled);
                    let active_tasks_clone = Arc::clone(&active_tasks);
                    let tx_clone = tx.clone();
                    let graph_ctx_clone = graph_ctx.clone();

                    tokio::spawn(async move {
                        let active_tasks_child = Arc::clone(&active_tasks_clone);
                        explore_path(
                            target,
                            target_id,
                            shard_clone,
                            all_shards_clone,
                            visited_clone,
                            path_clone,
                            cost,
                            cancelled_clone,
                            active_tasks_child,
                            max_parallel_paths,
                            max_depth,
                            tx_clone,
                            graph_ctx_clone,
                        ).await;
                        active_tasks_clone.fetch_sub(1, Ordering::SeqCst);
                    });
                } else {
                    explore_path(
                        target,
                        target_id,
                        Arc::clone(&active_shard),
                        all_shards.clone(),
                        visited.clone(),
                        path.clone(),
                        cost,
                        Arc::clone(&cancelled),
                        Arc::clone(&active_tasks),
                        max_parallel_paths,
                        max_depth,
                        tx.clone(),
                        graph_ctx.clone(),
                    ).await;
                }
            }
        }
    }.boxed()
}

fn apply_mutations(shard: &Arc<ActiveShard>, mutations: Vec<genome::GraphMutation>) -> Result<(), cluaizdb_errors::StorageError> {
    for mutation in mutations {
        match mutation {
            genome::GraphMutation::ConnectNeurons { source, target, weight } => {
                if let (Ok(src_uuid), Ok(tgt_uuid)) = (uuid::Uuid::parse_str(&source), uuid::Uuid::parse_str(&target)) {
                    let src_id = NeuronId::from_bytes(*src_uuid.as_bytes());
                    let tgt_id = NeuronId::from_bytes(*tgt_uuid.as_bytes());
                    if let Ok(mut src_neuron) = engine_lmdb::read_neuron(&shard.env, src_id, None) {
                        if let Some(edge) = src_neuron.adjacency.iter_mut().find(|e| e.target_id == tgt_id) {
                            edge.weight = weight;
                        } else {
                            src_neuron.adjacency.push(cluaizdb_types::NeuronEdge {
                                target_id: tgt_id,
                                weight,
                                last_accessed_ns: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_nanos() as u64,
                            });
                        }
                        let _ = engine_lmdb::write_neuron(&shard.env, &src_neuron);
                    }
                }
            }
            genome::GraphMutation::StrengthenEdge { source, target, amount } => {
                if let (Ok(src_uuid), Ok(tgt_uuid)) = (uuid::Uuid::parse_str(&source), uuid::Uuid::parse_str(&target)) {
                    let src_id = NeuronId::from_bytes(*src_uuid.as_bytes());
                    let tgt_id = NeuronId::from_bytes(*tgt_uuid.as_bytes());
                    if let Ok(mut src_neuron) = engine_lmdb::read_neuron(&shard.env, src_id, None) {
                        if let Some(edge) = src_neuron.adjacency.iter_mut().find(|e| e.target_id == tgt_id) {
                            edge.weight += amount;
                            if edge.weight > 1.0 {
                                edge.weight = 1.0;
                            }
                            let _ = engine_lmdb::write_neuron(&shard.env, &src_neuron);
                        }
                    }
                }
            }
            genome::GraphMutation::DecayEdge { source, target, amount } => {
                if let (Ok(src_uuid), Ok(tgt_uuid)) = (uuid::Uuid::parse_str(&source), uuid::Uuid::parse_str(&target)) {
                    let src_id = NeuronId::from_bytes(*src_uuid.as_bytes());
                    let tgt_id = NeuronId::from_bytes(*tgt_uuid.as_bytes());
                    if let Ok(mut src_neuron) = engine_lmdb::read_neuron(&shard.env, src_id, None) {
                        if let Some(edge) = src_neuron.adjacency.iter_mut().find(|e| e.target_id == tgt_id) {
                            edge.weight -= amount;
                            if edge.weight < 0.0 {
                                edge.weight = 0.0;
                            }
                            let _ = engine_lmdb::write_neuron(&shard.env, &src_neuron);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
