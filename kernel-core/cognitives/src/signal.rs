//! Intent signals and preflight evaluation response contracts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime intent signal presented prior to inference or tool action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// The high-level intent or candidate prompt string
    pub current_intent: String,
    /// Raw serialized payload (e.g., proposed tool parameters or record bytes)
    pub payload: Vec<u8>,
    /// Environmental metadata tags (e.g., user_id, session_id, capability_tier)
    pub metadata: HashMap<String, String>,
}

impl Signal {
    pub fn new(intent: impl Into<String>) -> Self {
        Self {
            current_intent: intent.into(),
            payload: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), val.into());
        self
    }
}

/// Dynamic outcome returned by the ActionInterceptor back to the inference runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostEvaluationResult {
    /// Proceed to execution path without modification (policy passed, no memory injection needed)
    Permit,
    /// Stop execution path and surface policy breach reason back to caller
    Reject { reason: String },
    /// Intercept runtime and push a micro-footprint compact context packet (< 50 tokens)
    Intervene { context: String },
}
