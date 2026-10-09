# 🌪️ Chaos Engineering & ACID Tests

## 🎯 Purpose
This workspace focuses on **Fault Injection and Concurrency Stress**. It proves that Cluaizdb is robust against:
- Data corruption
- Hardware failure
- Concurrent read/write race conditions (Deadlocks)

## 🧬 What we test here
1. **Atomicity:** Do failed transactions roll back cleanly?
2. **Consistency:** Does the system strongly reject invalid schemas/vectors?
3. **Isolation:** Can 1,000 concurrent threads write to the LMDB instance safely?
4. **Durability:** If the process is `SIGKILL`'d, does the WAL successfully replay the data on reboot?

## 🚀 How to Run
```bash
cargo test --package chaos-tests
```
