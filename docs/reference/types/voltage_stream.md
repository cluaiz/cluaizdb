# `voltage_stream` Data Type

The `voltage_stream` format is a specialized IoT datatype in Cluaizdb.

## Architectural Storage

### Binary Contiguous Float Streams
Rather than stringifying arrays of readings, `voltage_stream` stores contiguous arrays of raw `f32` sensor measurements. The time-series engine performs zero-copy window aggregations across these streams.

## Use Cases
- High-frequency battery telemetry, voltage readings, and vibration logs in edge robotics.
