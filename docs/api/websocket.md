# WebSocket Live Telemetry (`GET /ws/telemetry`)

> _"Your database has a heartbeat. Listen to it."_

## What is the HEART WebSocket?

CLUAIZD maps all its internal performance metrics to **biological biomarkers** through the `Cluaizdb-HEART` system. Instead of showing you boring CPU% graphs, it expresses database load as:

- **Heart Rate (BPM):** Maps to the number of active shards. More shards = faster heartbeat.
- **Blood Pressure:** Maps to write/read throughput pressure.
- **SpO2 (Oxygen Level):** Maps to database health — 100% = perfectly healthy, <95% = warning.
- **Metabolic Rate:** Maps to total active shard load factor.

This telemetry stream powers the **Genome Canvas GUI** health dashboard.

---

## Connecting via WebSocket

**Endpoint:** `ws://localhost:7331/ws/telemetry`

```javascript
// JavaScript / Node.js
const ws = new WebSocket("ws://localhost:7331/ws/telemetry");

ws.onmessage = (event) => {
	const telemetry = JSON.parse(event.data);
	console.log("Heart Rate:", telemetry.heart_rate_bpm, "BPM");
	console.log("SpO2:", telemetry.oxygen_level_spo2, "%");
	console.log("Metabolic Rate:", telemetry.metabolic_rate);
};
```

```python
# Python using websockets library
import asyncio
import websockets
import json

async def monitor():
    async with websockets.connect("ws://localhost:7331/ws/telemetry") as ws:
        async for message in ws:
            telemetry = json.loads(message)
            print(f"BPM: {telemetry['heart_rate_bpm']} | SpO2: {telemetry['oxygen_level_spo2']}%")

asyncio.run(monitor())
```

---

## Telemetry Message Format

The server sends a JSON message every **500ms**:

```json
{
	"heart_rate_bpm": 88,
	"blood_pressure_systolic": 122,
	"blood_pressure_diastolic": 81,
	"oxygen_level_spo2": 98.4,
	"metabolic_rate": 1.75
}
```

| Field                      | Type  | Maps To                                                          |
| -------------------------- | ----- | ---------------------------------------------------------------- |
| `heart_rate_bpm`           | `u32` | `72 + (open_shards × 8)`. 1 shard = 80 BPM, 10 shards = ~152 BPM |
| `blood_pressure_systolic`  | `u32` | Write pressure (fluctuates 100-150)                              |
| `blood_pressure_diastolic` | `u32` | Read pressure (fluctuates 60-95)                                 |
| `oxygen_level_spo2`        | `f32` | Database health. Below 95% = degraded state                      |
| `metabolic_rate`           | `f32` | `1.0 + (open_shards × 0.25)`. Reflects total shard load          |

---

## Sending Commands (Bidirectional Control)

The WebSocket is bidirectional. You can send control commands FROM the client TO the server:

### Command: `adrenaline_shot`

Triggers an emergency GC (Garbage Collection) sweep across all shards. Forces immediate Hot→Warm→Cold compression cycle.

```json
{ "command": "adrenaline_shot" }
```

```javascript
ws.send(JSON.stringify({ command: "adrenaline_shot" }));
```

### Command: `artificial_pacemaker`

Sets a hard write-rate limit (bytes per second). Use during runaway ingestion spikes.

```json
{ "command": "artificial_pacemaker", "payload": { "pulse_limit": 1048576 } }
```

Sets the limit to 1 MB/s.

### Command: `induced_coma`

Emergency: triggers an immediate WAL flush and database consistency verification. All writes are paused until the check completes.

```json
{ "command": "induced_coma" }
```

---

## Interpreting the Biomarkers

| Reading              | Meaning                           | Action                               |
| -------------------- | --------------------------------- | ------------------------------------ |
| BPM < 80             | System idle                       | Normal                               |
| BPM 80-120           | Moderate load                     | Normal                               |
| BPM > 150            | High shard count / heavy load     | Consider scaling or running GC       |
| SpO2 < 95%           | Database health degraded          | Run `adrenaline_shot`                |
| Metabolic > 2.5      | Extreme shard load                | Switch Booster to `performance` mode |
| Any sudden BPM spike | Shard opened or large write burst | Monitor for stabilization            |
