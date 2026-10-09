// ==============================================================================
// 🧱 CRUD Schema Validation (Auto-WASM Rust Edition)
// ==============================================================================

use cluaizdb_dna_sdk::prelude::*;
use serde_json::Value;

#[dna_hook(on_write)]
pub fn validate_schema(payload: &Payload, _metadata: &Metadata, config_json: &str) -> WriteDecision {
    // Note: The config_json here contains the actual JSON Schema rules 
    // parsed from the rules.json file attached to the node.
    let rules: Value = serde_json::from_str(config_json).unwrap();
    let data: Value = serde_json::from_str(&payload.as_text()).unwrap();

    // Iterate over required fields
    if let Some(required) = rules.get("required_fields").and_then(|r| r.as_array()) {
        for req in required {
            let field_name = req.get("name").unwrap().as_str().unwrap();
            let field_type = req.get("type").unwrap().as_str().unwrap();

            let field = data.get(field_name);
            
            if field.is_none() {
                return WriteDecision::Reject(format!("Missing required field: {}", field_name));
            }

            let field = field.unwrap();
            let is_valid = match field_type {
                "string" => field.is_string(),
                "integer" => field.is_i64(),
                "boolean" => field.is_boolean(),
                _ => true,
            };

            if !is_valid {
                return WriteDecision::Reject(format!("Invalid type for field: {}", field_name));
            }
        }
    }

    WriteDecision::Approve
}
