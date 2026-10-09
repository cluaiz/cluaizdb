/// All errors related to vector data incompatibility and validation.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// Returned when the model that generated the stored vector
    /// does not match the model making the query.
    /// This prevents meaningless vector comparisons across model boundaries.
    #[error(
        "Model hash mismatch: stored model={stored_hash:?}, querying model={query_hash:?}. \
         Re-embed the neuron with the current model before querying."
    )]
    ModelHashMismatch {
        stored_hash: [u8; 32],
        query_hash: [u8; 32],
    },

    /// Returned when a vector with wrong dimensions is submitted.
    /// CLUAIZD only accepts 16-dimensional vectors.
    #[error("Vector dimension mismatch: expected 16 dimensions, got {actual}")]
    DimensionMismatch { actual: usize },

    /// Returned when vector data contains NaN or Infinity values.
    #[error("Vector contains non-finite values (NaN or Infinity) at index {index}")]
    NonFiniteValue { index: usize },
}
