use std::sync::Arc;
use std::time::Duration;
use crossbeam::queue::ArrayQueue;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use cluaizdb_types::UniversalNeuron;
use engine_lmdb::LmdbEnv;
use wal::WalWriter;

/// The Volatile Synaptic Transit Lounge.
/// A lock-free, concurrent ring buffer that handles sub-millisecond writes
/// before batching them down to the WAL and LMDB Engine.
pub struct TransitLounge {
    queue: Arc<ArrayQueue<UniversalNeuron>>,
}

impl TransitLounge {
    /// Creates a new Transit Lounge with a specific buffer capacity and spawns the flusher thread.
    pub fn new(capacity: usize, env: Arc<LmdbEnv>, wal_writer: Arc<Mutex<WalWriter>>) -> Self {
        let queue = Arc::new(ArrayQueue::new(capacity));
        
        // Spawn the background flusher thread (Layer 1 -> Layer 2/3)
        let queue_clone = Arc::clone(&queue);
        tokio::spawn(async move {
            let mut flush_interval = tokio::time::interval(Duration::from_millis(50)); // Batch every 50ms
            
            loop {
                flush_interval.tick().await;
                
                let mut batch = Vec::new();
                while let Some(neuron) = queue_clone.pop() {
                    batch.push(neuron);
                }

                if !batch.is_empty() {
                    debug!("Transit Lounge: Flushing {} neurons to WAL & LMDB", batch.len());
                    
                    // 1. Write to WAL first for Crash Immunity
                    {
                        let mut wal = wal_writer.lock().await;
                        for neuron in &batch {
                            if let Err(e) = wal.append_write(neuron) {
                                error!("Transit Lounge: Failed to flush to WAL: {}", e);
                            }
                        }
                        // Sync once per batch
                        if let Err(e) = wal.sync() {
                            error!("Transit Lounge: Failed to sync WAL: {}", e);
                        }
                    } // wal lock drops here
                    
                    // 2. Commit to LMDB (Physical Shard)
                    for neuron in batch {
                        if let Err(e) = engine_lmdb::write_neuron(&env, &neuron) {
                            error!("Transit Lounge: Failed to flush to LMDB: {}", e);
                        }
                    }
                }
            }
        });

        Self { queue }
    }

    /// Push a neuron into the RAM ring buffer (O(1) Lock-free).
    /// If the buffer is full, it will return the neuron back as an Err.
    pub fn push(&self, neuron: UniversalNeuron) -> Result<(), UniversalNeuron> {
        self.queue.push(neuron)
    }

    /// Returns how many items are currently waiting in the transit lounge.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
