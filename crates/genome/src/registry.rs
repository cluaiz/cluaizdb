use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use cluaizdb_types::NeuronDna;
use cluaizdb_errors::StorageError;
use crate::crispr::CrisprSandbox;

/// The Corporate Genome Manager.
/// Stores predefined or user-uploaded DNA sequences.
#[derive(Clone)]
pub struct GenomeRegistry {
    templates: Arc<RwLock<HashMap<String, NeuronDna>>>,
    sandbox: Arc<CrisprSandbox>,
}

impl Default for GenomeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GenomeRegistry {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
            sandbox: Arc::new(CrisprSandbox::new()),
        }
    }

    /// Registers a new DNA sequence under a given name (e.g. `preset=my_custom_db`).
    /// The DNA is passed through the CRISPR Sandbox before being saved.
    pub fn register_dna(&self, name: &str, dna: NeuronDna) -> Result<(), StorageError> {
        // 1. Validation Gate (Sandbox)
        self.sandbox.validate_dna(&dna)?;
        
        // 2. Commit to Registry
        let mut map = self.templates.write().unwrap();
        map.insert(name.to_string(), dna);
        
        tracing::info!("Registered new Corporate Genome: {}", name);
        Ok(())
    }

    /// Fetches a DNA sequence by its name.
    pub fn get_dna(&self, name: &str) -> Option<NeuronDna> {
        let map = self.templates.read().unwrap();
        map.get(name).cloned()
    }

    /// Dynamically loads DNA sequences from a directory of JSON files.
    pub fn load_from_directory(&self, path: &str) -> Result<(), StorageError> {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(mut dna) = serde_json::from_str::<NeuronDna>(&contents) {
                            if let Some(ref wasm_path) = dna.wasm_module_path {
                                // Resolve path relative to the json directory
                                let mut resolved_path = std::path::PathBuf::from(path.parent().unwrap());
                                resolved_path.push(wasm_path);
                                if let Ok(wasm_bytes) = std::fs::read(&resolved_path) {
                                    dna.wasm_module = Some(wasm_bytes);
                                } else {
                                    tracing::warn!("WASM file not found at {:?} for DNA {}", resolved_path, name);
                                }
                            }
                            if let Err(e) = self.register_dna(name, dna) {
                                tracing::error!("Failed to register DNA from {}: {}", path.display(), e);
                            }
                        } else {
                            tracing::error!("Failed to parse DNA JSON: {}", path.display());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
