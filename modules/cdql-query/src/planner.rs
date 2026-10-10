/// CDQL Query Planner
///
/// Converts a `CdqlQuery` AST into an ordered `QueryPlan`.

use super::parser::{AggFunc, CdqlOp, CdqlQuery, CdqlValue, CompareOp, Filter, JoinType};

/// A resolved, executable step in the query execution pipeline
#[derive(Debug, Clone)]
pub enum PlanStep {
    // 1. FAST PATH (Key-Value)
    FastPathIdLookup {
        id: String,
    },

    // 2. CORE / DOCUMENT
    InsertData {
        label: String,
        data: std::collections::HashMap<String, CdqlValue>,
    },
    ScanAll {
        label_filter: Option<String>,
        filters: Vec<Filter>,
    },
    FilterResults {
        field: String,
        op: CompareOp,
        value: CdqlValue,
    },
    Unwind {
        field: String,
    },
    Project {
        keep: Vec<String>,
    },

    // 3. GRAPH
    GraphTraverse {
        edge: String,
        min_hops: usize,
        max_hops: usize,
        min_weight: f64,
    },
    ShortestPath {
        to_node: String,
    },

    // 4. RELATIONAL SQL
    RelationalJoin {
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

    // 5. TIME-SERIES
    TimeWindow {
        size: String,
    },

    // 6. VECTOR / AI
    VectorScan {
        vector: Vec<f32>,
        metric: String,
    },

    // 7. FULL TEXT SEARCH
    FullTextSearch {
        query: String,
        fuzzy: bool,
    },

    // 8. GEO-SPATIAL
    GeoNear {
        lat: f64,
        lon: f64,
        radius_km: f64,
    },

    // 9. WIDE-COLUMN
    RangeScan {
        field: String,
        start: CdqlValue,
        end: CdqlValue,
    },

    // 10. BLOB / OBJECT
    ByteStream {
        start_byte: usize,
        end_byte: usize,
    },

    // UTILITY
    Limit(usize),
    SortBy {
        field: String,
        ascending: bool,
    },
}

/// The full execution plan for a CDQL query
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub steps: Vec<PlanStep>,
    pub is_fast_path: bool,
    pub limit: usize,
}

impl QueryPlan {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            is_fast_path: false,
            limit: 100,
        }
    }
}

impl Default for QueryPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an executable `QueryPlan` from a parsed `CdqlQuery`
pub fn build_plan(query: &CdqlQuery) -> Result<QueryPlan, String> {
    let mut plan = QueryPlan::new();

    for op in &query.ops {
        match op {
            CdqlOp::Insert { label, data } => {
                plan.steps.push(PlanStep::InsertData {
                    label: label.clone(),
                    data: data.clone(),
                });
            }
            CdqlOp::FindById { id } => {
                plan.is_fast_path = true;
                plan.steps.push(PlanStep::FastPathIdLookup { id: id.clone() });
            }
            CdqlOp::Find { label, filters } => {
                let label_filter = if label == "*" { None } else { Some(label.clone()) };
                plan.steps.push(PlanStep::ScanAll {
                    label_filter,
                    filters: filters.clone(),
                });
            }
            CdqlOp::Filter { field, op, value } => {
                plan.steps.push(PlanStep::FilterResults {
                    field: field.clone(),
                    op: op.clone(),
                    value: value.clone(),
                });
            }
            CdqlOp::Unwind { field } => plan.steps.push(PlanStep::Unwind { field: field.clone() }),
            CdqlOp::Project { keep } => plan.steps.push(PlanStep::Project { keep: keep.clone() }),
            CdqlOp::Traverse { edge, min_hops, max_hops, min_weight } => {
                plan.steps.push(PlanStep::GraphTraverse {
                    edge: edge.clone(),
                    min_hops: *min_hops,
                    max_hops: *max_hops,
                    min_weight: *min_weight,
                });
            }
            CdqlOp::ShortestPath { to_node } => plan.steps.push(PlanStep::ShortestPath { to_node: to_node.clone() }),
            CdqlOp::Join { target, on_left, on_right, join_type } => {
                plan.steps.push(PlanStep::RelationalJoin {
                    target: target.clone(),
                    on_left: on_left.clone(),
                    on_right: on_right.clone(),
                    join_type: join_type.clone(),
                });
            }
            CdqlOp::GroupBy { fields } => plan.steps.push(PlanStep::GroupBy { fields: fields.clone() }),
            CdqlOp::Aggregate { functions } => plan.steps.push(PlanStep::Aggregate { functions: functions.clone() }),
            CdqlOp::TimeWindow { size } => plan.steps.push(PlanStep::TimeWindow { size: size.clone() }),
            CdqlOp::SimilarTo { vector, metric } => plan.steps.push(PlanStep::VectorScan { vector: vector.clone(), metric: metric.clone() }),
            CdqlOp::Search { query, fuzzy } => plan.steps.push(PlanStep::FullTextSearch { query: query.clone(), fuzzy: *fuzzy }),
            CdqlOp::GeoNear { lat, lon, radius_km } => plan.steps.push(PlanStep::GeoNear { lat: *lat, lon: *lon, radius_km: *radius_km }),
            CdqlOp::RangeScan { field, start, end } => plan.steps.push(PlanStep::RangeScan { field: field.clone(), start: start.clone(), end: end.clone() }),
            CdqlOp::Stream { start_byte, end_byte } => plan.steps.push(PlanStep::ByteStream { start_byte: *start_byte, end_byte: *end_byte }),
            CdqlOp::Limit(n) => {
                plan.limit = *n;
                plan.steps.push(PlanStep::Limit(*n));
            }
            CdqlOp::SortBy { field, ascending } => {
                plan.steps.push(PlanStep::SortBy {
                    field: field.clone(),
                    ascending: *ascending,
                });
            }
        }
    }

    if plan.steps.is_empty() {
        return Err("CDQL plan has no executable steps".to_string());
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_plan_fast_path() {
        let q = parse(r#"find id("123")"#).unwrap();
        let plan = build_plan(&q).unwrap();
        assert!(plan.is_fast_path);
        assert!(matches!(plan.steps[0], PlanStep::FastPathIdLookup { .. }));
    }

    #[test]
    fn test_plan_graph_traversal() {
        let q = parse(r#"find User -> traverse(edge: "friends")"#).unwrap();
        let plan = build_plan(&q).unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(plan.steps[1], PlanStep::GraphTraverse { .. }));
    }

    #[test]
    fn test_plan_time_window() {
        let q = parse(r#"find * -> time_window(size: "5m")"#).unwrap();
        let plan = build_plan(&q).unwrap();
        assert!(matches!(plan.steps[1], PlanStep::TimeWindow { .. }));
    }
}
