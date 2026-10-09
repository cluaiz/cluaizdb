# 🧪 Cluaizdb Unified Testing Framework

## 🎯 Architecture Philosophy
Welcome to the core testing directory of Cluaizdb. In accordance with professional Rust and database engineering standards, **all testing logic is strictly isolated from the main production code (`crates/`).** 

This directory is organized into specialized testing sub-workspaces, ensuring that everything from microsecond-level latency benchmarks to destructive fault-injection tests are beautifully separated.

## 📂 Structure Breakdown

### 1. `integration/`
- **Purpose:** Black-box HTTP and CDQL testing.
- **What it does:** Sends raw JSON and CDQL commands to a running Cluaizdb server instance to verify successful data ingestion, schema parsing, and Graph/SQL/Vector routing.

### 2. `chaos/`
- **Purpose:** Database resilience, ACID compliance, and Fault Injection (Jepsen-style).
- **What it does:** Runs tests that deliberately try to break the database: passing invalid payloads, launching 1,000 concurrent writes to trigger deadlocks, or simulating power failures to verify WAL (Write-Ahead Log) recovery.

### 3. `benchmarks/`
- **Purpose:** Performance regression tracking via Criterion.
- **What it does:** Measures the raw execution speed of the `genome` engine. Fails CI/CD if P99 read/write latency exceeds the 100µs / 500µs thresholds defined in our `production_readiness.md`.

### 4. `quality/`
- **Purpose:** Property-based testing, fuzzing, and internal mechanics validation.
- **What it does:** Focuses on the logical correctness of internal structures (e.g., throwing randomized strings at the CDQL parser to see if it panics).

### 5. `api/`
- **Purpose:** Public API contracts and regression.
- **What it does:** Validates that the REST endpoints return the exact expected JSON schema, and tests security boundaries/rate-limits.

### 6. `sdk/`
- **Purpose:** Client library bindings testing.
- **What it does:** Ensures that any external SDKs (like Rust or Python client libs) interact cleanly with the database without encoding errors.

### 7. `apps/`
- **Purpose:** External tools and CLI testing.
- **What it does:** Tests the `cluaizdb-cli` and GUI dashboard binaries. Checks if commands like `stats` or `backup` parse arguments and execute correctly.

### 8. `qa/`
- **Purpose:** Legacy scripts, human QA procedures, and experimental testing rigs.
- **What it does:** Currently archives the old `api_tests/` Python scripts (`legacy_python/`) for historical reference or external load testing.

### 9. `fuzz/`
- **Purpose:** Randomized input testing (`cargo-fuzz`).
- **What it does:** Bombards the CDQL parser and DNA execution engines with random bytes and garbage data to find panic-inducing edge cases that normal unit tests miss.

### 10. `migration/`
- **Purpose:** Schema upgrades and data migrations.
- **What it does:** Tests forward compatibility (upgrading older LMDB shard formats) and backward rollbacks to ensure zero data loss during Cluaizdb engine updates.

### 11. `replication/`
- **Purpose:** Distributed consistency and cluster testing.
- **What it does:** Simulates split-brain scenarios, network partitions, and verifies linearizability across multiple synced shards in a cluster environment.

## 🚀 Running the Tests
To run all Rust integration, chaos, and quality tests at once across the workspace:
```bash
cargo test --workspace
```
*(Ensure a local test instance of `cluaizdb` is running for integration tests that hit HTTP endpoints).*
