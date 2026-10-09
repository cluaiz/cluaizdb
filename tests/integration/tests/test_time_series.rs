use cluaizdb_time_series::{GorillaCompressor, DataPoint};

#[test]
fn test_gorilla_compression_flow() {
    let mut compressor = GorillaCompressor::new();

    // Simulate an IoT temperature sensor sending data every second
    let points = vec![
        DataPoint { timestamp_ms: 1000, value: 25.1 },
        DataPoint { timestamp_ms: 2000, value: 25.2 }, // +1s, +0.1
        DataPoint { timestamp_ms: 3000, value: 25.2 }, // +1s, +0.0 (identical)
        DataPoint { timestamp_ms: 4000, value: 25.3 }, // +1s, +0.1
    ];

    for p in points {
        compressor.append(p);
    }

    let compressed_bytes = compressor.finish();
    
    // Original size would be 4 * (8 byte TS + 8 byte f64) = 64 bytes
    // Gorilla compressed size should be significantly smaller.
    // In our simplified bit-stream layout: 
    // Header (16) + 3 diffs * (~10 bytes max) = ~46 bytes max, usually much less.
    println!("Compressed size: {} bytes", compressed_bytes.len());
    assert!(!compressed_bytes.is_empty());
    assert!(compressed_bytes.len() < 64, "Compression failed to reduce size");
}
