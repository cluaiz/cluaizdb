# 💾 Double-Buffered Group Commit Pipeline

> **Component:** `modules/group-commit` (`lib.rs`)  
> **Reusability Level:** High-Throughput Durable Write Coordination  
> **Throughput Model:** Decoupled In-Memory MPSC Queue with Tokio Worker Batching

---

## 🧭 Architectural Flow: Decoupled Batch Commit

Under heavy concurrent write workloads, performing synchronous disk syncs (`fsync`) for every single insertion causes severe I/O stalls.

The `GroupCommitter` decouples caller write dispatching from physical disk commits:

```mermaid
flowchart TD
    subgraph CALLERS["Concurrent Worker Threads / Tasks"]
        T1["Thread 1: commit(op1)"]
        T2["Thread 2: commit(op2)"]
        T3["Thread 3: commit(op3)"]
    end

    subgraph QUEUE["Decoupled Tokio MPSC Channel"]
        MPSC["mpsc::channel(capacity: 10,000)<br/>Buffers WriteTask { op, oneshot_sender }"]
    end

    subgraph WORKER["Background Committer Loop (Tokio Task)"]
        BATCH["Batch Accumulator<br/>Collects incoming WriteOperations"]
        TRIGGER{"Batch Trigger Condition Met?<br/>1. Batch size &gt;= max_batch_size (128)<br/>OR 2. Timeout &gt;= max_latency (2ms)"}
        COMMIT["Single Substrate Write Transaction<br/>Atomically writes entire batch into LMDB"]
        RESOLVE["Resolves all oneshot::Sender channels"]
    end

    T1 --> MPSC
    T2 --> MPSC
    T3 --> MPSC
    MPSC --> BATCH
    BATCH --> TRIGGER
    TRIGGER -->|Yes| COMMIT
    COMMIT --> RESOLVE
    RESOLVE -.->|Notify OK| T1
    RESOLVE -.->|Notify OK| T2
    RESOLVE -.->|Notify OK| T3
```

---

## ⚙️ Configuration & Batch Trigger Rules

The batching behavior is governed by `GroupCommitConfig`:

```rust
pub struct GroupCommitConfig {
    pub max_batch_size: usize,      // Default: 128 operations
    pub max_latency: Duration,      // Default: 2 milliseconds
    pub channel_capacity: usize,    // Default: 10,000 tasks
}
```

### The Dual Trigger Condition:
1. **Size Trigger (`max_batch_size = 128`)**:
   - If a burst of write operations arrives, the batch commits immediately upon reaching 128 items, maximizing throughput without unnecessary delays.
2. **Time-Limit Trigger (`max_latency = 2ms`)**:
   - Under low traffic, operations are never delayed indefinitely. If fewer than 128 items arrive, the worker flushes the batch as soon as the 2ms timer fires.

---

## 🛡️ Failure Modes & Recovery Guarantees

```mermaid
flowchart TD
    OP["WriteTask Dispatched"] --> RECV["Worker Receives Task"]
    RECV --> TXN["Open Substrate Write Transaction"]
    
    TXN -->|Transaction Fails| ERR_SUB["GroupCommitError::Substrate<br/>Notifies all waiting callers with Error"]
    TXN -->|Worker Dropped| ERR_CAN["GroupCommitError::Cancelled<br/>Oneshot channel dropped gracefully"]
    TXN -->|Transaction OK| OK_SUB["Oneshot responds Ok(()) to each caller"]
```

- **Thread Safety**: `GroupCommitter` derives `Clone` and can be passed across arbitrary threads and async tasks.
- **Graceful Error Bubbling**: If an LMDB disk quota is reached or I/O fails, the error bubbles back to every caller via their respective `oneshot::Receiver`.
- **Channel Saturation Defense**: If the channel exceeds `channel_capacity`, caller dispatches fail with `GroupCommitError::ChannelSend`, preventing unbounded RAM consumption.
