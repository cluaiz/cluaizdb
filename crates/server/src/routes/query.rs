use std::sync::Arc;
use axum::{Json, extract::State, http::{StatusCode, HeaderMap}, response::IntoResponse};
use serde::{Deserialize, Serialize};

use cluaizdb_types::UniversalNeuron;
use crate::routes::AppState;
use genome::cdql::{parser::{parse, CdqlValue, CompareOp}, planner::{build_plan, PlanStep}, eval_full_text, eval_geo_near, eval_range};

/// Request payload for `POST /query`.
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// Target database/tenant (e.g. "default_sandbox")
    pub tenant_id: Option<String>,
    /// CDQL Universal Query String (e.g. `find *(name: "Aryan") -> get friends`)
    #[serde(alias = "CDQL")]
    pub cdql: Option<String>,
    /// Raw DNA query string (JSON match, for WASM DNA engine)
    pub query_string: Option<String>,
    /// Legacy: Search Vector for Vector/Geospatial searching
    pub query_vector: Option<Vec<f32>>,
    /// Top-K results to return (overridden by CDQL `limit` clause if present)
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub neuron: UniversalNeuron,
    pub score: f32,
    /// Which query path matched: "CDQL", "wasm", "rhai", "vector"
    pub matched_by: String,
}

/// `POST /query` — Universal Query Handler
///
/// Supports three query modes (in priority order):
/// 1. **CDQL** — `find *(name: "Aryan") -> get friends -> limit 10`
/// 2. **WASM DNA** — Raw query string passed into the Neuron's `.wasm` module
/// 3. **Rhai DNA** — Legacy script-based query via `on_index`
pub async fn handle_query(
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
    Json(payload): Json<QueryRequest>,
) -> impl IntoResponse {
    let tenant_id = payload.tenant_id.as_deref().unwrap_or("default_sandbox");
    let global_limit = payload.limit.unwrap_or(100);

    let shard = match state.shard_manager.get_or_open_shard(tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to open tenant shard: {}", e) })),
            ).into_response();
        }
    };

    let all_neurons = match engine_lmdb::iter_all_neurons(&shard.env) {
        Ok(n) => n,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))
        ).into_response(),
    };

    // ──────────────────────────────────────────────────────────────
    // PATH 1: CDQL Universal Query
    // ──────────────────────────────────────────────────────────────
    if let Some(ref cdql_str) = payload.cdql {
        return execute_cdql(cdql_str, all_neurons, global_limit, shard).into_response();
    }

    // ──────────────────────────────────────────────────────────────
    // PATH 2 & 3: Legacy DNA-based queries (WASM or Rhai)
    // ──────────────────────────────────────────────────────────────
    let mut results: Vec<QueryResult> = Vec::new();
    let rhai_engine = genome::create_rhai_engine();

    for neuron in all_neurons {
        let mut score = 0.0f32;
        let mut matched_by = String::new();

        if let Some(ref dna) = neuron.dna {
            if let Some(index_script) = &dna.on_index {
                if dna.engine == "rhai" {
                    let mut scope = rhai::Scope::new();
                    if let Ok(dynamic_config) = rhai::serde::to_dynamic(&dna.parameters) {
                        scope.push_dynamic("config", dynamic_config);
                    }

                    let mut vec_arr = rhai::Array::new();
                    for v in &neuron.vector_data {
                        vec_arr.push(rhai::Dynamic::from(*v as rhai::FLOAT));
                    }
                    scope.push("vector_data", vec_arr);

                    if let Some(ref qv) = payload.query_vector {
                        let mut qv_arr = rhai::Array::new();
                        for v in qv {
                            qv_arr.push(rhai::Dynamic::from(*v as rhai::FLOAT));
                        }
                        scope.push("query_vector", qv_arr);
                    }

                    if let Ok(result_map) = rhai_engine.eval_with_scope::<rhai::Map>(&mut scope, index_script) {
                        if let Some(dist) = result_map.get("distance").and_then(|v| v.as_float().ok()) {
                            score = 1.0 / (1.0 + dist as f32);
                            matched_by = "rhai".to_string();
                        }
                        if let Some(s) = result_map.get("score").and_then(|v| v.as_float().ok()) {
                            score = s as f32;
                            matched_by = "rhai".to_string();
                        }
                    }

                } else if dna.engine == "wasm" {
                    if let Some(ref wasm_bytes) = dna.wasm_module {
                        let executor = genome::WasmExecutor::new();
                        let query_str = payload.query_string.as_deref().unwrap_or("{}");

                        if let Ok(res) = executor.execute_query(wasm_bytes, query_str, &neuron.raw_payload) {
                            if res > 0 {
                                score = 1.0;
                                matched_by = "wasm".to_string();
                            }
                        }
                    }
                }
            }
        }

        if score > 0.0 {
            results.push(QueryResult { neuron, score, matched_by });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(global_limit);

    (StatusCode::OK, Json(results)).into_response()
}

// ──────────────────────────────────────────────────────────────────
// CDQL Execution Engine
// ──────────────────────────────────────────────────────────────────

fn execute_cdql(
    cdql_str: &str,
    all_neurons: Vec<UniversalNeuron>,
    global_limit: usize,
    shard: Arc<crate::routes::shard_manager::ActiveShard>,
) -> impl IntoResponse {
    // Step 1: Parse
    let query = match parse(cdql_str) {
        Ok(q) => q,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("CDQL parse error: {}", e) })),
        ).into_response(),
    };

    // Step 2: Build plan
    let plan = match build_plan(&query) {
        Ok(p) => p,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("CDQL plan error: {}", e) })),
        ).into_response(),
    };

    // Step 3: Execute plan
    let mut working_set: Vec<&UniversalNeuron> = all_neurons.iter().collect();
    let effective_limit = plan.limit.min(global_limit);

    // O(1) neuron lookup by ID — used by traverse, join, shortest_path
    let neuron_index: std::collections::HashMap<String, &UniversalNeuron> =
        all_neurons.iter().map(|n| (n.id.to_string(), n)).collect();

    // State for GroupBy → Aggregate pipeline
    let mut group_buckets: Option<std::collections::HashMap<String, Vec<&UniversalNeuron>>> = None;
    let mut agg_results: Option<Vec<serde_json::Value>> = None;

    for step in &plan.steps {
        match step {
            PlanStep::FastPathIdLookup { id } => {
                working_set = working_set.into_iter().filter(|n| n.id.to_string() == *id).collect();
            }

            PlanStep::ScanAll { label_filter, filters } => {
                working_set = working_set.into_iter().filter(|n| {
                    let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                    if let Some(ref label) = label_filter {
                        if label != "*" && !payload_str.to_lowercase().contains(&label.to_lowercase()) {
                            return false;
                        }
                    }
                    for filter in filters {
                        if !matches_filter(n, filter, &payload_str) { return false; }
                    }
                    true
                }).collect();
            }

            PlanStep::FilterResults { field, op, value } => {
                working_set = working_set.into_iter().filter(|n| {
                    let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                    matches_filter(n, &cluaizdb_types_filter_ref(field, op, value), &payload_str)
                }).collect();
            }

            PlanStep::VectorScan { vector, metric: _ } => {
                let knn_results = shard.env.hnsw_index.search_knn(vector, effective_limit);
                let mut knn_set = std::collections::HashSet::new();
                for (id, _) in knn_results {
                    knn_set.insert(id.to_string());
                }
                working_set = working_set.into_iter().filter(|n| knn_set.contains(&n.id.to_string())).collect();
            }

            PlanStep::Limit(n) => {
                working_set.truncate(*n);
            }

            PlanStep::SortBy { field, ascending } => {
                working_set.sort_by(|a, b| {
                    let va = extract_json_field_str(&a.raw_payload, field);
                    let vb = extract_json_field_str(&b.raw_payload, field);
                    let cmp = va.cmp(&vb);
                    if *ascending { cmp } else { cmp.reverse() }
                });
            }

            // ── Full-Text / Inverted Search ─────────────────────────────────────────
            PlanStep::FullTextSearch { query, fuzzy } => {
                let mut scored: Vec<(&UniversalNeuron, f32)> = working_set
                    .iter()
                    .filter_map(|n| {
                        let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                        let score = eval_full_text(&payload_str, query, *fuzzy);
                        if score > 0.0 { Some((*n, score)) } else { None }
                    })
                    .collect();
                // Sort descending by relevance score.
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                working_set = scored.into_iter().map(|(n, _)| n).collect();
            }

            // ── Range Scan ───────────────────────────────────────────────────────────
            PlanStep::RangeScan { field, start, end } => {
                working_set = working_set.into_iter().filter(|n| {
                    let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                    eval_range(&payload_str, field, start, end)
                }).collect();
            }

            // ── Geo-Spatial Proximity ────────────────────────────────────────────────
            PlanStep::GeoNear { lat, lon, radius_km } => {
                let mut scored: Vec<(&UniversalNeuron, f32)> = working_set
                    .iter()
                    .filter_map(|n| {
                        let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                        eval_geo_near(&payload_str, *lat, *lon, *radius_km)
                            .map(|score| (*n, score))
                    })
                    .collect();
                // Sort descending: closest first (highest inverse-distance score).
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                working_set = scored.into_iter().map(|(n, _)| n).collect();
            }

            // ── Project: keep neurons that have at least one requested field ─────────
            PlanStep::Project { keep } => {
                working_set = working_set.into_iter().filter(|n| {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&n.raw_payload) {
                        keep.iter().any(|k| json.get(k).is_some())
                    } else { true }
                }).collect();
            }

            // ── Unwind: one result row per array element ──────────────────────────────
            PlanStep::Unwind { field } => {
                let mut expanded: Vec<&UniversalNeuron> = Vec::new();
                for n in &working_set {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&n.raw_payload) {
                        if let Some(arr) = json.get(field).and_then(|v| v.as_array()) {
                            for _ in arr { expanded.push(n); }
                            continue;
                        }
                    }
                    expanded.push(n);
                }
                working_set = expanded;
            }

            // ── Graph: multi-hop BFS traversal via adjacency list ─────────────────────
            PlanStep::GraphTraverse { edge: _, min_hops: _, max_hops, min_weight } => {
                struct MemFetcher<'a> { index: &'a std::collections::HashMap<String, &'a UniversalNeuron> }
                impl<'a> cluaizdb_graph_engine::NeuronFetcher for MemFetcher<'a> {
                    fn fetch(&self, id: &cluaizdb_types::NeuronId) -> Option<UniversalNeuron> {
                        self.index.get(&id.to_string()).map(|n| (*n).clone())
                    }
                }
                let fetcher = std::sync::Arc::new(MemFetcher { index: &neuron_index });
                let graph = cluaizdb_graph_engine::GraphEngine::new(fetcher);
                let config = cluaizdb_graph_engine::TraversalConfig {
                    max_depth: *max_hops,
                    min_weight: *min_weight as f32,
                    limit: effective_limit,
                };
                
                let mut graph_results = std::collections::HashSet::new();
                for start_node in &working_set {
                    if let Ok(paths) = graph.bfs_traverse(start_node.id.clone(), &config) {
                        for (tid, _) in paths {
                            if let Some(target) = neuron_index.get(&tid.to_string()) {
                                graph_results.insert(target.id.to_string());
                            }
                        }
                    }
                }
                working_set = working_set.into_iter().filter(|n| graph_results.contains(&n.id.to_string())).collect();
            }

            // ── Graph: BFS shortest path ──────────────────────────────────────────────
            PlanStep::ShortestPath { to_node } => {
                let mut path_neurons: Vec<&UniversalNeuron> = Vec::new();
                'outer: for start in &working_set {
                    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
                    let mut queue: std::collections::VecDeque<(String, Vec<String>)> =
                        std::collections::VecDeque::new();
                    let start_id = start.id.to_string();
                    visited.insert(start_id.clone());
                    queue.push_back((start_id, vec![]));
                    while let Some((cur, path)) = queue.pop_front() {
                        if cur == *to_node {
                            for pid in &path {
                                if let Some(n) = neuron_index.get(pid) { path_neurons.push(n); }
                            }
                            if let Some(n) = neuron_index.get(to_node) { path_neurons.push(n); }
                            break 'outer;
                        }
                        if let Some(neuron) = neuron_index.get(&cur) {
                            for e in &neuron.adjacency {
                                let nid = e.target_id.to_string();
                                if !visited.contains(&nid) {
                                    visited.insert(nid.clone());
                                    let mut np = path.clone();
                                    np.push(cur.clone());
                                    queue.push_back((nid, np));
                                }
                            }
                        }
                    }
                }
                working_set = path_neurons;
            }

            // ── Relational: in-memory hash join ──────────────────────────────────────
            PlanStep::RelationalJoin { target, on_left, on_right, .. } => {
                let right_map: std::collections::HashMap<String, &UniversalNeuron> = all_neurons
                    .iter()
                    .filter(|n| {
                        String::from_utf8_lossy(&n.raw_payload).contains(target.as_str())
                    })
                    .filter_map(|n| {
                        let json: serde_json::Value = serde_json::from_slice(&n.raw_payload).ok()?;
                        Some((json.get(on_right)?.to_string(), n))
                    })
                    .collect();
                working_set = working_set.into_iter().filter(|n| {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&n.raw_payload) {
                        let lv = json.get(on_left).map(|v| v.to_string()).unwrap_or_default();
                        right_map.contains_key(&lv)
                    } else { false }
                }).collect();
            }

            // ── SQL: group by one or more fields ─────────────────────────────────────
            PlanStep::GroupBy { fields } => {
                let mut buckets: std::collections::HashMap<String, Vec<&UniversalNeuron>> =
                    std::collections::HashMap::new();
                for n in &working_set {
                    let json: serde_json::Value =
                        serde_json::from_slice(&n.raw_payload).unwrap_or(serde_json::Value::Null);
                    let key = fields.iter()
                        .map(|f| json.get(f).map(|v| v.to_string()).unwrap_or_default())
                        .collect::<Vec<_>>().join("|");
                    buckets.entry(key).or_default().push(n);
                }
                group_buckets = Some(buckets);
            }

            // ── SQL: aggregate (count / sum / avg / min / max) ────────────────────────
            PlanStep::Aggregate { functions } => {
                let buckets = group_buckets.take().unwrap_or_else(|| {
                    let mut m = std::collections::HashMap::new();
                    m.insert("*".to_string(), working_set.clone());
                    m
                });
                let mut rows: Vec<serde_json::Value> = Vec::new();
                for (gkey, neurons) in &buckets {
                    let mut row = serde_json::json!({ "_group": gkey });
                    for func in functions {
                        match func {
                            genome::cdql::parser::AggFunc::Count => {
                                row["count"] = serde_json::json!(neurons.len());
                            }
                            genome::cdql::parser::AggFunc::Sum(field) => {
                                let v: f64 = neurons.iter().filter_map(|n| {
                                    serde_json::from_slice::<serde_json::Value>(&n.raw_payload)
                                        .ok()?.get(field)?.as_f64()
                                }).sum();
                                row[format!("sum_{field}")] = serde_json::json!(v);
                            }
                            genome::cdql::parser::AggFunc::Avg(field) => {
                                let vals: Vec<f64> = neurons.iter().filter_map(|n| {
                                    serde_json::from_slice::<serde_json::Value>(&n.raw_payload)
                                        .ok()?.get(field)?.as_f64()
                                }).collect();
                                if !vals.is_empty() {
                                    row[format!("avg_{field}")] = serde_json::json!(
                                        vals.iter().sum::<f64>() / vals.len() as f64);
                                }
                            }
                            genome::cdql::parser::AggFunc::Min(field) => {
                                let m = neurons.iter().filter_map(|n| {
                                    serde_json::from_slice::<serde_json::Value>(&n.raw_payload)
                                        .ok()?.get(field)?.as_f64()
                                }).fold(f64::INFINITY, f64::min);
                                if m.is_finite() { row[format!("min_{field}")] = serde_json::json!(m); }
                            }
                            genome::cdql::parser::AggFunc::Max(field) => {
                                let m = neurons.iter().filter_map(|n| {
                                    serde_json::from_slice::<serde_json::Value>(&n.raw_payload)
                                        .ok()?.get(field)?.as_f64()
                                }).fold(f64::NEG_INFINITY, f64::max);
                                if m.is_finite() { row[format!("max_{field}")] = serde_json::json!(m); }
                            }
                        }
                    }
                    rows.push(row);
                }
                agg_results = Some(rows);
                working_set = vec![];
            }

            // ── Time-Series: bucket neurons by time window ────────────────────────────
            PlanStep::TimeWindow { size } => {
                let window_ns = parse_time_window_ns(size);
                let mut buckets: std::collections::HashMap<String, Vec<&UniversalNeuron>> =
                    std::collections::HashMap::new();
                for n in &working_set {
                    let ts = get_timestamp_ns(n);
                    let key = if window_ns > 0 {
                        format!("{}", (ts / window_ns) * window_ns)
                    } else { ts.to_string() };
                    buckets.entry(key).or_default().push(n);
                }
                group_buckets = Some(buckets);
            }

            // ── Blob: byte-range pre-filter (actual slicing at serialisation) ─────────
            PlanStep::ByteStream { start_byte, .. } => {
                working_set = working_set.into_iter()
                    .filter(|n| n.raw_payload.len() > *start_byte)
                    .collect();
            }

            PlanStep::InsertData { .. } => {}
        }
    }

    // ── Aggregate short-circuit: return grouped rows directly ─────────────────────
    if let Some(rows) = agg_results {
        return (StatusCode::OK, Json(serde_json::json!(rows))).into_response();
    }

    // ── Capture byte-stream range for per-neuron slicing at serialisation ────────
    let byte_range = plan.steps.iter().find_map(|s| {
        if let PlanStep::ByteStream { start_byte, end_byte } = s {
            Some((*start_byte, *end_byte))
        } else { None }
    });

    // Build scored working set — preserve actual search/geo scores from pipeline.
    // For pure filter queries (no scoring step), use 1.0 as the default.
    let mut scored: Vec<(&UniversalNeuron, f32)> = working_set
        .into_iter()
        .map(|n| {
            let mut best_score: f32 = 1.0;
            for step in &plan.steps {
                match step {
                    PlanStep::FullTextSearch { query, fuzzy } => {
                        let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                        let s = eval_full_text(&payload_str, query, *fuzzy);
                        if s > 0.0 { best_score = s; }
                    }
                    PlanStep::GeoNear { lat, lon, radius_km } => {
                        let payload_str = String::from_utf8_lossy(&n.raw_payload).to_string();
                        if let Some(s) = eval_geo_near(&payload_str, *lat, *lon, *radius_km) {
                            best_score = s;
                        }
                    }
                    _ => {}
                }
            }
            (n, best_score)
        })
        .collect();

    scored.truncate(effective_limit);

    let results: Vec<QueryResult> = scored.into_iter().map(|(n, score)| {
        let neuron = if let Some((start, end)) = byte_range {
            let mut n2 = n.clone();
            let end_c = end.min(n2.raw_payload.len());
            let start_c = start.min(end_c);
            n2.raw_payload = n2.raw_payload.slice(start_c..end_c);
            n2
        } else { n.clone() };
        QueryResult { neuron, score, matched_by: "CDQL".to_string() }
    }).collect();

    (StatusCode::OK, Json(results)).into_response()
}


