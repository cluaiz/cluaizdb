# ⚡ Bare-Metal Native Deployment (Recommended)

> *"Docker adds overhead. For a nanosecond-scale database, bare-metal is the only way to unlock its true superpower."*

While CLUAIZD can run in Docker, **we highly discourage it for production workloads.** Docker and virtualized file systems introduce I/O overhead that significantly bottlenecks LMDB's zero-copy memory mapping. 

To get the absolute maximum performance out of CLUAIZD (especially for AI engines, robotics, and high-frequency sensor data), you must run it natively on the machine.

---

## Why Bare-Metal?

1. **Zero I/O Overhead**: LMDB relies on OS-level `mmap` (memory-mapped files). Virtualized containers intercept these calls, slowing down reads to milliseconds instead of nanoseconds.
2. **Direct Hardware Access**: For edge devices (like Raspberry Pi) or mobile apps, running the binary directly consumes far less RAM and CPU than running a container runtime.
3. **True Real-Time**: Native execution ensures the `Transit Lounge` ring buffer and `Dreamer` background threads aren't fighting the container daemon for CPU scheduling.

---

## 1. Deploying on Linux / Ubuntu / Debian Server

The most common way to deploy CLUAIZD on a production server.

### Build the Release Binary
```bash
# Clone the repository
git clone https://github.com/cluaiz/cluaizdb.git
cd cluaizdb

# Build the highly optimized release binary
cargo build --release -p cluaizdb-server
```

### Install as a Systemd Service
To ensure CLUAIZD runs in the background and restarts on failure, create a `systemd` service:

1. Copy the binary to a system path:
   ```bash
   sudo cp target/release/cluaizdb-server /usr/local/bin/cluaizdb
   ```
2. Create a data directory:
   ```bash
   sudo mkdir -p /var/lib/cluaizdb
   sudo chown $USER:$USER /var/lib/cluaizdb
   ```
3. Create the service file `/etc/systemd/system/cluaizdb.service`:
   ```ini
   [Unit]
   Description=CLUAIZD Nanosecond Database
   After=network.target

   [Service]
   Type=simple
   User=root
   WorkingDirectory=/var/lib/cluaizdb
   ExecStart=/usr/local/bin/cluaizdb
   Restart=always
   Environment="RUST_LOG=info"

   [Install]
   WantedBy=multi-user.target
   ```
4. Start the server:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable cluaizdb
   sudo systemctl start cluaizdb
   ```

You can now access the database at `http://localhost:7331`.

---

## 2. Deploying on Edge Devices (Raspberry Pi, Robotics)

For robotics and IoT devices, CLUAIZD is lightweight enough to run directly on ARM architecture.

1. Install the ARM rust toolchain on your build machine:
   ```bash
   rustup target add aarch64-unknown-linux-gnu
   ```
2. Cross-compile the binary:
   ```bash
   cargo build --release --target aarch64-unknown-linux-gnu
   ```
3. Transfer the binary to your Raspberry Pi via SCP and run it directly. It will comfortably run on devices with as little as 512MB RAM, automatically shifting to Warm/Cold storage if RAM fills up.

---

## 3. Embedding Locally (Laptops & Mobile Apps)

If you are building an AI agent or a desktop/mobile app and want to use CLUAIZD as your local knowledge engine:

- **Mac/Windows Laptops**: Simply compile with `cargo build --release` and spawn the `cluaizdb` binary as a child process from your main application (e.g., via Python's `subprocess` or Node.js `child_process`).
- **CLI Management**: You can manage the local instance using our CLI tools or directly via the `http://localhost:7331` endpoints. 
- **Data Location**: Set the `CLUAIZD_STORAGE_DATA_DIR` environment variable to a folder inside your app's `AppData` or `~/.config` directory to keep the local machine clean.
