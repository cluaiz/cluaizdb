use std::sync::Arc;
use std::time::Instant;
use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv, TraversalConfig};
use cluaizdb_index_mvhsnw::{CosineDistance, DistanceMetric, HnswIndex};
use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::eval::{eval_full_text, eval_geo_near, eval_range};
use crate::parser::{AggFunc, CdqlValue, CompareOp, Filter, JoinType};
use crate::planner::{PlanStep, QueryPlan};

/// Parse a string or decimal ID into a 128-bit unsigned integer
pub fn parse_or_hash_id(s: &str) -> u128 {
    if let Ok(num) = s.parse::<u128>() {
        num
    } else {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        s.hash(&mut h1);
        let part1 = h1.finish();
        let mut h2 = DefaultHasher::new();
        s.chars().rev().collect::<String>().hash(&mut h2);
        let part2 = h2.finish();
        ((part1 as u128) << 64) | (part2 as u128)
    }
}

/// Execution Context holding references to substrate and compute engines
pub struct QueryContext<'a> {
    pub substrate: &'a SubstrateEnv,
    pub col_id: u32,
    pub vector_index: Option<&'a Arc<HnswIndex<CosineDistance>>>,
    pub wasm_runner: Option<&'a Arc<WasmGenomeRunner>>,
}

impl<'a> QueryContext<'a> {
    pub fn new(substrate: &'a SubstrateEnv, col_id: u32) -> Self {
        Self {
            substrate,
            col_id,
            vector_index: None,
            wasm_runner: None,
        }
    }

    pub fn with_vector_index(
        substrate: &'a SubstrateEnv,
        col_id: u32,
        vector_index: &'a Arc<HnswIndex<CosineDistance>>,
    ) -> Self {
        Self {
            substrate,
            col_id,
            vector_index: Some(vector_index),
            wasm_runner: None,
        }
    }

    pub fn with_wasm_runner(
        substrate: &'a SubstrateEnv,
        col_id: u32,
        wasm_runner: &'a Arc<WasmGenomeRunner>,
    ) -> Self {
        Self {
            substrate,
            col_id,
            vector_index: None,
            wasm_runner: Some(wasm_runner),
        }
    }
}

/// The result of executing a CDQL QueryPlan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub records: Vec<Value>,
    pub count: usize,
    pub execution_time_us: u64,
}

