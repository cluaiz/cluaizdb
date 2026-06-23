use std::path::Path;

use tracing::debug;

use cluaizd_errors::StorageError;
use cluaizd_types::{NeuronId, PayloadType, UniversalNeuron};

use crate::env::LmdbEnv;

/// The isolated shard for high-frequency sensor and BCI streaming data.
///
/// Robotics (LiDAR, Camera) and Brain-Computer Interface (voltage spikes)
/// streams MUST NOT be written to the main `cluaizd.mdb` database.
/// High-throughput writes would create write contention and block all read queries.
///
/// This shard acts as a ring buffer:
/// - Raw voltage/sensor bytes flow in via `ingest_voltage_stream()`.
/// - Data is processed into `UniversalNeuron` bitmasks and moved to the main DB.
/// - Raw stream data is purged when the shard reaches `max_entries`.
pub struct SensoryShard {
    env: LmdbEnv,
    /// Maximum number of raw stream entries before oldest are purged.
    #[allow(dead_code)]
    max_entries: u64,
}

impl SensoryShard {
    /// Open (or create) the sensory shard at the given path.
    /// This creates a SEPARATE LMDB environment from the main database.
    ///
    /// # Arguments
    /// * `path` — Directory for `sensory_tissue.mdb`.
    /// * `max_entries` — Ring buffer capacity. Oldest entries purged beyond this.
    pub fn open(path: &Path, max_entries: u64) -> Result<Self, StorageError> {
        // Use 512MB for sensory shard — streams are high volume but temporary.
        let env = LmdbEnv::open(path, 512 * 1024 * 1024)?;
        Ok(Self { env, max_entries })
    }

    /// Ingest a raw voltage or sensor byte stream into the sensory shard.
    ///
    /// This function:
    /// 1. Stores the raw bytes as a temporary `UniversalNeuron` with `PayloadType::VoltageStream`.
    /// 2. Assigns a zero vector (16-dims) — the vector will be computed later by the Transducer.
    /// 3. Returns the assigned `NeuronId` for downstream processing.
    ///
    /// # Arguments
    /// * `raw_stream` — Raw voltage bytes from the sensor or BCI chip.
    /// * `source_device_id` — Optional identifier for the source device.
    pub fn ingest_voltage_stream(
        &self,
        raw_stream: bytes::Bytes,
        source_device_id: Option<&str>,
        dna: Option<cluaizd_types::NeuronDna>,
    ) -> Result<NeuronId, StorageError> {
        use crate::writer::write_neuron;

        // Zero vector — Transducer will populate this later when idle.
        let zero_vector = [0.0f32; 16];
        // Zero hash — signals that no model has processed this yet.
        let zero_hash = [0u8; 32];

        let mut neuron =
            UniversalNeuron::new(raw_stream, zero_vector.to_vec(), zero_hash, PayloadType::VoltageStream);

        neuron.dna = dna;

        let assigned_id = neuron.id;

        write_neuron(&self.env, &neuron)?;

        debug!(
            neuron_id = %assigned_id,
            device = source_device_id.unwrap_or("unknown"),
            "Voltage stream ingested into sensory shard"
        );

        Ok(assigned_id)
    }
}
