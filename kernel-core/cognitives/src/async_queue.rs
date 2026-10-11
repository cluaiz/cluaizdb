//! Asymmetric double-queue ring pipeline bypassing LMDB single-writer mutex.

use crate::error::{CognitiveError, Result};
use cluaizdb_substrate::schema::EdgeMetadata;
use cluaizdb_substrate::traits::StorageSubstrate;
use crossbeam_queue::SegQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug)]
pub enum QueueCommand {
    PersistPayload {
        col: u32,
        id: u128,
        data: Vec<u8>,
    },
    RecordTimeseries {
        series_id: u64,
        ts: u64,
        metrics: Vec<u8>,
    },
    MapEdge {
        source: u128,
        label: u32,
        dest: u128,
    },
    FlushPipeline,
}

/// Asymmetric write pipeline decoupling writes into a lock-free queue to protect read latency.
pub struct AsymmetricWriteQueue {
    queue: Arc<SegQueue<QueueCommand>>,
    is_running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
}

impl AsymmetricWriteQueue {
    pub fn new<S: StorageSubstrate + 'static>(substrate: Arc<S>) -> Self {
        let queue = Arc::new(SegQueue::new());
        let is_running = Arc::new(AtomicBool::new(true));

        let worker_queue = Arc::clone(&queue);
        let worker_running = Arc::clone(&is_running);

        let worker_handle = thread::spawn(move || {
            while worker_running.load(Ordering::Relaxed) || !worker_queue.is_empty() {
                let mut processed_batch = 0;
                while let Some(command) = worker_queue.pop() {
                    match command {
                        QueueCommand::PersistPayload { col, id, data } => {
                            if let Err(e) = substrate.put_record(col, id, &data) {
                                eprintln!("[AsymmetricWriteQueue] put_record failed: {:?}", e);
                            }
                        }
                        QueueCommand::RecordTimeseries {
                            series_id,
                            ts,
                            metrics,
                        } => {
                            if let Err(e) = substrate.put_timeseries_chunk(series_id, ts, &metrics) {
                                eprintln!("[AsymmetricWriteQueue] put_timeseries_chunk failed: {:?}", e);
                            }
                        }
                        QueueCommand::MapEdge { source, label, dest } => {
                            let metadata = EdgeMetadata {
                                weight: 1.0,
                                created_at: 0,
                                valid_until: 0,
                            };
                            let _ = substrate.add_edge(source, label, dest, metadata);
                        }
                        QueueCommand::FlushPipeline => {
                            let _ = substrate.sync();
                        }
                    }
                    processed_batch += 1;
                    if processed_batch >= 128 {
                        break;
                    }
                }

                if processed_batch > 0 {
                    let _ = substrate.sync();
                } else {
                    thread::sleep(Duration::from_millis(2));
                }
            }
        });

        Self {
            queue,
            is_running,
            worker_handle: Some(worker_handle),
        }
    }

    /// Dispatches a write command into the lock-free queue in nanoseconds (< 100ns).
    pub fn dispatch(&self, command: QueueCommand) -> Result<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(CognitiveError::QueueDispatch(
                "Cannot dispatch command: pipeline is terminated".into(),
            ));
        }
        self.queue.push(command);
        Ok(())
    }

    /// Flushes and gracefully shuts down the background commit worker.
    pub fn shutdown(mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AsymmetricWriteQueue {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
