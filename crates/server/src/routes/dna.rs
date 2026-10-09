use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};
use cluaizdb_types::NeuronDna;
use crate::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct DnaSetupRequest {
    pub name: String,
    pub engine: String, // "cdql", "wasm", "auto-wasm", "rhai"
    pub code: String,   // Raw script, JSON, or Base64 bytes
}

#[derive(Debug, Serialize)]
pub struct DnaSetupResponse {
    pub status: String,
    pub message: String,
}

pub async fn handle_dna_setup(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DnaSetupRequest>,
) -> Result<Json<DnaSetupResponse>, (StatusCode, String)> {
    tracing::info!("Received DNA Setup Request for '{}' (Engine: {})", payload.name, payload.engine);

    match payload.engine.as_str() {
        "cdql" => {
            // Validate that the code is valid JSON representing CDQL
            if let Err(e) = serde_json::from_str::<serde_json::Value>(&payload.code) {
                return Err((StatusCode::BAD_REQUEST, format!("Invalid CDQL JSON: {}", e)));
            }

            let dna = NeuronDna {
                on_write: None,
                on_read: Some(payload.code), // CDQL rules stored here
                on_index: None,
                on_traverse: None,
                on_dream: None,
                on_lifecycle: None,
                on_path_step: None,
                on_path_resolve: None,
                wasm_module: None,
                wasm_module_path: None,
                parameters: serde_json::json!({}),
                engine: "cdql".to_string(),
            };
            
            state.genome_registry.register_dna(&payload.name, dna)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
            Ok(Json(DnaSetupResponse {
                status: "success".to_string(),
                message: format!("CDQL DNA '{}' registered successfully", payload.name),
            }))
        }
        "rhai" => {
            let dna = NeuronDna {
                on_write: None,
                on_read: Some(payload.code),
                on_index: None,
                on_traverse: None,
                on_dream: None,
                on_lifecycle: None,
                on_path_step: None,
                on_path_resolve: None,
                wasm_module: None,
                wasm_module_path: None,
                parameters: serde_json::json!({}),
                engine: "rhai".to_string(),
            };
            
            state.genome_registry.register_dna(&payload.name, dna)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
            Ok(Json(DnaSetupResponse {
                status: "success".to_string(),
                message: format!("Rhai DNA '{}' registered successfully", payload.name),
            }))
        }
        "wasm" => {
            // Decode base64 and write directly to active_dnas
            let decoded = general_purpose::STANDARD.decode(&payload.code)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid Base64 WASM: {}", e)))?;
                
            let active_dnas_dir = std::path::PathBuf::from("active_dnas");
            if !active_dnas_dir.exists() {
                std::fs::create_dir_all(&active_dnas_dir).unwrap_or_default();
            }
            
            let wasm_path = active_dnas_dir.join(format!("{}.wasm", payload.name));
            std::fs::write(&wasm_path, decoded)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save WASM: {}", e)))?;
                
            Ok(Json(DnaSetupResponse {
                status: "success".to_string(),
                message: format!("WASM DNA '{}' saved to disk. Hot-reload watcher will load it instantly.", payload.name),
            }))
        }
        "auto-wasm" => {
            // Use the AutoCompiler to build from Rust code
            // This blocks the async executor momentarily, but we could wrap in spawn_blocking if needed.
            let name = payload.name.clone();
            let code = payload.code.clone();
            
            match tokio::task::spawn_blocking(move || {
                genome::auto_compiler::compile_rust_to_wasm(&name, &code)
            }).await.unwrap() {
                Ok(_) => {
                    Ok(Json(DnaSetupResponse {
                        status: "success".to_string(),
                        message: format!("Auto-WASM DNA '{}' compiled successfully and hot-reloaded.", payload.name),
                    }))
                }
                Err(e) => {
                    Err((StatusCode::BAD_REQUEST, format!("Auto-WASM Compilation Error: {}", e)))
                }
            }
        }
        _ => {
            Err((StatusCode::BAD_REQUEST, format!("Unknown DNA Engine: {}", payload.engine)))
        }
    }
}
