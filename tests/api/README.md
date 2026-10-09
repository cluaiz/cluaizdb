# 🌍 API Contract & Regression Tests

## 🎯 Purpose
This workspace tests the stability of the public REST and gRPC/WebSocket APIs exposed by the Cluaizdb server. 

## 🧬 What we test here
1. **Schema Validation:** Ensure that API responses match the documented schemas exactly.
2. **Backwards Compatibility:** Ensure that updates to the database engine don't break existing API clients.
3. **Authentication/Security:** Test unauthorized access, rate limits, and permission boundaries.

## 🚀 How to Run
```bash
cargo test --package api-tests
```
