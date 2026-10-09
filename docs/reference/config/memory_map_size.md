# `memory_map_size` Configuration

The `memory_map_size` parameter in `cluaizdb.toml` dictates the maximum virtual address space the LMDB engine is allowed to reserve from the Operating System.

## Architectural Implication

### Virtual vs Physical Memory
Setting `memory_map_size = "100GB"` **does not** mean the engine will instantly consume 100GB of physical RAM. It merely reserves that much Virtual Memory Address (VMA) space via `mmap()`. The actual physical RAM usage (Resident Set Size - RSS) will grow dynamically as records are inserted and accessed, constrained only by the OS Page Cache limits.

### Resizing Penalty
If the database file grows beyond the allocated `memory_map_size`, the Engine will encounter a fatal `MDB_MAP_FULL` exception. To recover, the Engine must be rebooted with a larger configured size. Because virtual memory allocation is nearly "free" on 64-bit operating systems, it is highly recommended to set this value significantly higher than your anticipated data footprint.

## Configuration Syntax

Found in `cluaizdb.toml` under the `[storage]` section:

```toml
[storage]
# Recommended: 10GB for testing, 1TB for production
memory_map_size = "1TB"
```

## OS Compatibility

- **Linux / macOS (64-bit)**: Can safely support terabytes of virtual memory allocation regardless of physical RAM.
- **Windows**: Requires strict contiguous memory blocks. Setting excessively high values (e.g., 10TB) on heavily fragmented Windows servers may cause the initial `mmap` allocation to fail on boot.
