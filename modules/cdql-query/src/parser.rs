use serde::{Deserialize, Serialize};

/// A single value in a CDQL expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CdqlValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Vector(Vec<f32>),
    Parameter, // Represents a placeholder (?) mapped to a binary binding
    Null,
}

/// Comparison operators for filters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,        // =
    NotEq,     // !=
    Gt,        // >
    Lt,        // <
    Gte,       // >=
    Lte,       // <=
    Contains,  // contains
}

/// A single key-value filter condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub op: CompareOp,
    pub value: CdqlValue,
}

/// Join types for Relational queries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// Aggregation functions for Time-Series and SQL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggFunc {
    Count,
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
}

/// Each "step" in a CDQL pipeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CdqlOp {
    Insert {
        label: String,
        data: std::collections::HashMap<String, CdqlValue>,
    },
    Find {
        label: String,
        filters: Vec<Filter>,
    },
    FindById {
        id: String,
    },
    Filter {
        field: String,
        op: CompareOp,
        value: CdqlValue,
    },
    Project {
        keep: Vec<String>,
    },
    Unwind {
        field: String,
    },
    Traverse {
        edge: String,
        min_hops: usize,
        max_hops: usize,
        min_weight: f64,
    },
    ShortestPath {
        to_node: String,
    },
    Join {
        target: String,
        on_left: String,
        on_right: String,
        join_type: JoinType,
    },
    GroupBy {
        fields: Vec<String>,
    },
    Aggregate {
        functions: Vec<AggFunc>,
    },
    TimeWindow {
        size: String,
    },
    SimilarTo {
        vector: Vec<f32>,
        metric: String,
    },
    Search {
        query: String,
        fuzzy: bool,
    },
    GeoNear {
        lat: f64,
        lon: f64,
        radius_km: f64,
    },
    RangeScan {
        field: String,
        start: CdqlValue,
        end: CdqlValue,
    },
    Stream {
        start_byte: usize,
        end_byte: usize,
    },
    Limit(usize),
    SortBy {
        field: String,
        ascending: bool,
    },
}

/// The full CDQL query — an ordered pipeline of operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CdqlQuery {
    pub ops: Vec<CdqlOp>,
}

/// Parse a CDQL string into a structured `CdqlQuery`.
pub fn parse(input: &str) -> Result<CdqlQuery, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty CDQL query".to_string());
    }

    let segments: Vec<&str> = input.split("->").map(|s| s.trim()).collect();
    if segments.is_empty() {
        return Err("No operations found".to_string());
    }

    let mut ops = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        let op = if i == 0 {
            parse_find(segment)?
        } else {
            parse_pipeline_op(segment)?
        };
        ops.push(op);
    }

    Ok(CdqlQuery { ops })
}

fn parse_find(segment: &str) -> Result<CdqlOp, String> {
    let segment = segment.trim();

    if segment.starts_with("insert ") {
        return parse_insert(segment);
    }

    if !segment.starts_with("find") {
        return Err(format!("Expected 'find' or 'insert' keyword, got: '{}'", segment));
    }

    let after_find = segment["find".len()..].trim();

    if after_find.starts_with("id(") {
        let id_str = after_find
            .strip_prefix("id(")
            .and_then(|s| s.strip_suffix(')'))
            .map(|s| s.trim_matches('"'))
            .ok_or("Malformed find id()")?;
        return Ok(CdqlOp::FindById { id: id_str.to_string() });
    }

    let (label, filter_str) = if let Some(paren_pos) = after_find.find('(') {
        let label = after_find[..paren_pos].trim().to_string();
        let rest = &after_find[paren_pos..];
        let close = rest.rfind(')').ok_or("Missing closing ')' in find")?;
        let filter_body = &rest[1..close];
        (label, filter_body.trim())
    } else {
        (after_find.to_string(), "")
    };

    let label = if label.is_empty() { "*".to_string() } else { label };
    let filters = if filter_str.is_empty() { vec![] } else { parse_filters(filter_str)? };

    Ok(CdqlOp::Find { label, filters })
}

