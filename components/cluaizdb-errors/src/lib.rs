//! `cluaizdb-errors`
//!
//! Centralized error definitions for all crates in the Cluaizdb CLUAIZD workspace.
//! Every crate uses these types for consistent, structured error propagation.
//! Never use `anyhow` inside library crates — only at the binary layer.

mod storage_error;
mod vector_error;

pub use storage_error::StorageError;
pub use vector_error::VectorError;
