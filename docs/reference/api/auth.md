# `Authorization` Mechanics

The cluaizdb Engine does not enforce a rigid, legacy Username/Password table. Instead, authentication operates at the edge via cryptographic signatures, allowing it to easily integrate with modern Auth providers (OAuth, Auth0, custom JWTs).

## Architectural Execution

### 1. Ed25519 Public Key Verification
When a request hits the cluaizdb HTTP router, the Engine intercepts the `Authorization` header. If configured in `cluaizdb.toml`, the Engine loads a pre-defined Ed25519 or RSA public key into CPU cache. The incoming JWT (JSON Web Token) signature is verified mathematically before the payload is even allowed to enter the CDQL parser.

### 2. Zero-Copy Token Parsing
If the signature is valid, the Engine uses a zero-allocation parser to extract the `claims` (e.g., `"role": "admin"`). These claims are injected into the WASM `ctx` (Context Variable), making them instantly accessible to any `on_read` or `on_write` execution affordances for dynamic Row-Level Security (RLS).

## Syntax & Usage

All HTTP routes and WebSocket handshakes require the Bearer token in the headers.

```http
POST /neuron HTTP/1.1
Host: cluaizdb-engine.local:7331
Authorization: Bearer eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9...
```

## Performance Overhead

| Operation | Complexity | Notes |
| :--- | :--- | :--- |
| **Signature Validation** | **O(1)** | Highly optimized via cryptographic hardware acceleration (AES-NI / Cryptography extensions). Adds <1ms latency. |
