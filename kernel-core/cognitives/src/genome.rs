//! Declarative Genome templates, composable DNA modules, and Wasmtime runtime sandboxing.

use crate::error::{CognitiveError, Result};
use crate::signal::Signal;
use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};
use std::collections::HashMap;

pub type GenomeId = u64;

/// Low-latency programmatic interception bounds inside the CluaizDB engine environment.
pub trait DnaModule: Send + Sync {
    /// Static identifier for this DNA module (e.g., "dna.core.policy")
    fn uuid(&self) -> &'static str;

    /// Preflight gate invoked before persisting a record to underlying substrate.
    fn process_on_write(&self, key: &[u8], payload: &[u8]) -> Result<()> {
        let _ = (key, payload);
        Ok(())
    }

    /// Preflight evaluation invoked when an agent intent or tool action is proposed.
    /// Returns `Some(fact)` if memory context should be injected, or `None` if silent.
    fn evaluate_runtime_signal(&self, signal: &Signal) -> Result<Option<String>> {
        let _ = signal;
        Ok(None)
    }
}

/// Static pre-compiled native security rule targeting invariants (e.g. data leak prevention).
#[derive(Debug, Clone, Default)]
pub struct PolicyDna {
    pub blocked_keywords: Vec<String>,
}

impl PolicyDna {
    pub fn new(blocked_keywords: Vec<String>) -> Self {
        Self { blocked_keywords }
    }
}

impl DnaModule for PolicyDna {
    fn uuid(&self) -> &'static str {
        "dna.core.system-policy"
    }

    fn process_on_write(&self, _key: &[u8], payload: &[u8]) -> Result<()> {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(payload) {
            if val.get("sensitive_identity_token").is_some() {
                return Err(CognitiveError::PolicyViolation(
                    "Security Block: Sensitive identity credentials cannot be stored in unmasked collections.".into(),
                ));
            }
        }
        Ok(())
    }

    fn evaluate_runtime_signal(&self, signal: &Signal) -> Result<Option<String>> {
        for keyword in &self.blocked_keywords {
            if signal.current_intent.contains(keyword) {
                return Err(CognitiveError::PolicyViolation(format!(
                    "Governance Violation: Action contains blocked policy term '{}'",
                    keyword
                )));
            }
        }
        if signal.current_intent.contains("unsafe_override_keys") {
            return Err(CognitiveError::PolicyViolation(
                "Governance Block: Intent attempts unauthorized structural keys override.".into(),
            ));
        }
        Ok(None)
    }
}

/// Associative preference DNA module providing fast domain memory rules.
#[derive(Debug, Clone, Default)]
pub struct PreferenceDna {
    pub keyword_associations: HashMap<String, String>,
}

impl PreferenceDna {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_association(mut self, keyword: impl Into<String>, memory: impl Into<String>) -> Self {
        self.keyword_associations.insert(keyword.into(), memory.into());
        self
    }
}

impl DnaModule for PreferenceDna {
    fn uuid(&self) -> &'static str {
        "dna.domain.preference"
    }

    fn evaluate_runtime_signal(&self, signal: &Signal) -> Result<Option<String>> {
        let intent_lower = signal.current_intent.to_lowercase();
        for (kw, memory) in &self.keyword_associations {
            if intent_lower.contains(&kw.to_lowercase()) {
                return Ok(Some(memory.clone()));
            }
        }
        Ok(None)
    }
}

/// Fuel-metered sandboxed WASM DNA module using the warm WasmGenomeRunner.
pub struct WarmWasmDna {
    runner: WasmGenomeRunner,
    module: wasmtime::Module,
}

impl WarmWasmDna {
    pub fn compile(bytecode: &[u8], fuel_limit: u64) -> Result<Self> {
        let config = SandboxConfig::new(fuel_limit, 1);
        let runner = WasmGenomeRunner::new(config)?;
        let module = runner.compile(bytecode)?;
        Ok(Self { runner, module })
    }
}

impl DnaModule for WarmWasmDna {
    fn uuid(&self) -> &'static str {
        "dna.sandbox.wasm"
    }

    fn process_on_write(&self, _key: &[u8], payload: &[u8]) -> Result<()> {
        let val = wasmtime::Val::I32(payload.len() as i32);
        // Call validation entrypoint if present
        let result = self.runner.call_scalar(&self.module, "validate_len", &[val])?;
        if let Some(code) = result.first().and_then(|v| v.i32()) {
            if code != 0 {
                return Err(CognitiveError::PolicyViolation(format!(
                    "WASM sandbox validation rejected write with error code {}",
                    code
                )));
            }
        }
        Ok(())
    }
}

/// Core installable template governing systemic memory rules and verification paths.
pub struct AgentGenome {
    pub identity: GenomeId,
    pub components: Vec<Box<dyn DnaModule>>,
    pub is_production_strict: bool,
}

impl AgentGenome {
    pub fn new(id: GenomeId) -> Self {
        Self {
            identity: id,
            components: Vec::new(),
            is_production_strict: true,
        }
    }

    pub fn plug_dna(&mut self, module: Box<dyn DnaModule>) {
        self.components.push(module);
    }

    pub fn dispatch_preflight_write(&self, key: &[u8], data: &[u8]) -> Result<()> {
        for component in &self.components {
            component.process_on_write(key, data)?;
        }
        Ok(())
    }

    pub fn execute_evaluations(&self, signal: &Signal) -> Result<Vec<String>> {
        let mut extracted_facts = Vec::new();
        for component in &self.components {
            if let Some(fact) = component.evaluate_runtime_signal(signal)? {
                extracted_facts.push(fact);
            }
        }
        Ok(extracted_facts)
    }
}
