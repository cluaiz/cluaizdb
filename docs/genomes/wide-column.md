# Wide-Column Genome (`sensory_stream.json`)

> *"Write fast. Write forever. Never look back."*

## When to Use This Genome
Use the `sensory_stream` genome when:
- Your data is **append-only** — records are never updated, only added.
- Write throughput is extreme (BCI signals, robot joint encoders, financial tick feeds).
- Data is immutable by contract (audit logs, blockchain-adjacent records, sensor archives).
- You need to enforce that NO modifications can be made to existing records.

Real-world use cases: Brain-Computer Interface (BCI) neural signal capture, robotic motor encoder streams, stock market tick feeds, server access logs, immutable audit trails.

---

## The Core Principle: Append-Only Immutability

Wide-column databases like Wide-Column DB achieve extreme write speeds because they never update records — they only append new ones. Updates are achieved by writing a new record with a newer timestamp. The "latest" record wins during reads.

CLUAIZD's `sensory_stream` genome enforces this constraint at the DNA level:

### The `sensory_stream.json` Genome
```json
{
  "on_write": "let res = #{ action: \"Allow\" };\nif payload.is_update {\n    res.action = \"Abort\";\n    res.error = \"Sensory streams are append-only. Create a new record instead.\"\n}\nif !payload.contains(\"timestamp\") {\n    res.action = \"Abort\";\n    res.error = \"All sensory records must include a timestamp.\"\n}\nres",
  "engine": "rhai"
}
```

Any write that includes `"is_update": true` is **blocked**. This prevents accidental mutations of immutable sensor logs.

---

## BCI (Brain-Computer Interface) Use Case

A BCI device records 1000 neural samples per second per electrode, across 256 electrodes. That is **256,000 writes/second** — a throughput that would destroy a standard Relational DB setup.

CLUAIZD handles this through:
1. **Sensory Shard Isolation:** BCI traffic is routed to `?tenant_id=sensory_tissue` — a completely separate LMDB memory-map that does not block the main cortical database.
2. **C-FFI Direct Writes:** The BCI device's C++ firmware writes directly via the C-FFI, bypassing HTTP/TCP entirely.
3. **Sequential Append:** Because LMDB's B-tree allows sequential appends without lock contention, write speeds remain high.

```c
// BCI C++ firmware writing to CLUAIZD at 256,000 writes/second
#include "cluaizdb.h"

cluaizdbdHandle* handle = cluaizdb_open("./out/sensory_tissue", 8192);

while (recording) {
    char payload[256];
    snprintf(payload, sizeof(payload),
        "{\"electrode\": %d, \"value\": %.4f, \"ts\": %lld}",
        electrode_id, sample_voltage, timestamp_ns);
    
    cluaizdb_write(handle, "bci_stream", payload);
}
```

---

## Ordered Range Scans (Wide-Column DB CDQL Equivalent)

After data is captured, you can replay any time range:

```text
// Replay all electrode 42 readings between two timestamps
find BCIRecord(electrode: 42)
  -> range_scan(field: "timestamp", start: 1717789200, end: 1717789260)
  -> limit 60000
```

---

## Immutable Audit Logs (Financial / Legal Use Case)

For financial systems requiring tamper-proof logs:

```bash
curl -X POST "http://localhost:7331/neuron?tenant_id=audit_ledger" \
  -d '{
    "id": "tx_log_00019234",
    "tier": "Hot",
    "raw_payload": [bytes for {"action": "FUNDS_TRANSFER", "amount": 50000, "from": "acc_A", "to": "acc_B", "ts": 1717789200}]
  }'
```

Any attempt to modify this record is blocked by the `sensory_stream` genome's append-only enforcement.

---

## Comparison: CLUAIZD vs Wide-Column DB

| Feature | Wide-Column DB | CLUAIZD (sensory_stream) |
|---|---|---|
| Append-Only Enforcement | ⚠️ (convention only) | ✅ (DNA-enforced) |
| Partition Key Routing | ✅ | ✅ (via tenant_id sharding) |
| CDQL Range Scans | ✅ | ✅ (via CDQL range_scan) |
| JVM Dependency | ✅ (requires JVM) | ❌ (pure Rust) |
| Vector Search on Stream | ❌ | ✅ |
| Graph on Stream Records | ❌ | ✅ |