/// Helper function to match a JSON value with a CDQL filter
fn matches_filter(json: &Value, filter: &Filter) -> bool {
    let field_val = match json.get(&filter.field) {
        Some(v) => v,
        None => return false,
    };

    match (&filter.op, &filter.value) {
        (CompareOp::Eq, CdqlValue::Text(t)) => field_val.as_str() == Some(t.as_str()),
        (CompareOp::Eq, CdqlValue::Number(n)) => field_val.as_f64().map_or(false, |v| (v - n).abs() < f64::EPSILON),
        (CompareOp::Eq, CdqlValue::Bool(b)) => field_val.as_bool() == Some(*b),
        (CompareOp::Eq, CdqlValue::Null) => field_val.is_null(),

        (CompareOp::NotEq, CdqlValue::Text(t)) => field_val.as_str() != Some(t.as_str()),
        (CompareOp::NotEq, CdqlValue::Number(n)) => field_val.as_f64().map_or(true, |v| (v - n).abs() >= f64::EPSILON),
        (CompareOp::NotEq, CdqlValue::Bool(b)) => field_val.as_bool() != Some(*b),
        (CompareOp::NotEq, CdqlValue::Null) => !field_val.is_null(),

        (CompareOp::Gt, CdqlValue::Number(n)) => field_val.as_f64().map_or(false, |v| v > *n),
        (CompareOp::Lt, CdqlValue::Number(n)) => field_val.as_f64().map_or(false, |v| v < *n),
        (CompareOp::Gte, CdqlValue::Number(n)) => field_val.as_f64().map_or(false, |v| v >= *n),
        (CompareOp::Lte, CdqlValue::Number(n)) => field_val.as_f64().map_or(false, |v| v <= *n),

        (CompareOp::Contains, CdqlValue::Text(t)) => {
            if let Some(arr) = field_val.as_array() {
                arr.iter().any(|item| item.as_str() == Some(t.as_str()))
            } else if let Some(s) = field_val.as_str() {
                s.contains(t.as_str())
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Executes a compiled `QueryPlan` against the physical substrate.
pub fn execute_plan(plan: &QueryPlan, ctx: &QueryContext) -> Result<QueryResult, String> {
    let start_time = Instant::now();

    // 1. FAST-PATH EXECUTION (Sub-microsecond point lookup for single-step queries)
    if plan.is_fast_path && plan.steps.len() == 1 {
        if let Some(PlanStep::FastPathIdLookup { id }) = plan.steps.first() {
            let record_id = parse_or_hash_id(id);
            match ctx.substrate.get_record(ctx.col_id, record_id) {
                Ok(Some(raw_bytes)) => {
                    let json_val = serde_json::from_slice(&raw_bytes).unwrap_or_else(|_| {
                        Value::String(String::from_utf8_lossy(&raw_bytes).to_string())
                    });
                    let elapsed = start_time.elapsed().as_micros() as u64;
                    return Ok(QueryResult {
                        records: vec![json_val],
                        count: 1,
                        execution_time_us: elapsed,
                    });
                }
                Ok(None) => {
                    let elapsed = start_time.elapsed().as_micros() as u64;
                    return Ok(QueryResult {
                        records: vec![],
                        count: 0,
                        execution_time_us: elapsed,
                    });
                }
                Err(e) => return Err(format!("Storage error during fast-path lookup: {}", e)),
            }
        }
    }

    // 2. PIPELINE EXECUTION
    let mut working_set: Vec<(u128, Value)> = Vec::new();

    for step in &plan.steps {
        match step {
            PlanStep::FastPathIdLookup { id } => {
                let record_id = parse_or_hash_id(id);
                if let Ok(Some(bytes)) = ctx.substrate.get_record(ctx.col_id, record_id) {
                    let val: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                        Value::String(String::from_utf8_lossy(&bytes).to_string())
                    });
                    working_set.push((record_id, val));
                }
            }

            PlanStep::ScanAll { label_filter, filters } => {
                let target_col_id = if let Some(lbl) = label_filter {
                    if lbl != "*" {
                        ctx.substrate.get_collection_id(lbl).unwrap_or(None).unwrap_or(ctx.col_id)
                    } else {
                        ctx.col_id
                    }
                } else {
                    ctx.col_id
                };

                // Check if any filter is an equality match that can use secondary index seek
                let mut index_hit = false;
                for f in filters {
                    if f.op == CompareOp::Eq {
                        let term_opt = match &f.value {
                            CdqlValue::Text(t) => Some(format!("{}:{}", f.field, t)),
                            CdqlValue::Number(n) => Some(format!("{}:{}", f.field, n)),
                            CdqlValue::Bool(b) => Some(format!("{}:{}", f.field, b)),
                            _ => None,
                        };
                        if let Some(term) = term_opt {
                            if let Ok(matching_ids) = ctx.substrate.seek_index(target_col_id, term.as_bytes()) {
                                if !matching_ids.is_empty() {
                                    for rec_id in matching_ids {
                                        if let Ok(Some(bytes)) = ctx.substrate.get_record(target_col_id, rec_id) {
                                            if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                                                if filters.iter().all(|flt| matches_filter(&val, flt)) {
                                                    working_set.push((rec_id, val));
                                                }
                                            }
                                        }
                                    }
                                    index_hit = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !index_hit {
                    // Fall back to sequential B+Tree scan
                    let raw_records = ctx.substrate
                        .scan_records(target_col_id, 10_000)
                        .map_err(|e| format!("Scan error: {}", e))?;

                    for (rec_id, bytes) in raw_records {
                        let val: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                            Value::String(String::from_utf8_lossy(&bytes).to_string())
                        });

                        let mut matched = true;
                        for f in filters {
                            if !matches_filter(&val, f) {
                                matched = false;
                                break;
                            }
                        }

                        if matched {
                            working_set.push((rec_id, val));
                        }
                    }
                }
            }

            PlanStep::FilterResults { field, op, value } => {
                let filter = Filter {
                    field: field.clone(),
                    op: op.clone(),
                    value: value.clone(),
                };
                working_set.retain(|(_, v)| matches_filter(v, &filter));
            }

            PlanStep::Project { keep } => {
                for (_, val) in working_set.iter_mut() {
                    if let Some(obj) = val.as_object_mut() {
                        let mut filtered = serde_json::Map::new();
                        for key in keep {
                            if let Some(item) = obj.get(key) {
                                filtered.insert(key.clone(), item.clone());
                            }
                        }
                        *val = Value::Object(filtered);
                    }
                }
            }

            PlanStep::Unwind { field } => {
                let mut unwound = Vec::new();
                for (id, val) in &working_set {
                    if let Some(arr) = val.get(field).and_then(|v| v.as_array()) {
                        for item in arr {
                            let mut clone_val = val.clone();
                            if let Some(obj) = clone_val.as_object_mut() {
                                obj.insert(field.clone(), item.clone());
                            }
                            unwound.push((*id, clone_val));
                        }
                    } else {
                        unwound.push((*id, val.clone()));
                    }
                }
                working_set = unwound;
            }

            PlanStep::VectorScan { vector, .. } => {
                let mut new_set = Vec::new();
                if let Some(index) = ctx.vector_index {
                    let top_k = plan.limit.max(10);
                    let results = index.search_knn(vector, top_k);
                    for (rec_id, _) in results {
                        if let Ok(Some(bytes)) = ctx.substrate.get_record(ctx.col_id, rec_id as u128) {
                            if let Ok(val) = serde_json::from_slice(&bytes) {
                                new_set.push((rec_id as u128, val));
                            }
                        }
                    }
                } else if let Ok(stored_vectors) = ctx.substrate.scan_vectors(ctx.col_id, 10_000) {
                    let mut scored: Vec<(u128, f32)> = stored_vectors
                        .into_iter()
                        .map(|(id, v)| {
                            let dist = CosineDistance::calculate(vector, &v);
                            (id, dist)
                        })
                        .collect();
                    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    let top_k = plan.limit.max(10);
                    for (rec_id, _) in scored.into_iter().take(top_k) {
                        if let Ok(Some(bytes)) = ctx.substrate.get_record(ctx.col_id, rec_id) {
                            if let Ok(val) = serde_json::from_slice(&bytes) {
                                new_set.push((rec_id, val));
                            }
                        }
                    }
                }
                working_set = new_set;
            }

            PlanStep::GraphTraverse { min_hops, max_hops, min_weight, .. } => {
                let mut new_set = Vec::new();
                let config = TraversalConfig {
                    max_depth: *max_hops,
                    min_weight: *min_weight as f32,
                    direction: cluaizdb_substrate::TraversalDirection::Outbound,
                    edge_type: None,
                    limit: 1000,
                };

                for (id, _) in &working_set {
                    if let Ok(nodes) = ctx.substrate.traverse_graph(*id, &config) {
                        for node in nodes {
                            if node.depth >= *min_hops {
                                if let Ok(Some(bytes)) = ctx.substrate.get_record(ctx.col_id, node.node_id) {
                                    if let Ok(val) = serde_json::from_slice(&bytes) {
                                        new_set.push((node.node_id, val));
                                    }
                                }
                            }
                        }
                    }
                }
                working_set = new_set;
            }

            PlanStep::FullTextSearch { query, fuzzy } => {
                working_set.retain(|(_, v)| {
                    let json_str = v.to_string();
                    eval_full_text(&json_str, query, *fuzzy) > 0.0
                });
            }

            PlanStep::GeoNear { lat, lon, radius_km } => {
                working_set.retain(|(_, v)| {
                    let json_str = v.to_string();
                    eval_geo_near(&json_str, *lat, *lon, *radius_km).is_some()
                });
            }

            PlanStep::RangeScan { field, start, end } => {
                working_set.retain(|(_, v)| {
                    let json_str = v.to_string();
                    eval_range(&json_str, field, start, end)
                });
            }

            PlanStep::SortBy { field, ascending } => {
                working_set.sort_by(|(_, a), (_, b)| {
                    let val_a = a.get(field);
                    let val_b = b.get(field);
                    match (val_a, val_b) {
                        (Some(Value::Number(na)), Some(Value::Number(nb))) => {
                            let cmp = na.as_f64().unwrap_or(0.0).total_cmp(&nb.as_f64().unwrap_or(0.0));
                            if *ascending { cmp } else { cmp.reverse() }
                        }
                        (Some(Value::String(sa)), Some(Value::String(sb))) => {
                            let cmp = sa.cmp(sb);
                            if *ascending { cmp } else { cmp.reverse() }
                        }
                        _ => std::cmp::Ordering::Equal,
                    }
                });
            }

            PlanStep::Limit(limit) => {
                if working_set.len() > *limit {
                    working_set.truncate(*limit);
                }
            }

            PlanStep::WasmTransform { genome_id, func } => {
                let default_runner;
                let runner = if let Some(r) = ctx.wasm_runner {
                    r
                } else {
                    default_runner = Arc::new(WasmGenomeRunner::new(SandboxConfig::default())
                        .map_err(|e| format!("WASM runner initialization failed: {}", e))?);
                    &default_runner
                };

                let bytecode = ctx.substrate.get_genome(*genome_id)
                    .map_err(|e| format!("Storage error fetching WASM genome {}: {}", genome_id, e))?
                    .ok_or_else(|| format!("WASM genome id {} not found in substrate", genome_id))?;

                let module = runner.compile(&bytecode)
                    .map_err(|e| format!("WASM compilation failed for genome {}: {}", genome_id, e))?;

                for (_, val) in working_set.iter_mut() {
                    let input_bytes = serde_json::to_vec(val)
                        .map_err(|e| format!("Serialization error for WASM input: {}", e))?;

                    let output_bytes = runner.execute_memory_transform(&module, func, &input_bytes)
                        .map_err(|e| format!("WASM runtime execution trapped in function '{}': {}", func, e))?;

                    let transformed: Value = serde_json::from_slice(&output_bytes).unwrap_or_else(|_| {
                        Value::String(String::from_utf8_lossy(&output_bytes).to_string())
                    });
                    *val = transformed;
                }
            }

            PlanStep::TimeWindow { size } => {
                let window_ms = parse_time_window_ms(size);
                let mut valid_records: Vec<(u64, (u128, Value))> = Vec::new();
                for (id, val) in &working_set {
                    if let Some(ts) = extract_timestamp(val) {
                        valid_records.push((ts, (*id, val.clone())));
                    }
                }

                if !valid_records.is_empty() {
                    let max_ts = valid_records.iter().map(|(ts, _)| *ts).max().unwrap_or(0);
                    let threshold = max_ts.saturating_sub(window_ms);
                    working_set = valid_records
                        .into_iter()
                        .filter(|(ts, _)| *ts >= threshold)
                        .map(|(_, item)| item)
                        .collect();
                } else {
                    working_set.clear();
                }
            }

            PlanStep::Aggregate { functions } => {
                let mut agg_result = serde_json::Map::new();
                for func in functions {
                    match func {
                        AggFunc::Count => {
                            agg_result.insert("count".to_string(), Value::from(working_set.len()));
                        }
                        AggFunc::Sum(field) => {
                            let sum: f64 = working_set.iter()
                                .filter_map(|(_, v)| v.get(field).and_then(|x| x.as_f64()))
                                .sum();
                            agg_result.insert(format!("sum_{}", field), Value::from(sum));
                        }
                        AggFunc::Avg(field) => {
                            let numbers: Vec<f64> = working_set.iter()
                                .filter_map(|(_, v)| v.get(field).and_then(|x| x.as_f64()))
                                .collect();
                            let avg = if !numbers.is_empty() {
                                numbers.iter().sum::<f64>() / numbers.len() as f64
                            } else {
                                0.0
                            };
                            agg_result.insert(format!("avg_{}", field), Value::from(avg));
                        }
                        AggFunc::Min(field) => {
                            let min = working_set.iter()
                                .filter_map(|(_, v)| v.get(field).and_then(|x| x.as_f64()))
                                .fold(f64::INFINITY, f64::min);
                            if min != f64::INFINITY {
                                agg_result.insert(format!("min_{}", field), Value::from(min));
                            }
                        }
                        AggFunc::Max(field) => {
                            let max = working_set.iter()
                                .filter_map(|(_, v)| v.get(field).and_then(|x| x.as_f64()))
                                .fold(f64::NEG_INFINITY, f64::max);
                            if max != f64::NEG_INFINITY {
                                agg_result.insert(format!("max_{}", field), Value::from(max));
                            }
                        }
                    }
                }
                working_set = vec![(0u128, Value::Object(agg_result))];
            }

            PlanStep::GroupBy { fields } => {
                let mut groups: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
                for (_, val) in &working_set {
                    let group_key = fields.iter()
                        .map(|f| val.get(f).map(|v| v.to_string()).unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join("::");
                    groups.entry(group_key).or_default().push(val.clone());
                }
                working_set = groups.into_iter().enumerate().map(|(idx, (key, items))| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("group".to_string(), Value::String(key));
                    obj.insert("items".to_string(), Value::Array(items));
                    (idx as u128, Value::Object(obj))
                }).collect();
            }

            PlanStep::RelationalJoin { target, on_left, on_right, join_type } => {
                let target_col = target.parse::<u32>().unwrap_or_else(|_| {
                    ctx.substrate.get_collection_id(target).unwrap_or(None).unwrap_or(ctx.col_id)
                });
                let target_records = ctx.substrate.scan_records(target_col, 5000).unwrap_or_default();
                let mut target_map: std::collections::HashMap<String, Value> = std::collections::HashMap::new();

                for (_, bytes) in target_records {
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        if let Some(key_val) = val.get(on_right) {
                            let k_str = key_val.to_string();
                            target_map.insert(k_str, val);
                        }
                    }
                }

                let mut joined = Vec::new();
                for (id, val) in &working_set {
                    let left_key = val.get(on_left).map(|k| k.to_string()).unwrap_or_default();
                    if let Some(target_val) = target_map.get(&left_key) {
                        let mut merged = val.clone();
                        if let (Some(left_obj), Some(right_obj)) = (merged.as_object_mut(), target_val.as_object()) {
                            for (k, v) in right_obj {
                                left_obj.insert(format!("{}_{}", target, k), v.clone());
                            }
                        }
                        joined.push((*id, merged));
                    } else if matches!(join_type, JoinType::Left | JoinType::Full) {
                        joined.push((*id, val.clone()));
                    }
                }
                working_set = joined;
            }

            PlanStep::ShortestPath { to_node } => {
                let target_id = parse_or_hash_id(to_node);
                let mut path_set = Vec::new();
                let config = TraversalConfig {
                    max_depth: 10,
                    min_weight: 0.0,
                    direction: cluaizdb_substrate::TraversalDirection::Outbound,
                    edge_type: None,
                    limit: 100,
                };
                for (id, _) in &working_set {
                    if let Ok(Some(path_ids)) = ctx.substrate.find_shortest_path(*id, target_id, &config) {
                        for path_node_id in path_ids {
                            if let Ok(Some(bytes)) = ctx.substrate.get_record(ctx.col_id, path_node_id) {
                                if let Ok(val) = serde_json::from_slice(&bytes) {
                                    path_set.push((path_node_id, val));
                                }
                            }
                        }
                    }
                }
                working_set = path_set;
            }

            PlanStep::InsertData { label, data } => {
                let target_col_id = if !label.is_empty() && label != "*" {
                    ctx.substrate.get_or_create_collection(label).unwrap_or(ctx.col_id)
                } else {
                    ctx.col_id
                };

                let mut map = serde_json::Map::new();
                let mut rec_id = None;

                for (k, v) in data {
                    let j_val = cdql_val_to_json(v);
                    if k == "id" {
                        if let Some(id_num) = j_val.as_u64() {
                            rec_id = Some(id_num as u128);
                        } else if let Some(id_str) = j_val.as_str() {
                            rec_id = Some(parse_or_hash_id(id_str));
                        }
                    }
                    map.insert(k.clone(), j_val);
                }

                let record_id = rec_id.unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u128)
                        .unwrap_or(1)
                });

                let val = Value::Object(map);
                if let Ok(bytes) = serde_json::to_vec(&val) {
                    if let Ok(()) = ctx.substrate.put_record(target_col_id, record_id, &bytes) {
                        // Automatically index string, numeric, and bool terms
                        if let Some(obj) = val.as_object() {
                            for (k, v) in obj {
                                let term_str = match v {
                                    Value::String(s) => format!("{}:{}", k, s),
                                    Value::Number(n) => format!("{}:{}", k, n),
                                    Value::Bool(b) => format!("{}:{}", k, b),
                                    _ => continue,
                                };
                                let _ = ctx.substrate.index_term(target_col_id, term_str.as_bytes(), record_id);
                            }
                        }
                        working_set.push((record_id, val));
                    }
                }
            }

            PlanStep::ByteStream { start_byte, end_byte } => {
                for (_, val) in working_set.iter_mut() {
                    if let Some(s) = val.as_str() {
                        let b = s.as_bytes();
                        let start = (*start_byte).min(b.len());
                        let end = (*end_byte).min(b.len());
                        if start <= end {
                            *val = Value::String(String::from_utf8_lossy(&b[start..end]).to_string());
                        }
                    }
                }
            }
        }
    }

    if plan.limit > 0 && working_set.len() > plan.limit {
        working_set.truncate(plan.limit);
    }

    let records: Vec<Value> = working_set.into_iter().map(|(_, v)| v).collect();
    let count = records.len();
    let elapsed = start_time.elapsed().as_micros() as u64;

    Ok(QueryResult {
        records,
        count,
        execution_time_us: elapsed,
    })
}

