use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimeSeriesError {
    #[error("Stream is empty or truncated")]
    TruncatedStream,
    #[error("Invalid control byte")]
    InvalidControlByte,
}

/// A DataPoint in the Time-Series engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp_ms: u64,
    pub value: f64,
}

/// Implements the Facebook Gorilla Time-Series compression protocol.
/// Provides Delta-of-Delta timestamp compression and XOR float compression.
pub struct GorillaCompressor {
    buffer: BytesMut,
    previous_timestamp: u64,
    previous_delta: i64,
    previous_value: u64,
    count: usize,
}

impl GorillaCompressor {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            previous_timestamp: 0,
            previous_delta: 0,
            previous_value: 0,
            count: 0,
        }
    }

    /// Appends a new data point to the compressed stream.
    pub fn append(&mut self, point: DataPoint) {
        let val_bits = point.value.to_bits();

        if self.count == 0 {
            // Header: Store block starting timestamp and first value as raw 64-bit blocks
            self.buffer.put_u64(point.timestamp_ms);
            self.buffer.put_u64(val_bits);

            self.previous_timestamp = point.timestamp_ms;
            self.previous_value = val_bits;
            self.count += 1;
            return;
        }

        // 1. Timestamp Delta-of-Delta Compression
        let delta = point.timestamp_ms as i64 - self.previous_timestamp as i64;
        let delta_of_delta = delta - self.previous_delta;

        if delta_of_delta == 0 {
            self.buffer.put_u8(0); // 0: delta unchanged
        } else {
            self.buffer.put_u8(1); // 1: delta changed
            self.buffer.put_i64(delta_of_delta);
        }

        // 2. Value XOR Compression
        let xor = self.previous_value ^ val_bits;
        if xor == 0 {
            self.buffer.put_u8(0); // 0: value unchanged
        } else {
            self.buffer.put_u8(1); // 1: value changed
            self.buffer.put_u64(xor);
        }

        self.previous_timestamp = point.timestamp_ms;
        self.previous_delta = delta;
        self.previous_value = val_bits;
        self.count += 1;
    }

    /// Finishes the stream and returns the compressed bytes prepended with total point count.
    pub fn finish(self) -> Bytes {
        let mut final_buf = BytesMut::with_capacity(8 + self.buffer.len());
        final_buf.put_u64(self.count as u64);
        final_buf.extend_from_slice(&self.buffer);
        final_buf.freeze()
    }
}

impl Default for GorillaCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Decompresses a Gorilla-encoded stream back into DataPoints.
pub struct GorillaDecompressor;

impl GorillaDecompressor {
    pub fn decompress(mut bytes: Bytes) -> Result<Vec<DataPoint>, TimeSeriesError> {
        if bytes.len() < 8 {
            return Err(TimeSeriesError::TruncatedStream);
        }

        let count = bytes.get_u64() as usize;
        if count == 0 {
            return Ok(Vec::new());
        }

        if bytes.len() < 16 {
            return Err(TimeSeriesError::TruncatedStream);
        }

        let first_timestamp = bytes.get_u64();
        let first_val_bits = bytes.get_u64();

        let mut points = Vec::with_capacity(count);
        points.push(DataPoint {
            timestamp_ms: first_timestamp,
            value: f64::from_bits(first_val_bits),
        });

        let mut prev_timestamp = first_timestamp;
        let mut prev_delta = 0i64;
        let mut prev_val_bits = first_val_bits;

        for _ in 1..count {
            if bytes.is_empty() {
                return Err(TimeSeriesError::TruncatedStream);
            }

            // 1. Decode timestamp
            let dod_control = bytes.get_u8();
            let dod = if dod_control == 0 {
                0i64
            } else if dod_control == 1 {
                if bytes.len() < 8 {
                    return Err(TimeSeriesError::TruncatedStream);
                }
                bytes.get_i64()
            } else {
                return Err(TimeSeriesError::InvalidControlByte);
            };

            let delta = prev_delta + dod;
            let timestamp = (prev_timestamp as i64 + delta) as u64;

            // 2. Decode value XOR
            if bytes.is_empty() {
                return Err(TimeSeriesError::TruncatedStream);
            }
            let val_control = bytes.get_u8();
            let xor = if val_control == 0 {
                0u64
            } else if val_control == 1 {
                if bytes.len() < 8 {
                    return Err(TimeSeriesError::TruncatedStream);
                }
                bytes.get_u64()
            } else {
                return Err(TimeSeriesError::InvalidControlByte);
            };

            let val_bits = prev_val_bits ^ xor;
            let value = f64::from_bits(val_bits);

            points.push(DataPoint {
                timestamp_ms: timestamp,
                value,
            });

            prev_timestamp = timestamp;
            prev_delta = delta;
            prev_val_bits = val_bits;
        }

        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gorilla_roundtrip_compression() {
        let mut compressor = GorillaCompressor::new();

        let original_points = vec![
            DataPoint { timestamp_ms: 1000, value: 25.5 },
            DataPoint { timestamp_ms: 2000, value: 25.5 }, // Same value
            DataPoint { timestamp_ms: 3000, value: 26.0 }, // Constant delta, different value
            DataPoint { timestamp_ms: 4500, value: 26.0 }, // Different delta, same value
            DataPoint { timestamp_ms: 6000, value: 30.125 },
        ];

        for pt in &original_points {
            compressor.append(pt.clone());
        }

        let compressed = compressor.finish();
        assert!(compressed.len() > 0);

        let decompressed = GorillaDecompressor::decompress(compressed).unwrap();
        assert_eq!(original_points, decompressed);
    }
}