fn parse_pipeline_op(segment: &str) -> Result<CdqlOp, String> {
    let segment = segment.trim();

    if segment.starts_with("get ") {
        let relation = segment["get ".len()..].trim().to_string();
        return Ok(CdqlOp::Traverse {
            edge: relation,
            min_hops: 1,
            max_hops: 1,
            min_weight: 0.0,
        });
    }

    if segment.starts_with("filter ") {
        let filter_body = &segment["filter ".len()..];
        let filters = parse_filters(filter_body)?;
        if let Some(f) = filters.into_iter().next() {
            return Ok(CdqlOp::Filter { field: f.field, op: f.op, value: f.value });
        }
    }

    if segment.starts_with("limit ") {
        let n_str = segment["limit ".len()..].trim();
        let n: usize = n_str.parse().unwrap_or(10);
        return Ok(CdqlOp::Limit(n));
    }

    if segment.starts_with("traverse(") {
        let body = segment.strip_prefix("traverse(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let edge = extract_named_string(body, "edge").unwrap_or_else(|| "*".to_string());
        let min_hops = extract_named_f64(body, "min_hops").map(|v| v as usize).unwrap_or(1);
        let max_hops = extract_named_f64(body, "max_hops").map(|v| v as usize).unwrap_or(3);
        let min_weight = extract_named_f64(body, "min_weight").unwrap_or(0.0);
        return Ok(CdqlOp::Traverse { edge, min_hops, max_hops, min_weight });
    }

    if segment.starts_with("shortest_path(") {
        let body = segment.strip_prefix("shortest_path(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let to_node = extract_named_string(body, "to").unwrap_or_default();
        return Ok(CdqlOp::ShortestPath { to_node });
    }

    if segment.starts_with("join(") {
        let body = segment.strip_prefix("join(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let target = extract_named_string(body, "target").unwrap_or_default();
        let on_left = extract_named_string(body, "on_left").unwrap_or_default();
        let on_right = extract_named_string(body, "on_right").unwrap_or_default();
        let join_type_str = extract_named_string(body, "type").unwrap_or_else(|| "inner".to_string());
        let join_type = match join_type_str.to_lowercase().as_str() {
            "left" => JoinType::Left,
            "right" => JoinType::Right,
            "full" => JoinType::Full,
            _ => JoinType::Inner,
        };
        return Ok(CdqlOp::Join { target, on_left, on_right, join_type });
    }

    if segment.starts_with("group_by(") {
        let body = segment.strip_prefix("group_by(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let fields: Vec<String> = body.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(CdqlOp::GroupBy { fields });
    }

    if segment.starts_with("aggregate(") {
        let body = segment.strip_prefix("aggregate(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let mut functions = Vec::new();
        let parts: Vec<&str> = body.split(',').collect();
        for part in parts {
            let p = part.trim();
            if p.starts_with("count()") {
                functions.push(AggFunc::Count);
            } else if p.starts_with("sum(") {
                let f = p.strip_prefix("sum(").and_then(|s| s.strip_suffix(')')).unwrap_or("").trim_matches('"').to_string();
                if !f.is_empty() { functions.push(AggFunc::Sum(f)); }
            } else if p.starts_with("avg(") {
                let f = p.strip_prefix("avg(").and_then(|s| s.strip_suffix(')')).unwrap_or("").trim_matches('"').to_string();
                if !f.is_empty() { functions.push(AggFunc::Avg(f)); }
            } else if p.starts_with("min(") {
                let f = p.strip_prefix("min(").and_then(|s| s.strip_suffix(')')).unwrap_or("").trim_matches('"').to_string();
                if !f.is_empty() { functions.push(AggFunc::Min(f)); }
            } else if p.starts_with("max(") {
                let f = p.strip_prefix("max(").and_then(|s| s.strip_suffix(')')).unwrap_or("").trim_matches('"').to_string();
                if !f.is_empty() { functions.push(AggFunc::Max(f)); }
            }
        }
        return Ok(CdqlOp::Aggregate { functions });
    }

    if segment.starts_with("time_window(") {
        let body = segment.strip_prefix("time_window(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let size = extract_named_string(body, "size").unwrap_or_else(|| "1h".to_string());
        return Ok(CdqlOp::TimeWindow { size });
    }

    if segment.starts_with("project(") {
        let body = segment.strip_prefix("project(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let keep: Vec<String> = body.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(CdqlOp::Project { keep });
    }

    if segment.starts_with("unwind(") {
        let body = segment.strip_prefix("unwind(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let field = body.trim_matches('"').to_string();
        return Ok(CdqlOp::Unwind { field });
    }

    if segment.starts_with("geo_near(") {
        let body = segment
            .strip_prefix("geo_near(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or("");
        let lat = extract_named_f64(body, "lat").unwrap_or(0.0);
        let lon = extract_named_f64(body, "lon").unwrap_or(0.0);
        let radius_km = extract_named_f64(body, "radius_km").unwrap_or(5.0);
        return Ok(CdqlOp::GeoNear { lat, lon, radius_km });
    }

    if segment.starts_with("search(") {
        let body = segment
            .strip_prefix("search(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or("");
        let query = extract_named_string(body, "query").unwrap_or_default();
        let fuzzy = extract_named_bool(body, "fuzzy").unwrap_or(false);
        return Ok(CdqlOp::Search { query, fuzzy });
    }

    if segment.starts_with("range(") {
        let body = segment
            .strip_prefix("range(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or("");
        let field = extract_named_string(body, "field").unwrap_or_default();
        let start = if let Some(n) = extract_named_f64(body, "start") {
            CdqlValue::Number(n)
        } else {
            CdqlValue::Text(extract_named_string(body, "start").unwrap_or_default())
        };
        let end = if let Some(n) = extract_named_f64(body, "end") {
            CdqlValue::Number(n)
        } else {
            CdqlValue::Text(extract_named_string(body, "end").unwrap_or_default())
        };
        return Ok(CdqlOp::RangeScan { field, start, end });
    }

    if segment.starts_with("stream(") {
        let body = segment.strip_prefix("stream(").and_then(|s| s.strip_suffix(')')).unwrap_or("");
        let start_byte = extract_named_f64(body, "start").map(|v| v as usize).unwrap_or(0);
        let end_byte = extract_named_f64(body, "end").map(|v| v as usize).unwrap_or(usize::MAX);
        return Ok(CdqlOp::Stream { start_byte, end_byte });
    }

    Err(format!("Unknown or unimplemented pipeline operation: '{}'", segment))
}

fn parse_filters(input: &str) -> Result<Vec<Filter>, String> {
    let input = input.trim();
    if input.is_empty() { return Ok(vec![]); }

    let mut filters = Vec::new();
    let parts: Vec<&str> = input.split(',').collect();

    for part in parts {
        if part.trim().is_empty() { continue; }
        filters.push(parse_single_filter(part.trim())?);
    }

    Ok(filters)
}

fn parse_single_filter(input: &str) -> Result<Filter, String> {
    let ops = [
        (">=", CompareOp::Gte),
        ("<=", CompareOp::Lte),
        ("!=", CompareOp::NotEq),
        (">", CompareOp::Gt),
        ("<", CompareOp::Lt),
        ("contains", CompareOp::Contains),
        (":", CompareOp::Eq),
        ("=", CompareOp::Eq),
    ];

    for (op_str, op) in &ops {
        if let Some(pos) = input.find(op_str) {
            let field = input[..pos].trim().to_string();
            let value_str = input[pos + op_str.len()..].trim();
            let value = parse_value(value_str)?;
            return Ok(Filter { field, op: op.clone(), value });
        }
    }
    Err(format!("Cannot parse filter: '{}'", input))
}

fn parse_value(input: &str) -> Result<CdqlValue, String> {
    let input = input.trim();
    if input == "?" { return Ok(CdqlValue::Parameter); }
    if input.starts_with('"') && input.ends_with('"') {
        return Ok(CdqlValue::Text(input[1..input.len() - 1].to_string()));
    }
    if input == "true" { return Ok(CdqlValue::Bool(true)); }
    if input == "false" { return Ok(CdqlValue::Bool(false)); }
    if input == "null" { return Ok(CdqlValue::Null); }
    if let Ok(n) = input.parse::<f64>() { return Ok(CdqlValue::Number(n)); }
    Ok(CdqlValue::Text(input.to_string()))
}

fn parse_insert(segment: &str) -> Result<CdqlOp, String> {
    let after_insert = segment["insert into".len()..].trim();
    let (label, data_str) = if let Some(paren_pos) = after_insert.find('(') {
        let label = after_insert[..paren_pos].trim().to_string();
        let rest = &after_insert[paren_pos..];
        let close = rest.rfind(')').ok_or("Missing closing ')' in insert")?;
        let body = &rest[1..close];
        (label, body.trim())
    } else {
        return Err("Malformed insert command. Expected: insert into Label(key: value)".to_string());
    };

    let mut data = std::collections::HashMap::new();
    let parts: Vec<&str> = data_str.split(',').collect();
    for part in parts {
        if part.trim().is_empty() { continue; }
        if let Some(colon) = part.find(':') {
            let key = part[..colon].trim().to_string();
            let val_str = part[colon + 1..].trim();
            data.insert(key, parse_value(val_str)?);
        }
    }

    Ok(CdqlOp::Insert { label, data })
}

fn extract_named_f64(body: &str, key: &str) -> Option<f64> {
    let search = format!("{}:", key);
    let pos = body.find(search.as_str())?;
    let after = body[pos + search.len()..].trim();
    let raw = after.split(',').next().unwrap_or("").trim();
    raw.parse::<f64>().ok()
}

fn extract_named_string(body: &str, key: &str) -> Option<String> {
    let search = format!("{}:", key);
    let pos = body.find(search.as_str())?;
    let after = body[pos + search.len()..].trim();
    if after.starts_with('"') {
        let inner = &after[1..];
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        let raw = after.split(',').next().unwrap_or("").trim();
        if raw.is_empty() { None } else { Some(raw.to_string()) }
    }
}

fn extract_named_bool(body: &str, key: &str) -> Option<bool> {
    let search = format!("{}:", key);
    let pos = body.find(search.as_str())?;
    let after = body[pos + search.len()..].trim();
    let raw = after.split(',').next().unwrap_or("").trim();
    match raw {
        "true"  => Some(true),
        "false" => Some(false),
        _       => None,
    }
}
