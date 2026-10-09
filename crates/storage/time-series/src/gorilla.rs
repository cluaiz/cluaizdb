use bytes::{BufMut, Bytes, BytesMut};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Stream is empty")]
    EmptyStream,
}

/// A DataPoint in the Time-Series engine.
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub timestamp_ms: u64,
    pub value: f64,
}

/// Implements the Facebook Gorilla Time-Series compression protocol.
/// This implementation provides the interface and structural logic for 
/// Delta-of-Delta timestamp compression and XOR float compression.
pub struct GorillaCompressor {
    buffer: BytesMut,
    previous_timestamp: u64,
    previous_delta: i64,
    previous_value: u64, // f64 represented as bits
    first_point: bool,
}

impl GorillaCompressor {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            previous_timestamp: 0,
            previous_delta: 0,
            previous_value: 0,
            first_point: true,
        }
    }

    /// Appends a new data point to the compressed stream.
    pub fn append(&mut self, point: DataPoint) {
        let val_bits = point.value.to_bits();

        if self.first_point {
            // Header: Store block starting timestamp and first value as raw 64-bit blocks
            self.buffer.put_u64(point.timestamp_ms);
            self.buffer.put_u64(val_bits);
            
            self.previous_timestamp = point.timestamp_ms;
            self.previous_value = val_bits;
            self.first_point = false;
            return;
        }

        // 1. Timestamp Delta-of-Delta Compression
        let delta = point.timestamp_ms as i64 - self.previous_timestamp as i64;
        let delta_of_delta = delta - self.previous_delta;

        // In a true bit-packed Gorilla, we would emit control bits (e.g. '0' if dod == 0, '10' if small range, etc.)
        // For architectural readiness, we simulate this by packing the dod.
        if delta_of_delta == 0 {
            self.buffer.put_u8(0); // Control bit 0
        } else {
            self.buffer.put_u8(1); // Control bit 1
            self.buffer.put_i64(delta_of_delta);
        }

        // 2. Value XOR Compression
        let xor = self.previous_value ^ val_bits;

        if xor == 0 {
            self.buffer.put_u8(0); // Control bit 0: Same value
        } else {
            self.buffer.put_u8(1); // Control bit 1: Value changed
            // In full Gorilla, we calculate leading/trailing zeros to only store meaningful bits.
            // Here we store the raw XOR diff.
            self.buffer.put_u64(xor);
        }

        self.previous_timestamp = point.timestamp_ms;
        self.previous_delta = delta;
        self.previous_value = val_bits;
    }

    /// Finishes the stream and returns the compressed bytes.
    pub fn finish(self) -> Bytes {
        self.buffer.freeze()
    }
}
