# 📊 Chapter 2: Understanding The Pulse (Cluaizdb-HEART)

Most databases fail silently. Under heavy load, memory spikes, swap space is consumed, and eventually, the OS kills the database process with an OOM (Out of Memory) error. 

**Cluaizdb Neural System Database is alive.** It has a heartbeat, blood pressure, and oxygen levels. We call this the `Cluaizdb-HEART` Autonomic Subsystem.

## The 3 Biomarkers of Your Database

Inside `crates/heart`, Cluaizdb constantly monitors the host operating system using the `sysinfo` crate. It translates hardware metrics into biological equivalents:

1. **Blood Pressure (BP)** = CPU Load (%)
   - If your server's CPU hits 90%, the database's BP is critically high.
2. **Oxygen Level (SpO2)** = Available RAM (%)
   - If your RAM usage goes over 95%, your database is "choking" (SpO2 drops below 10%).
3. **Heart Rate (BPM)** = Active API Requests / Shard Activity
   - The more data you pump into the API, the higher the heart rate.

## The Zero "Stop-the-World" Approach

In Java (Elasticsearch) or Go, Garbage Collection pauses the entire application ("Stop the World"). This causes massive latency spikes for your users.

**How Cluaizdb fixes this:**
When `Cluaizdb-HEART` detects that Blood Pressure > 90% or SpO2 < 10%, it does **NOT** crash. It does **NOT** pause.
Instead, it kicks in a `delay_ms` throttle to the background "Dreaming Engine" (our biological Garbage Collector and Edge Forger). 

```rust
// Inside the Dreaming Engine (dreamer.rs)
if bp > 90 || spo2 < 10 { 
    throttle_delay = 500; // Slow down background tasks by 500ms
}
```

By adding a microsecond delay to background tasks, it instantly frees up CPU cycles for the main Network API, allowing your user-facing queries to remain blazing fast. As the CPU cools down, the Blood Pressure drops, and the delay is automatically lifted.

## Testing it Yourself

1. Boot the server.
2. Connect a WebSocket client to `ws://localhost:7331/ws/telemetry`.
3. You will see a live JSON stream of `heart_rate_bpm`, `blood_pressure_systolic`, and `oxygen_level_spo2`.
4. Open a stress-test tool (like `stress` on Linux or open many heavy apps on Windows) to intentionally spike your CPU.
5. Watch the WebSocket stream. You will see the Blood Pressure instantly rise to 90+, and the backend terminal will print a Warning: *"System Booster: Resources under stress. Dreaming Engine throttling applied."*

Your database just saved its own life without human intervention!
