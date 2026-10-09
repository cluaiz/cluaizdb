use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::debug;

use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv, SubstrateError};

#[derive(Error, Debug)]
pub enum GroupCommitError {
    #[error("Channel send failed: {0}")]
    ChannelSend(String),

    #[error("Substrate storage error: {0}")]
    Substrate(#[from] SubstrateError),

    #[error("Worker task dropped or cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, GroupCommitError>;

/// A decoupled raw write operation targeting Substrate.
#[derive(Debug, Clone)]
pub struct WriteOperation {
    pub col_id: u32,
    pub record_id: u128,
    pub payload: Vec<u8>,
}

struct WriteTask {
    op: WriteOperation,
    respond_to: oneshot::Sender<Result<()>>,
}

/// Configuration parameters for Group Commit batching.
#[derive(Debug, Clone)]
pub struct GroupCommitConfig {
    pub max_batch_size: usize,
    pub max_latency: Duration,
    pub channel_capacity: usize,
}

impl Default for GroupCommitConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 128,
            max_latency: Duration::from_millis(2),
            channel_capacity: 10_000,
        }
    }
}

/// Thread-safe coordinator for batched durable writes.
#[derive(Clone)]
pub struct GroupCommitter {
    sender: mpsc::Sender<WriteTask>,
}

impl GroupCommitter {
    /// Spawns the background committer worker loop using the ambient Tokio runtime.
    pub fn new(substrate: SubstrateEnv, config: GroupCommitConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.channel_capacity);
        tokio::spawn(Self::worker_loop(substrate, rx, config));
        Self { sender: tx }
    }

    /// Submits a write and awaits until it is durably committed to physical disk.
    pub async fn write(&self, col_id: u32, record_id: u128, payload: Vec<u8>) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.sender
            .send(WriteTask {
                op: WriteOperation {
                    col_id,
                    record_id,
                    payload,
                },
                respond_to: resp_tx,
            })
            .await
            .map_err(|e| GroupCommitError::ChannelSend(e.to_string()))?;

        match resp_rx.await {
            Ok(res) => res,
            Err(_) => Err(GroupCommitError::Cancelled),
        }
    }

    /// Background worker draining tasks, executing atomic batch writes, and issuing physical sync.
    async fn worker_loop(
        substrate: SubstrateEnv,
        mut receiver: mpsc::Receiver<WriteTask>,
        config: GroupCommitConfig,
    ) {
        let mut batch: Vec<WriteTask> = Vec::with_capacity(config.max_batch_size);

        while let Some(first_task) = receiver.recv().await {
            batch.push(first_task);

            let deadline = tokio::time::Instant::now() + config.max_latency;

            while batch.len() < config.max_batch_size {
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    break;
                }
                let remaining = deadline - now;

                tokio::select! {
                    biased;
                    task_opt = receiver.recv() => {
                        match task_opt {
                            Some(task) => batch.push(task),
                            None => break,
                        }
                    }
                    _ = tokio::time::sleep(remaining) => {
                        break;
                    }
                }
            }

            debug!("GroupCommitter flushing batch of size: {}", batch.len());

            // Execute atomic batch mutations against substrate
            let mut write_err = None;
            for task in &batch {
                if let Err(e) = substrate.put_record(task.op.col_id, task.op.record_id, &task.op.payload) {
                    write_err = Some(e);
                    break;
                }
            }

            // Enforce physical SSD barrier if writes succeeded
            let commit_result = if let Some(e) = write_err {
                Err(GroupCommitError::Substrate(e))
            } else {
                match substrate.sync() {
                    Ok(()) => Ok(()),
                    Err(e) => Err(GroupCommitError::Substrate(e)),
                }
            };

            // Notify all waiting callers
            for task in batch.drain(..) {
                let _ = task.respond_to.send(match &commit_result {
                    Ok(()) => Ok(()),
                    Err(_) => Err(GroupCommitError::Cancelled),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_group_commit_single_and_batch() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 10 * 1024 * 1024).unwrap();
        let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());

        let col_id = 1u32;
        let rec_id = 42u128;
        let data = b"durable memory payload".to_vec();

        // 1. Single Write
        committer.write(col_id, rec_id, data.clone()).await.unwrap();

        // Verify read from substrate
        let fetched = db.get_record(col_id, rec_id).unwrap();
        assert_eq!(fetched, Some(data));
    }

    #[tokio::test]
    async fn test_concurrent_group_commit_batching() {
        let dir = tempdir().unwrap();
        let db = SubstrateEnv::open(dir.path(), 20 * 1024 * 1024).unwrap();
        let committer = Arc::new(GroupCommitter::new(db.clone(), GroupCommitConfig {
            max_batch_size: 128,
            max_latency: Duration::from_millis(5),
            channel_capacity: 5_000,
        }));

        let count = 128;
        let start = Instant::now();
        let mut handles = Vec::new();

        for i in 0..count {
            let committer_clone = Arc::clone(&committer);
            handles.push(tokio::spawn(async move {
                let rec_id = (1000 + i) as u128;
                let payload = format!("concurrent item {}", i).into_bytes();
                committer_clone.write(1, rec_id, payload).await.unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let elapsed = start.elapsed();
        println!("Committed {} durable items in {:?}", count, elapsed);

        // Verify all 128 items exist in substrate
        for i in 0..count {
            let rec_id = (1000 + i) as u128;
            let expected = format!("concurrent item {}", i).into_bytes();
            assert_eq!(db.get_record(1, rec_id).unwrap(), Some(expected));
        }
    }
}
