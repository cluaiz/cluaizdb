# WAL Crash Recovery

Database crashes happen. Power goes out, instances get preempted, or hardware faults. Data integrity in these scenarios separates toy databases from production systems.

CLUAIZD guarantees durability through its **Write-Ahead Log (WAL)**.

## How the WAL Works
Before any mutation (Insert, Update, Delete) is flushed to the core LMDB storage, it is synchronously appended to the `cluaizdb_wal.log` file. 

This log is an append-only sequence of `WalOperation` events. Because it is append-only, the operating system can write to it at blazing fast speeds sequentially, avoiding the disk-seek latency that random writes incur.

### The Boot Sequence
When the `cluaizdb-server` starts, it does not immediately accept HTTP traffic. It undergoes an **Autonomic Boot Sequence**:

1. **Lock Acquisition:** It locks the LMDB environment.
2. **WAL Scan:** It reads `cluaizdb_wal.log` sequentially from the beginning.
3. **Idempotent Replay:** For every `Insert` or `Delete` operation found in the WAL, it checks if that operation is already reflected in the core LMDB shards.
   - If missing, the operation is **Replayed** into LMDB.
   - If present, the operation is safely ignored (Idempotency).
4. **Trimming:** Once the replay is complete, the WAL file is truncated and reset, freeing up disk space.
5. **Gateway Open:** The server finally opens port `8080` to accept incoming traffic.

> [!WARNING]
> Do NOT manually delete `cluaizdb_wal.log` if the server experienced a hard crash. Doing so will result in permanent data loss for any transactions that were acknowledged to the user but not yet flushed from the OS page cache to the LMDB `.mdb` data files.

## Performance Impact
By default, CLUAIZD uses asynchronous OS flushes for its WAL to maintain extreme write speeds. If you are operating in a mission-critical financial environment where even 100ms of data loss is unacceptable on power failure, you can configure the WAL to force an `fsync` after every transaction via the `config.toml`.
