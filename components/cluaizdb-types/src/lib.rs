//! `cluaizdb-types`
//!
//! Core data types for the Cluaizdb Nervous System Database.
//! All other crates in this workspace depend on this crate.
//!
//! Public API surface is intentionally flat — import everything from here.

mod neuron;
mod neuron_id;
mod payload_type;
pub mod distance;

// Flat public re-exports
pub use neuron::{NeuronDna, StorageTier, UniversalNeuron, NeuronEdge};
pub use neuron_id::NeuronId;
pub use payload_type::PayloadType;
pub use distance::{cosine_similarity, euclidean_distance, dot_product, magnitude};
