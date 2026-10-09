# `wal_sync` Configuration

The `wal_sync` parameter governs the durability guarantees of the Write-Ahead Log (WAL). It allows DBAs to tune the engine on a spectrum between absolute data safety and extreme ingestion throughput.

## Architectural Options

### 1. `fsync_per_tx` (Default, Safest)
The Engine issues an `fsync()` system call for every single `insert`, `update`, or `delete` transaction. The HTTP response is not sent to the client until the physical NVMe/SSD drive confirms the data is fully flushed from volatile cache to stable storage.
* **Throughput**: ~5,000 - 15,000 Writes Per Second (WPS), bottlenecked by physical drive latency.
* **Durability**: 100% crash resilient. Zero data loss upon power failure.

### 2. `fsync_batch` (High Performance)
The Engine buffers transactions in memory and issues an `fsync()` every 10 milliseconds, or when the buffer hits 1 Megabyte.
* **Throughput**: ~100,000 - 300,000 WPS.
* **Durability**: In the event of an abrupt power loss, the engine may lose up to 10 milliseconds of recent data.

### 3. `async_only` (Extreme Throughput)
The Engine hands the data to the Operating System and immediately replies to the client. The OS flushes to disk whenever it decides.
* **Throughput**: >1,000,000 WPS.
* **Durability**: High risk. A kernel panic or power outage will result in arbitrary data loss.

## Configuration Syntax

Found in `cluaizdb.toml` under the `[storage.wal]` section:

```toml
[storage.wal]
# Options: "fsync_per_tx", "fsync_batch", "async_only"
wal_sync = "fsync_batch"
```
