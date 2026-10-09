# ⚙️ Configuration Reference (A-Z)

Welcome to the **cluaizdb Configuration Reference**.
Engine parameters are strictly governed via the `cluaizdb.toml` file located in the root installation directory. This reference details how Database Administrators (DBAs) can tune the engine for specific hardware constraints.

## Configuration Index

| Parameter | Category | Description |
|---|---|---|
| [`memory_map_size`](./config/memory_map_size.md) | Storage | Allocates the maximum virtual memory space for the LMDB environment.<hr>`memory_map_size = "10GB"` |
| [`port_binding`](./config/port_binding.md) | Networking | Configures the TCP socket bindings for the API and telemetry endpoints.<hr>`port = 7331` |
| [`telemetry`](./config/telemetry.md) | Observability | Configures Prometheus scraping metrics and console logging verbosity.<hr>`metrics_enabled = true` |
| [`wal_sync`](./config/wal_sync.md) | Durability | Tunes the `fsync` behavior of the Write-Ahead Log for speed vs safety.<hr>`wal_sync = "fsync_per_tx"` |
