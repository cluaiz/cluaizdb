//! `cluaizd-common`
//!
//! Shared traits, utility functions, and constants used across
//! all crates in the Cluaizd CLUAIZD workspace.

/// Validates that a vector contains only finite values.
/// Returns the index of the first invalid value, or `Ok(())` if valid.
pub fn validate_vector(vector: &[f32]) -> Result<(), usize> {
    for (i, val) in vector.iter().enumerate() {
        if !val.is_finite() {
            return Err(i);
        }
    }
    Ok(())
}

/// Maximum recommended adjacency edges per neuron before graph
/// memory bloat becomes a concern.
pub const MAX_ADJACENCY_EDGES: usize = 512;

/// The exact number of dimensions in the hardware footprint vector.
pub const VECTOR_DIMENSIONS: usize = 16;

/// Maximum size of a single raw_payload in bytes (1 GB).
/// Payloads exceeding this should be chunked into multiple neurons.
pub const MAX_PAYLOAD_SIZE_BYTES: usize = 1_073_741_824;
