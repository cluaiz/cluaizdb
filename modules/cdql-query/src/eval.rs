//! CDQL Search Evaluation Functions
//!
//! Provides pure, stateless evaluation helpers for:
//! - Full-Text / Inverted Search  (`eval_full_text`)
//! - Range Scan                   (`eval_range`)
//! - Geo-Spatial Proximity        (`eval_geo_near`)

use super::parser::CdqlValue;

/// Score how well `payload` matches `query` tokens.
pub fn eval_full_text(payload: &str, query: &str, fuzzy: bool) -> f32 {
    if query.is_empty() || payload.is_empty() {
        return 0.0;
    }

    let searchable = extract_text_from_json(payload);
    let query_tokens: Vec<&str> = query.split_whitespace().collect();
    let payload_words: Vec<&str> = searchable.split_whitespace().collect();

    let mut matched = 0usize;

    for q_tok in &query_tokens {
        let q_lower = q_tok.to_lowercase();
        let found = payload_words.iter().any(|w| {
            let w_lower = w.to_lowercase();
            if fuzzy {
                w_lower.contains(q_lower.as_str())
            } else {
                w_lower == q_lower.as_str()
            }
        });
        if found {
            matched += 1;
        }
    }

    if matched == 0 {
        return 0.0;
    }

    matched as f32 / query_tokens.len() as f32
}

/// Check whether a JSON document's `field` falls within `[start, end]`.
pub fn eval_range(payload: &str, field: &str, start: &CdqlValue, end: &CdqlValue) -> bool {
    let json: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let field_val = match json.get(field) {
        Some(v) => v,
        None => return false,
    };

    match (start, end) {
        (CdqlValue::Number(lo), CdqlValue::Number(hi)) => {
            if let Some(num) = field_val.as_f64() {
                num >= *lo && num <= *hi
            } else {
                false
            }
        }
        (CdqlValue::Text(lo), CdqlValue::Text(hi)) => {
            if let Some(s) = field_val.as_str() {
                s >= lo.as_str() && s <= hi.as_str()
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check whether the document's stored coordinates are within `radius_km` of `(target_lat, target_lon)`.
pub fn eval_geo_near(
    payload: &str,
    target_lat: f64,
    target_lon: f64,
    radius_km: f64,
) -> Option<f32> {
    let json: serde_json::Value = serde_json::from_str(payload).ok()?;

    let neuron_lat = probe_f64(&json, &["lat", "latitude"])?;
    let neuron_lon = probe_f64(&json, &["lon", "longitude"])?;

    let dist_km = haversine_km(neuron_lat, neuron_lon, target_lat, target_lon);

    if dist_km <= radius_km {
        let score = 1.0_f32 / (1.0_f32 + dist_km as f32);
        Some(score)
    } else {
        None
    }
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

fn probe_f64(json: &serde_json::Value, candidates: &[&str]) -> Option<f64> {
    for key in candidates {
        if let Some(v) = json.get(*key).and_then(|v| v.as_f64()) {
            return Some(v);
        }
    }
    None
}

fn extract_text_from_json(payload: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(json) => collect_strings(&json),
        Err(_) => payload.to_string(),
    }
}

fn collect_strings(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => map
            .values()
            .map(collect_strings)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(collect_strings)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fulltext_exact_match() {
        let payload = r#"{"name": "database engine"}"#;
        let score = eval_full_text(payload, "database engine", false);
        assert!(score > 0.0, "exact match must score > 0");
    }

    #[test]
    fn test_fulltext_no_match() {
        let payload = r#"{"name": "robotics stream"}"#;
        let score = eval_full_text(payload, "quantum entanglement", false);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_fulltext_fuzzy() {
        let payload = r#"{"description": "distributed database kernel"}"#;
        let score = eval_full_text(payload, "data", true);
        assert!(score > 0.0, "fuzzy should match 'database' with 'data'");
    }

    #[test]
    fn test_range_numeric_inside() {
        let payload = r#"{"age": 25}"#;
        let result = eval_range(payload, "age", &CdqlValue::Number(20.0), &CdqlValue::Number(30.0));
        assert!(result);
    }

    #[test]
    fn test_range_numeric_outside() {
        let payload = r#"{"age": 50}"#;
        let result = eval_range(payload, "age", &CdqlValue::Number(20.0), &CdqlValue::Number(30.0));
        assert!(!result);
    }

    #[test]
    fn test_geo_near_delhi_within_50km() {
        let payload = r#"{"lat": 28.4595, "lon": 77.0266, "name": "Gurgaon"}"#;
        let result = eval_geo_near(payload, 28.6139, 77.2090, 50.0);
        assert!(result.is_some(), "Gurgaon should be within 50km of Delhi");
    }

    #[test]
    fn test_geo_near_mumbai_outside_delhi_radius() {
        let payload = r#"{"lat": 19.0760, "lon": 72.8777, "name": "Mumbai"}"#;
        let result = eval_geo_near(payload, 28.6139, 77.2090, 50.0);
        assert!(result.is_none(), "Mumbai should be outside 50km of Delhi");
    }
}