fn parse_time_window_ms(size: &str) -> u64 {
    let s = size.trim().to_lowercase();
    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse().unwrap_or(1000)
    } else if s.ends_with('s') {
        s.trim_end_matches('s').parse::<u64>().unwrap_or(1) * 1000
    } else if s.ends_with('m') {
        s.trim_end_matches('m').parse::<u64>().unwrap_or(1) * 60 * 1000
    } else if s.ends_with('h') {
        s.trim_end_matches('h').parse::<u64>().unwrap_or(1) * 3600 * 1000
    } else if s.ends_with('d') {
        s.trim_end_matches('d').parse::<u64>().unwrap_or(1) * 86400 * 1000
    } else {
        s.parse::<u64>().unwrap_or(60_000)
    }
}

fn extract_timestamp(val: &Value) -> Option<u64> {
    for candidate in &["timestamp_ms", "timestamp", "created_at", "time", "ts"] {
        if let Some(n) = val.get(*candidate).and_then(|v| v.as_u64()) {
            return Some(n);
        }
        if let Some(f) = val.get(*candidate).and_then(|v| v.as_f64()) {
            return Some(f as u64);
        }
    }
    None
}

fn cdql_val_to_json(val: &CdqlValue) -> Value {
    match val {
        CdqlValue::Text(s) => Value::String(s.clone()),
        CdqlValue::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Value::Number(num)
            } else {
                Value::Null
            }
        }
        CdqlValue::Bool(b) => Value::Bool(*b),
        CdqlValue::Vector(v) => Value::Array(v.iter().map(|f| Value::from(*f)).collect()),
        CdqlValue::Parameter | CdqlValue::Null => Value::Null,
    }
}

