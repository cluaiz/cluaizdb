# Installation Guide

CLUAIZD is a self-contained Rust binary. There are no external service dependencies (no JVM, no Python runtime, no Node.js). The only requirement is the Rust toolchain to compile it from source.

## Method 1: From Source (Recommended for Development)

### Step 1: Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup update stable
```

Verify: `rustc --version` should output `1.75+`.

### Step 2: Install LLVM (Required for LMDB C bindings)

**Windows (via winget):**
```powershell
winget install LLVM.LLVM
# Restart your terminal after installation
```

**macOS:**
```bash
brew install llvm
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

**Ubuntu / Debian:**
```bash
sudo apt-get update && sudo apt-get install -y clang llvm libclang-dev
```

### Step 3: Clone and Run
```bash
git clone https://github.com/cluaiz/cluaizdb.git
cd cluaizdb
cargo run -p cluaizdb-server
```

> [!TIP]
> The first build may take 3-5 minutes as Cargo compiles the WASM runtime (wasmtime) and its dependencies. Subsequent builds are incremental and take ~10 seconds.

On success, you will see:
```
[CLUAIZD] WAL recovery complete — 0 entries replayed.
[CLUAIZD] Shard manager initialized at ./out/
[CLUAIZD] HTTP server listening on 0.0.0.0:7331
```

---

## Method 2: Docker

```bash
docker pull ghcr.io/cluaiz/cluaizdb:latest
docker run -d \
  --name cluaizdb \
  -p 7331:7331 \
  -v $(pwd)/cluaizdb-data:/app/out \
  ghcr.io/cluaiz/cluaizdb:latest
```

The `/app/out` volume is where CLUAIZD stores its LMDB `.mdb` data files and WAL log. Always mount this as a persistent volume to survive container restarts.

---

## Method 3: Build the C-FFI Shared Library

For embedding CLUAIZD into a Python, C++, or Rust application as a native library (with 0ms latency, no HTTP overhead):

```bash
cargo build --release -p cluaizdb-ffi

# Linux output:
ls target/release/libcluaizdb.so

# Windows output:
ls target/release/cluaizdb.dll
```

Copy the `.so` or `.dll` and the `ffi/cluaizdb.h` header into your target project.

---

## Verifying Your Installation

Run the health check:
```bash
curl http://localhost:7331/ws/telemetry
```

Expected response:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "shards_open": 1,
  "wal_entries": 0
}
```

---

## Configuration (Optional)

CLUAIZD reads a `config.toml` at startup if present in the working directory.

```toml
[server]
host = "0.0.0.0"
port = 7331

[storage]
data_dir = "./out"
map_size_mb = 4096   # Maximum LMDB database size (4 GB default)
wal_fsync = false    # Set to true for mission-critical durability

[dreamer]
ram_pressure_threshold = 0.15  # Start evicting at 15% free RAM
cold_ttl_seconds = 3600        # Demote to Cold after 1 hour of inactivity
```