/// Check if a neuron matches a single filter condition
fn matches_filter(
    _neuron: &UniversalNeuron,
    filter: &genome::cdql::parser::Filter,
    payload_str: &str,
) -> bool {
    // Try to parse payload as JSON for field extraction
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(payload_str) {
        let field_val = json_val.get(&filter.field);

        match (&filter.op, &filter.value, field_val) {
            (CompareOp::Eq, CdqlValue::Text(t), Some(v)) => {
                v.as_str().map(|s| s == t.as_str()).unwrap_or(false)
            }
            (CompareOp::Eq, CdqlValue::Number(n), Some(v)) => {
                v.as_f64().map(|x| (x - n).abs() < 1e-9).unwrap_or(false)
            }
            (CompareOp::NotEq, CdqlValue::Text(t), Some(v)) => {
                v.as_str().map(|s| s != t.as_str()).unwrap_or(true)
            }
            (CompareOp::Gt, CdqlValue::Number(n), Some(v)) => {
                v.as_f64().map(|x| x > *n).unwrap_or(false)
            }
            (CompareOp::Lt, CdqlValue::Number(n), Some(v)) => {
                v.as_f64().map(|x| x < *n).unwrap_or(false)
            }
            (CompareOp::Gte, CdqlValue::Number(n), Some(v)) => {
                v.as_f64().map(|x| x >= *n).unwrap_or(false)
            }
            (CompareOp::Lte, CdqlValue::Number(n), Some(v)) => {
                v.as_f64().map(|x| x <= *n).unwrap_or(false)
            }
            (CompareOp::Contains, CdqlValue::Text(t), Some(v)) => {
                v.as_str().map(|s| s.contains(t.as_str())).unwrap_or(false)
            }
            // Field not found — no match
            (_, _, None) => false,
            _ => false,
        }
    } else {
        // Non-JSON payload: fallback to string contains match
        matches!(&filter.op, CompareOp::Contains | CompareOp::Eq) &&
            matches!(&filter.value, CdqlValue::Text(t) if payload_str.contains(t.as_str()))
    }
}

