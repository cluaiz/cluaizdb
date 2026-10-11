//! Error definitions for CluaizDB Cognitive Governance Subsystem.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CognitiveError>;

#[derive(Debug, Error)]
pub enum CognitiveError {
    #[error("Substrate storage failure: {0}")]
    Substrate(#[from] cluaizdb_substrate::error::SubstrateError),

    #[error("WASM sandbox execution defect: {0}")]
    WasmEvaluation(#[from] cluaizdb_wasm_sandbox::error::SandboxError),

    #[error("Security policy validation blocked: {0}")]
    PolicyViolation(String),

    #[error("Context packet exceeds size envelope (> 190 bytes): {0} bytes")]
    PacketOverflow(usize),

    #[error("Genome schema collision: {0}")]
    GenomeCollision(String),

    #[error("Asymmetric queue dispatch failed: {0}")]
    QueueDispatch(String),
}
