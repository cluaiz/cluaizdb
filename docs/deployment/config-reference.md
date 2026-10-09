# ⚙️ Configuration Reference (`cluaizdb.toml`)

CLUAIZD is configured via a `cluaizdb.toml` file located in the working directory from which the server is run. 

If no `cluaizdb.toml` is found, CLUAIZD will boot with sensible defaults.

## Full Reference

```toml
[server]
# Port to bind the HTTP and WebSocket server.
# Default: 7331
port = 7331

# IP address to bind to. 0.0.0.0 binds to all available interfaces.
# Default: "0.0.0.0"
host = "0.0.0.0"

[database]
# Default concurrency model for new collections/tenants.
# "dashmap": Multi-threaded lock-free access for high-speed concurrent loads (Max OPS).
# "mutex": Single-threaded strict locks for absolute transaction isolation.
# Default: "dashmap"
concurrency_mode = "dashmap"

# Default payload serialization format.
# "json": Flexible schema-less text format.
# "protobuf": Fast binary encoded structured data.
# "flatbuffers": Zero-copy layout binary format.
# Default: "json"
payload_format = "json"

[storage]
# Directory where LMDB environment files and WAL files will be stored.
# Default: "./data"
data_dir = "./data"

# Maximum size of a single LMDB environment (shard) in bytes.
# Default: 10GB (10737418240)
# Note: LMDB requires continuous virtual memory. Ensure the OS has enough swap/memory limits.
map_size_bytes = 10737418240

[dreamer]
# How often the Dreamer GC daemon evaluates neurons for promotion/eviction.
# Default: 10 (seconds)
scan_interval_seconds = 10

# RAM threshold at which Hot neurons are demoted to Warm (Payload stripped).
# E.g., 0.20 means when only 20% of system RAM is free.
# Default: 0.20
ram_warn_threshold = 0.20

# RAM threshold at which Warm neurons are compressed to Cold (ZSTD).
# Default: 0.05 (5% free RAM)
ram_critical_threshold = 0.05

# Default time-to-live for neurons that do not have an explicit `on_lifecycle` hook.
# Default: 3600 (1 hour)
default_cold_ttl_seconds = 3600

[transit_lounge]
# Size of the lock-free RAM ring buffer for incoming writes.
# Increase if you have massive burst ingestion, but requires more RAM.
# Default: 1000000 (1 million neurons)
queue_capacity = 1000000

# How often the WAL Writer polling loop flushes the Transit Lounge to disk.
# Default: 50 (milliseconds)
flush_interval_ms = 50
```

## Environment Variables
Any value in `cluaizdb.toml` can be overridden by an environment variable using the prefix `CLUAIZD_`, followed by the section and key in uppercase.

Examples:
- `CLUAIZD_SERVER_PORT=8080` overrides `[server] port`.
- `CLUAIZD_STORAGE_DATA_DIR=/mnt/nvme0/data` overrides `[storage] data_dir`.

## Collection-Level Local Overrides (Method B)
While `cluaizdb.toml` sets the global default for the entire engine, you can override database settings (like `concurrency_mode` and `payload_format`) on a per-collection (tenant) basis.

When a new tenant/collection is created, CLUAIZD creates a `collection_config.json` inside its physical shard directory (`data/shards/<tenant_id>/collection_config.json`).
- If you edit this file or specify it during creation, those local settings take precedence.
- This allows you to run your `payments_db` with strict `mutex` locks and `protobuf`, while running `analytics_db` on lock-free `dashmap` with `json`—all seamlessly on the exact same server!