/// Helper to build a temporary Filter reference for inline use
fn cluaizdb_types_filter_ref<'a>(
    field: &'a str,
    op: &'a CompareOp,
    value: &'a CdqlValue,
) -> genome::cdql::parser::Filter {
    genome::cdql::parser::Filter {
        field: field.to_string(),
        op: op.clone(),
        value: value.clone(),
    }
}

/// Extract a JSON field from raw_payload bytes as a string (for sorting)
fn extract_json_field_str(raw: &bytes::Bytes, field: &str) -> String {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(raw) {
        json.get(field)
            .map(|v| v.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Parse a time window string like "1m", "5m", "1h", "1d" into nanoseconds.
fn parse_time_window_ns(size: &str) -> u64 {
    let s = size.trim();
    if let Some(num_str) = s.strip_suffix('d') {
        num_str.parse::<u64>().unwrap_or(1) * 86_400 * 1_000_000_000
    } else if let Some(num_str) = s.strip_suffix('h') {
        num_str.parse::<u64>().unwrap_or(1) * 3_600 * 1_000_000_000
    } else if let Some(num_str) = s.strip_suffix('m') {
        num_str.parse::<u64>().unwrap_or(1) * 60 * 1_000_000_000
    } else if let Some(num_str) = s.strip_suffix('s') {
        num_str.parse::<u64>().unwrap_or(1) * 1_000_000_000
    } else {
        60 * 1_000_000_000 // default: 1 minute
    }
}

/// Get the timestamp (in nanoseconds) from a neuron.
/// Tries to read a "timestamp" field from the JSON payload first,
/// then falls back to `created_at_ns`.
fn get_timestamp_ns(n: &UniversalNeuron) -> u64 {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&n.raw_payload) {
        // Try common timestamp field names
        for key in &["timestamp", "ts", "time", "created_at"] {
            if let Some(v) = json.get(*key) {
                if let Some(num) = v.as_u64() { return num; }
                if let Some(num) = v.as_f64() { return num as u64; }
            }
        }
    }
    n.created_at_ns
}
