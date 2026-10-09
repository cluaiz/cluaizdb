use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use cluaizdb_types::UniversalNeuron;
use tracing::{info, warn};

/// The Transit Lounge (Lock-Free Ring Buffer Queue)
/// 
/// This acts as a high-speed RAM buffer for incoming Neurons.
/// Instead of blocking the HTTP/WebSocket threads while writing to LMDB (which could cause stuttering),
/// the engine instantly pushes the Neuron into this lock-free ArrayQueue in O(1) time.
/// 
/// A separate background worker thread (The Flusher) will continuously pop from this queue,
/// execute DNA hooks (if any), and persist the records into the LMDB database.
pub struct TransitLounge {
    /// Lock-free bounded queue. We use ArrayQueue to prevent unbounded memory growth (OOM protection).
    queue: Arc<ArrayQueue<UniversalNeuron>>,
    capacity: usize,
}

impl TransitLounge {
    /// Initializes a new Transit Lounge with a fixed capacity.
    /// Default recommended capacity is 1,000,000 for high-throughput environments.
    pub fn new(capacity: usize) -> Self {
        info!("Initializing TransitLounge (Lock-Free RAM Queue) with capacity: {}", capacity);
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            capacity,
        }
    }

    /// Pushes a new Neuron into the queue.
    /// This is completely lock-free and returns instantly (0ms latency).
    /// If the queue is full, it returns an error, triggering backpressure on the ingestion layer.
    pub fn push(&self, neuron: UniversalNeuron) -> Result<(), String> {
        self.queue.push(neuron).map_err(|_| {
            let err_msg = format!("TransitLounge capacity ({}) exceeded! Dropping Neuron. Backpressure required.", self.capacity);
            warn!("{}", err_msg);
            err_msg
        })
    }

    /// Pops a Neuron from the queue. Returns None if the queue is empty.
    /// The background Flusher thread calls this in a loop.
    pub fn pop(&self) -> Option<UniversalNeuron> {
        self.queue.pop()
    }

    /// Returns the current number of pending Neurons in the queue.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns true if the queue is completely full.
    pub fn is_full(&self) -> bool {
        self.queue.is_full()
    }
}
