use serde::{Deserialize, Serialize};

/// Represents the modality/type of data stored in the raw payload of a Neuron.
/// This allows the storage engine and transducer to handle each type correctly
/// without runtime inspection of the payload bytes themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PayloadType {
    /// Plain text data (UTF-8 encoded).
    Text = 0,
    /// Compressed audio bytes (PCM, MP3, FLAC, etc.).
    Audio = 1,
    /// Video frames (H.264, AV1, etc.).
    Video = 2,
    /// Raw electrophysiological voltage stream — for Robotics and BCI use cases.
    /// These MUST be ingested via the dedicated `sensory_shard` write path.
    VoltageStream = 3,
    /// Source code, shell scripts, or any structured program text.
    Code = 4,
    /// Binary blob — any untyped raw byte data.
    Binary = 5,
}

impl std::fmt::Display for PayloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PayloadType::Text => "text",
            PayloadType::Audio => "audio",
            PayloadType::Video => "video",
            PayloadType::VoltageStream => "voltage_stream",
            PayloadType::Code => "code",
            PayloadType::Binary => "binary",
        };
        write!(f, "{}", s)
    }
}
