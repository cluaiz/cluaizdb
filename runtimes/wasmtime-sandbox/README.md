# 🧬 runtimes/wasmtime-sandbox: Fuel-Metered WASM Genome Runtime
> **Engine:** Wasmtime JIT (Cranelift)  
> **Isolation Target:** Hard Linear Memory Bounds + 50k Fuel Metering  
> **Concurrency Rule:** Linear Memory Staging Buffer (Zero LMDB Read/Write Deadlock)

---

## 🎯 1. Purpose
`runtimes/wasmtime-sandbox` executes user-supplied and agent-compiled WebAssembly Genomes within CluaizDB. Genomes represent programmable mutation triggers, validation gates, semantic filters, and embedding extractors.

---

## ⚡ 2. Core Safety Guarantees

### ⛽ 1. Fuel-Metered Execution (50,000 Instructions Cap)
- Every WASM execution runs on a strict instruction-fuel meter.
- Infinite loops (`loop {}`) or non-terminating algorithms are forcibly trapped with `SandboxError::OutOfFuel` before consuming host CPU threads.

### 🛡️ 2. Linear Memory Staging Buffer (Deadlock Elimination)
- In legacy architectures, user code running inside an LMDB read transaction could attempt to acquire write locks, causing irreversible deadlocks.
- In Kernel v2, Genomes operate strictly over isolated linear memory buffers:
  1. Host writes input record bytes into linear memory.
  2. Genome executes and writes output mutations to an isolated memory buffer.
  3. Host extracts outputs and stages them for `GroupCommit` after the read phase completes.
- **Reads Never Write** invariant is physically preserved.

---

## 🏛️ 3. Domain Boundaries (Khichdi-Zero)
### ✅ What this crate CAN do:
- JIT compile `.wasm` bytecode into native host machine code via Wasmtime.
- Enforce strict linear memory maximums (default: 16 MB).
- Enforce instruction fuel budgets.
- Expose pure computational host calls (e.g. hash computation).

### ❌ What this crate CANNOT do:
- Directly acquire LMDB write transactions (`RwTxn`).
- Perform host disk I/O or network socket syscalls (WASIp1 filesystem access is stripped/disabled).
- Retain global state across genome invocation lifecycles.