/// Rehydrates an in-memory HNSW index with all durable vectors stored in SubstrateEnv for a collection.
pub fn rehydrate_hnsw_from_substrate(
    substrate: &SubstrateEnv,
    col_id: u32,
    index: &HnswIndex<CosineDistance>,
) -> Result<usize, String> {
    let vectors = substrate
        .scan_vectors(col_id, usize::MAX)
        .map_err(|e| format!("Failed to scan durable vectors from substrate: {}", e))?;
    let count = vectors.len();
    for (id, vec) in vectors {
        index.insert(id, vec);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::parser::parse;
    use crate::planner::build_plan;

    #[test]
    fn test_execute_fast_path_point_lookup() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 1;

        let record_id = 42u128;
        let payload = serde_json::json!({
            "name": "Aryan",
            "role": "Chief Architect",
            "active": true
        });
        substrate.put_record(col_id, record_id, &serde_json::to_vec(&payload).unwrap()).unwrap();

        let ast = parse("find id(\"42\")").unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);

        let result = execute_plan(&plan, &ctx).unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(result.records[0]["name"], "Aryan");
        assert_eq!(result.records[0]["role"], "Chief Architect");
    }

    #[test]
    fn test_execute_scan_with_filter_and_project() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 1;

        for i in 1..=5 {
            let data = serde_json::json!({
                "id": i,
                "score": i * 10,
                "tier": if i > 3 { "gold" } else { "silver" }
            });
            substrate.put_record(col_id, i as u128, &serde_json::to_vec(&data).unwrap()).unwrap();
        }

        let ast = parse("find *(tier: \"gold\") -> project(keep: [\"id\", \"score\"])").unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);

        let result = execute_plan(&plan, &ctx).unwrap();
        assert_eq!(result.count, 2);
        for rec in &result.records {
            assert!(rec.get("tier").is_none()); // Project stripped 'tier'
            assert!(rec.get("score").is_some());
        }
    }

    #[test]
    fn test_execute_wasm_transform_pipeline() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 1;

        // Ingest record
        let data = serde_json::json!({
            "id": 101,
            "name": "Aryan",
            "clearance": "TopSecret"
        });
        substrate.put_record(col_id, 101, &serde_json::to_vec(&data).unwrap()).unwrap();

        // 2. Compile and store WASM genome into substrate
        let wat_src = r#"
            (module
                (memory (export "memory") 1)
                (func (export "anonymize") (param $offset i32) (param $len i32) (result i32 i32)
                    ;; Write '{"anonymized":true}' into linear memory at 1024
                    (i32.store8 (i32.const 1024) (i32.const 123))
                    (i32.store8 (i32.const 1025) (i32.const 34))
                    (i32.store8 (i32.const 1026) (i32.const 97))
                    (i32.store8 (i32.const 1027) (i32.const 110))
                    (i32.store8 (i32.const 1028) (i32.const 111))
                    (i32.store8 (i32.const 1029) (i32.const 110))
                    (i32.store8 (i32.const 1030) (i32.const 121))
                    (i32.store8 (i32.const 1031) (i32.const 109))
                    (i32.store8 (i32.const 1032) (i32.const 105))
                    (i32.store8 (i32.const 1033) (i32.const 122))
                    (i32.store8 (i32.const 1034) (i32.const 101))
                    (i32.store8 (i32.const 1035) (i32.const 100))
                    (i32.store8 (i32.const 1036) (i32.const 34))
                    (i32.store8 (i32.const 1037) (i32.const 58))
                    (i32.store8 (i32.const 1038) (i32.const 116))
                    (i32.store8 (i32.const 1039) (i32.const 114))
                    (i32.store8 (i32.const 1040) (i32.const 117))
                    (i32.store8 (i32.const 1041) (i32.const 101))
                    (i32.store8 (i32.const 1042) (i32.const 125))
                    (i32.const 1024)
                    (i32.const 19)
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat_src).expect("Failed to parse WAT");
        let genome_id = 1001u64;
        substrate.put_genome(genome_id, &wasm_bytes).unwrap();

        // 3. Execute CDQL query with in-process WASM transform
        let query = "find id(\"101\") -> wasm(id: 1001, func: \"anonymize\")";
        let ast = parse(query).unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);

        let result = execute_plan(&plan, &ctx).expect("WASM query plan execution failed");
        assert_eq!(result.count, 1);
        assert_eq!(result.records[0]["anonymized"], true);
    }

    #[test]
    fn test_execute_time_window_pipeline() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 1;

        let now = 1_700_000_000_000u64;
        let e1 = serde_json::json!({ "id": 1, "cpu": 85, "timestamp_ms": now - 60_000 });
        let e2 = serde_json::json!({ "id": 2, "cpu": 92, "timestamp_ms": now });
        let e3 = serde_json::json!({ "id": 3, "cpu": 40, "timestamp_ms": now - 36_000_000 });

        substrate.put_record(col_id, 1, &serde_json::to_vec(&e1).unwrap()).unwrap();
        substrate.put_record(col_id, 2, &serde_json::to_vec(&e2).unwrap()).unwrap();
        substrate.put_record(col_id, 3, &serde_json::to_vec(&e3).unwrap()).unwrap();

        let query = "find * -> time_window(\"5m\")";
        let ast = parse(query).unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);

        let result = execute_plan(&plan, &ctx).expect("Time window execution failed");
        assert_eq!(result.count, 2, "Expected 2 events within 5m time window");
        let ids: Vec<i64> = result.records.iter().map(|r| r["id"].as_i64().unwrap()).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn test_execute_aggregate_and_group_by() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 1;

        for i in 1..=4 {
            let data = serde_json::json!({
                "id": i,
                "score": i * 10,
                "team": if i <= 2 { "alpha" } else { "beta" }
            });
            substrate.put_record(col_id, i as u128, &serde_json::to_vec(&data).unwrap()).unwrap();
        }

        let query_agg = "find * -> aggregate(count(), sum(\"score\"), avg(\"score\"))";
        let ast = parse(query_agg).unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);

        let result = execute_plan(&plan, &ctx).expect("Aggregate execution failed");
        assert_eq!(result.count, 1);
        let agg = &result.records[0];
        assert_eq!(agg["count"], 4);
        assert_eq!(agg["sum_score"], 100.0);
        assert_eq!(agg["avg_score"], 25.0);
    }

    #[test]
    fn test_vector_persistence_and_rehydration() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 99;

        // Insert records and durable vectors into substrate
        substrate.put_record(col_id, 1, b"{\"text\": \"machine learning\"}").unwrap();
        substrate.put_vector(col_id, 1, &[1.0, 0.0, 0.0]).unwrap();

        substrate.put_record(col_id, 2, b"{\"text\": \"astronomy\"}").unwrap();
        substrate.put_vector(col_id, 2, &[0.0, 1.0, 0.0]).unwrap();

        // 1. Query without in-memory index -> uses durable vector fallback in Substrate
        let query = "find * -> vector_knn([0.9, 0.1, 0.0], 5)";
        let ast = parse(query).unwrap();
        let plan = build_plan(&ast).unwrap();
        let ctx = QueryContext::new(&substrate, col_id);
        let result = execute_plan(&plan, &ctx).expect("Vector fallback failed");
        assert_eq!(result.count, 2);
        assert_eq!(result.records[0]["text"], "machine learning");

        // 2. Rehydrate an HNSW index from substrate
        let hnsw = Arc::new(HnswIndex::<CosineDistance>::new());
        let count = rehydrate_hnsw_from_substrate(&substrate, col_id, &hnsw).unwrap();
        assert_eq!(count, 2);

        // 3. Query with rehydrated in-memory index
        let ctx_with_hnsw = QueryContext::with_vector_index(&substrate, col_id, &hnsw);
        let result_hnsw = execute_plan(&plan, &ctx_with_hnsw).expect("HNSW query failed");
        assert_eq!(result_hnsw.count, 2);
        assert_eq!(result_hnsw.records[0]["text"], "machine learning");
    }

    #[test]
    fn test_execute_insert_and_index_accelerated_scan() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 7;
        let ctx = QueryContext::new(&substrate, col_id);

        // 1. Insert via CDQL
        let insert_q = "insert into Agent(id: 101, name: \"CluaizCore\", role: \"orchestrator\")";
        let ast = parse(insert_q).unwrap();
        let plan = build_plan(&ast).unwrap();
        let res = execute_plan(&plan, &ctx).expect("Insert execution failed");
        assert_eq!(res.count, 1);
        assert_eq!(res.records[0]["name"], "CluaizCore");

        // 2. Query via index-accelerated scan
        let find_q = "find Agent(role == \"orchestrator\")";
        let find_ast = parse(find_q).unwrap();
        let find_plan = build_plan(&find_ast).unwrap();
        let find_res = execute_plan(&find_plan, &ctx).expect("Find execution failed");
        assert_eq!(find_res.count, 1);
        assert_eq!(find_res.records[0]["id"].as_f64(), Some(101.0));
    }

    #[test]
    fn test_execute_shortest_path_pipeline() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let col_id = 8;
        let ctx = QueryContext::new(&substrate, col_id);

        // Put records 1, 2, 3
        substrate.put_record(col_id, 1, b"{\"node\": 1}").unwrap();
        substrate.put_record(col_id, 2, b"{\"node\": 2}").unwrap();
        substrate.put_record(col_id, 3, b"{\"node\": 3}").unwrap();

        // Edge 1 -> 2, 2 -> 3
        let meta = cluaizdb_substrate::EdgeMetadata { weight: 1.0, created_at: 0, valid_until: u64::MAX };
        substrate.add_edge(1, 1, 2, meta).unwrap();
        substrate.add_edge(2, 1, 3, meta).unwrap();

        // Query shortest path from 1 to 3
        let query = "find id(\"1\") -> shortest_path(\"3\")";
        let ast = parse(query).unwrap();
        let plan = build_plan(&ast).unwrap();
        let res = execute_plan(&plan, &ctx).expect("Shortest path failed");
        assert_eq!(res.count, 3);
        assert_eq!(res.records[0]["node"], 1);
        assert_eq!(res.records[1]["node"], 2);
        assert_eq!(res.records[2]["node"], 3);
    }

    #[test]
    fn test_collection_registry_cdql_isolation() {
        let dir = tempdir().unwrap();
        let substrate = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let ctx = QueryContext::new(&substrate, 999);

        // 1. Insert into "agents" collection
        let q1 = "insert into agents(id: 1, name: \"AgentAlpha\")";
        let plan1 = build_plan(&parse(q1).unwrap()).unwrap();
        let res1 = execute_plan(&plan1, &ctx).unwrap();
        assert_eq!(res1.count, 1);

        // 2. Insert into "memories" collection
        let q2 = "insert into memories(id: 2, note: \"InitialMemory\")";
        let plan2 = build_plan(&parse(q2).unwrap()).unwrap();
        let res2 = execute_plan(&plan2, &ctx).unwrap();
        assert_eq!(res2.count, 1);

        // 3. Find from agents
        let q3 = "find agents(name == \"AgentAlpha\")";
        let plan3 = build_plan(&parse(q3).unwrap()).unwrap();
        let res_agents = execute_plan(&plan3, &ctx).unwrap();
        assert_eq!(res_agents.count, 1);
        assert_eq!(res_agents.records[0]["name"], "AgentAlpha");

        // 4. Find from memories
        let q4 = "find memories(note == \"InitialMemory\")";
        let plan4 = build_plan(&parse(q4).unwrap()).unwrap();
        let res_mem = execute_plan(&plan4, &ctx).unwrap();
        assert_eq!(res_mem.count, 1);
        assert_eq!(res_mem.records[0]["note"], "InitialMemory");

        // 5. Cross isolation verification: agents collection must NOT have memories
        let q5 = "find agents(note == \"InitialMemory\")";
        let plan5 = build_plan(&parse(q5).unwrap()).unwrap();
        let res_empty = execute_plan(&plan5, &ctx).unwrap();
        assert_eq!(res_empty.count, 0);

        // 6. Substrate registry verification
        let cols = substrate.list_collections().unwrap();
        assert_eq!(cols.len(), 2);
        assert!(cols.contains(&("agents".to_string(), 1)) || cols.contains(&("agents".to_string(), 2)));
        assert!(cols.contains(&("memories".to_string(), 1)) || cols.contains(&("memories".to_string(), 2)));
    }
}

