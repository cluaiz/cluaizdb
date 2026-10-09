use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique, time-sortable identifier for a single neuron in the database.
/// Uses UUID v7 internally for both uniqueness and natural time ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NeuronId(Uuid);

impl NeuronId {
    /// Generate a new unique `NeuronId` using UUID v7.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a `NeuronId` from raw bytes. Used for deserialization from storage.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Return the underlying raw bytes. Used for serialization into storage.
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl std::str::FromStr for NeuronId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for NeuronId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NeuronId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_id_round_trip() {
        let id = NeuronId::new();
        let bytes = *id.as_bytes();
        let recovered = NeuronId::from_bytes(bytes);
        assert_eq!(id, recovered);
    }
}
