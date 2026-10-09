# Cluaizdb Enterprise CLI (`cluaizdb`)

The Cluaizdb CLI is the primary administrative interface for the database. It is designed for **headless server administration**, allowing you to manage configurations, background daemons, and database shards directly from the terminal without a GUI.

> **Note:** The CLI connects directly to the LMDB storage engine via FFI for most operations, meaning you can inspect and modify database shards even when the server daemon is completely stopped.

---

## 🚀 Quick Start

To build and run the CLI locally from the source:
```bash
cargo build -p cli
./target/debug/cluaizdb --help
```

---

## 🏃 Direct Server Launch (`run`)

The most common command. Instead of using the background daemon tools, you can directly launch the `cluaizdb` server process in the foreground:

```bash
cluaizdb run
```

*Note: This automatically targets `./target/debug/server` by default, or you can specify a custom binary with `--bin`.*

---

## 🛠️ Global Flags

These flags can be applied to **any** CLI command.

| Flag | Description |
|---|---|
| `--json` | Outputs the result as structured JSON. Essential for Bash scripting and CI/CD pipelines. |
| `--verbose` or `-v` | Prints extra debugging information (e.g., shard paths, map sizes, execution modes). |
| `--path <dir>` | Overrides the default shard directory (default is `./data/shards`). |

**Example:** Fetch server port for a bash script:
```bash
PORT=$(cluaizdb --json config get server.port)
```

---

## ⚙️ Config Management (`config`)

Read and modify the `cluaizdb.toml` file dynamically without opening an editor. The CLI uses dot-notation for nested fields.

### Commands

- **`config show`**
  Prints the entire configuration file.
  
- **`config get <key>`**
  Extracts a specific key.
  ```bash
  cluaizdb config get database.concurrency_mode
  ```

- **`config set <key> [value]`**
  Updates a specific key and atomically writes it back to `cluaizdb.toml`.
  If you omit the `[value]`, the CLI will launch an **interactive dropdown menu** for supported keys (e.g., `database.concurrency_mode`, `database.payload_format`).
  
  *Direct assignment:*
  ```bash
  cluaizdb config set server.port 8080
  ```
  
  *Interactive selection:*
  ```bash
  cluaizdb config set database.concurrency_mode
  # Prompts: Select concurrency_mode:
  # > dashmap
  #   mutex
  
  cluaizdb config set database.payload_format
  # Prompts: Select payload_format:
  # > flatbuffers
  #   protobuf
  #   json
  ```

---

## 🖥️ Server Daemon Control (`server`)

Manage the Cluaizdb background process.

### Commands

- **`server start`**
  Spawns the server binary as a detached background daemon. It writes its process ID to `data/cluaizdb.pid`.
  ```bash
  cluaizdb server start --bin ./target/release/server
  ```

- **`server status`**
  Checks if the daemon is currently running and outputs its PID.

- **`server stop`**
  Gracefully terminates the background daemon using its PID.

- **`server logs`**
  Tails the server's background log file directly to your terminal.
  ```bash
  cluaizdb server logs --lines 100
  ```

---

## 💾 Database Shard Operations (`db`)

Directly interact with the LMDB shard files via FFI. **These commands do not require the server to be running.**

### Commands

- **`db health`**
  Performs a quick sanity check and returns the total number of neurons in the shard.

- **`db stats`**
  Provides a deep dive into your database footprint. Outputs physical file size and a breakdown of neurons currently residing in Hot, Warm, and Cold storage tiers.
  ```bash
  cluaizdb db stats
  ```

- **`db backup <destination>`**
  Creates a **live, zero-downtime safe copy** of the database shard using LMDB's native copy-on-write mechanisms.
  ```bash
  cluaizdb db backup ./data/nightly_backup
  ```

- **`db compact`**
  Creates a defragmented copy of the database, removing any free pages to save disk space. *Requires the server daemon to be stopped.*

- **`db inspect <uuid>`**
  Fetches a specific neuron directly from the disk and prints its raw state.

---

## 📜 Write-Ahead Log Diagnostics (`wal`)

The Write-Ahead Log ensures ACID compliance. The CLI allows you to inspect it for corruption or uncommitted operations.

### Commands

- **`wal inspect`**
  Reads the `.wal` files in `data/wal` and prints pending operations. Crucially, it detects and reports the exact number of corrupt entries if a catastrophic crash occurred.
  ```bash
  cluaizdb wal inspect --limit 50
  ```

---

## 🧬 DNA & WASM Management (`dna`)

Manage hot-reloadable WASM business logic without recompiling the core database.

### Commands

- **`dna list`**
  Shows all active WASM logic templates currently loaded in the database.

- **`dna deploy <wasm_file>`**
  Deploys a compiled WASM file into the active DNA directory, instantly making it available to the query execution engine.

- **`dna remove <name>`**
  Unloads a DNA module.

---

## 🔍 Raw FFI Querying (`query`)

Execute CDQL queries directly against the storage engine without passing through the HTTP layer. 

> **Note:** Because this bypasses the server, only basic read operations (`Find`, `FindById`, `Limit`) are supported. Advanced operations (Vector Similarity, Graph Traversal) require the full server runtime.

```bash
cluaizdb-cli query "FIND * LIMIT 10"
```
