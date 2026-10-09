use serde::{Deserialize, Serialize};

/// Configuration governing execution constraints within the WASM sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum instructions allowed per execution invocation (default: 50,000).
    pub fuel_limit: u64,
    /// Maximum linear memory pages allowed (1 page = 64 KiB; 256 pages = 16 MiB).
    pub max_memory_pages: u32,
    /// Whether instruction fuel accounting is enforced.
    pub enable_fuel: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            fuel_limit: 50_000,
            max_memory_pages: 256,
            enable_fuel: true,
        }
    }
}

impl SandboxConfig {
    pub fn new(fuel_limit: u64, max_memory_pages: u32) -> Self {
        Self {
            fuel_limit,
            max_memory_pages,
            enable_fuel: true,
        }
    }
}
