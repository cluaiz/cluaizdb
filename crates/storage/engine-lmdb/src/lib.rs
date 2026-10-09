//! `engine-lmdb`
//!
//! Physical LMDB-backed storage engine for Cluaiz CLUAIZD.
//!
//! ## Architecture
//! - `env` — Opens and manages the raw LMDB environment (file handle + mmap).
//! - `writer` — Serializes and writes `UniversalNeuron` structs to disk.
//! - `reader` — Deserializes and reads neurons, with model hash validation.
//! - `sensory_shard` — Isolated ring-buffer shard for Robotics/BCI streaming data.

pub mod env;
pub mod reader;
pub mod sensory_shard;
pub mod writer;
pub mod gc;
pub mod sandbox;
pub mod manifest;
pub mod ffi;
pub mod codecs;
pub mod specular;

// Flat public API surface
pub use codecs::*;
pub use env::LmdbEnv;
pub use reader::{read_neuron, iter_all_neurons, scan_neurons_by_type};
pub use sensory_shard::SensoryShard;
pub use writer::write_neuron;
pub use gc::{spawn_biological_gc, run_gc_sweep};
pub use sandbox::DeepArcherSandbox;
pub use manifest::{GeneRegistryManifest, GeneTrait, ManifestError};
pub use specular::SpecularGraph;

pub mod transit;
pub mod group_commit;

pub use group_commit::*;
 