//! Direct dynamic generation preflight scanner capturing structured information prior to LLM/tool calls.

use crate::genome::AgentGenome;
use crate::signal::{PostEvaluationResult, Signal};
use cluaizdb_substrate::traits::StorageSubstrate;
use std::sync::Arc;

/// Compact context packet strictly bounded under 50 tokens (<= 190 characters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactContextPacket {
    pub tokens_serialized: String,
}

pub struct ContextSerializer;

impl ContextSerializer {
    /// Formulates targeted facts utilizing structural layouts, guaranteeing the output length stays minimal.
    pub fn compress_into_active_compact_state(facts: &[&str]) -> CompactContextPacket {
        let mut target_builder = String::with_capacity(192);
        target_builder.push_str("[SYSTEM MEMORY:");

        for (idx, item) in facts.iter().enumerate() {
            if idx > 0 {
                target_builder.push(';');
            }
            target_builder.push_str(item);
        }
        target_builder.push(']');

        // Dynamic hard bounds: Guarantee absolute length values match the token consumption contract parameters.
        if target_builder.len() > 190 {
            target_builder.truncate(186);
            target_builder.push_str("...]");
        }

        CompactContextPacket {
            tokens_serialized: target_builder,
        }
    }
}

/// Action and tool-call boundary interceptor enforcing policy gates and memory injection.
pub struct ActionInterceptor<S: StorageSubstrate> {
    pub db_layer: Arc<S>,
    pub running_policy: Arc<AgentGenome>,
}

impl<S: StorageSubstrate> ActionInterceptor<S> {
    pub fn new(db_layer: Arc<S>, running_policy: Arc<AgentGenome>) -> Self {
        Self {
            db_layer,
            running_policy,
        }
    }

    /// Evaluates structural preflight actions prior to actual LLM consumption or task executions.
    pub fn evaluate_and_route_intent(&self, signal: &Signal) -> PostEvaluationResult {
        match self.running_policy.execute_evaluations(signal) {
            Ok(facts) => {
                if !facts.is_empty() {
                    let fact_refs: Vec<&str> = facts.iter().map(|s| s.as_str()).collect();
                    let packet = ContextSerializer::compress_into_active_compact_state(&fact_refs);
                    PostEvaluationResult::Intervene {
                        context: packet.tokens_serialized,
                    }
                } else {
                    PostEvaluationResult::Permit
                }
            }
            Err(e) => PostEvaluationResult::Reject {
                reason: format!("Security Engine Refused Pre-execution Path: {}", e),
            },
        }
    }
}
