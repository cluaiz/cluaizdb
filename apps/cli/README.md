# 💻 apps/cli: CluaizDB Kernel v2 Administration CLI
> **Binary:** `cluaizdb` / `cluaizdb.exe`  
> **Recall Target:** `< 5 µs` Point Recall & Physical SSD Durability  
> **Interface:** Terminal Commands & Structured `--json` Pipeline Mode

---

## 🎯 1. Purpose
`apps/cli` is the official administrative command-line interface for CluaizDB Kernel v2. It provides zero-overhead, headless database control directly linking to `cluaizdb-substrate`, `cluaizdb-group-commit`, and `cluaizdb-mcp`.

---

## 🛠️ 2. Core Commands

```bash
# 1. Start Server Daemon (REST + MCP)
cluaizdb serve --port 8080 --data-dir ./data/cluaizdb

# 2. Store Record (Durable Group Commit ~121 µs)
cluaizdb put --col 1 --id 42 --payload '{"status":"active"}'

# 3. Retrieve Record (< 5 µs Memory Page-Cache Recall)
cluaizdb get --col 1 --id 42

# 4. Secondary Index Term
cluaizdb index-term --col 1 --term "user:alice" --id 42

# 5. Seek Term Index (< 5 µs)
cluaizdb seek-term --col 1 --term "user:alice"

# 6. Delete Record
cluaizdb delete --col 1 --id 42

# 7. Model Context Protocol (MCP) Inspection
cluaizdb mcp-list
cluaizdb mcp-call --tool db_inspect_node --args '{"term":"user:alice"}'
```

---

## ⚙️ 3. Global Flags
- `--data-dir <path>`: Physical LMDB storage directory (default: `./data/cluaizdb`).
- `--map-size <bytes>`: Maximum virtual memory map size (default: 10 GB).
- `--json`: Format output as machine-readable JSON for CI/CD scripting.
