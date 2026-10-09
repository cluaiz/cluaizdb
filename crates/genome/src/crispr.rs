use rhai::Engine;
use cluaizdb_errors::StorageError;
use cluaizdb_types::NeuronDna;

/// The CRISPR Sandbox ensures that injected DNA (Rhai scripts) is syntactically
/// valid and safe before being committed to the database genome.
pub struct CrisprSandbox {
    engine: Engine,
}

impl Default for CrisprSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl CrisprSandbox {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    /// Validates all provided DNA sequences in the 4-hook architecture.
    pub fn validate_dna(&self, dna: &NeuronDna) -> Result<(), StorageError> {
        if dna.engine != "rhai" {
            return Ok(());
        }

        let mut sequences_to_test = Vec::new();
        if let Some(s) = &dna.on_write { sequences_to_test.push(("on_write", s)); }
        if let Some(s) = &dna.on_read { sequences_to_test.push(("on_read", s)); }
        if let Some(s) = &dna.on_index { sequences_to_test.push(("on_index", s)); }
        if let Some(s) = &dna.on_lifecycle { sequences_to_test.push(("on_lifecycle", s)); }
        if let Some(s) = &dna.on_path_step { sequences_to_test.push(("on_path_step", s)); }
        if let Some(s) = &dna.on_path_resolve { sequences_to_test.push(("on_path_resolve", s)); }

        for (hook_name, sequence) in sequences_to_test {
            if let Err(err) = self.engine.compile(sequence) {
                tracing::warn!("CRISPR Sandbox rejected DNA at hook '{}': {}", hook_name, err);
                return Err(StorageError::DnaValidationFailed(format!("DNA Validation Failed at {}: {}", hook_name, err)));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dna_passes() {
        let sandbox = CrisprSandbox::new();
        let dna = NeuronDna {
            on_write: Some("let x = 1;".to_string()),
            on_read: None,
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
        assert!(sandbox.validate_dna(&dna).is_ok());
    }

    #[test]
    fn test_invalid_dna_rejected() {
        let sandbox = CrisprSandbox::new();
        let dna = NeuronDna {
            on_write: None,
            on_read: None,
            on_index: None,
            on_traverse: None,
            on_dream: None,
            on_lifecycle: Some("let res = # { missing_semicolon }".to_string()),
            on_path_step: None,
            on_path_resolve: None,
            wasm_module: None,
            wasm_module_path: None,
            parameters: serde_json::json!({}),
            engine: "rhai".to_string(),
        };
        assert!(sandbox.validate_dna(&dna).is_err());
    }
}
