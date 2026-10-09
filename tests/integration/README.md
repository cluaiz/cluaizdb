# 🔌 Integration & End-to-End Tests

## 🎯 Purpose
This workspace treats the Cluaizdb database as a black box. It interacts purely via the external HTTP API to ensure that end-users receive expected results.

## 🧬 What we test here
1. **HTTP Endpoints:** `POST /neuron` and `POST /query`.
2. **CDQL Router:** Verifies that string CDQL commands correctly map to Relational, Graph, Vector, and Document paradigms without backend crashes.

## 🚀 How to Run
```bash
cargo test --package integration-tests
```
