# 🌐 api: Shared Protocol & Boundary Specifications
> **Layer:** System APIs, Wire Protocols, and Integration Interfaces  
> **Sub-Modules:**  
> - `mcp-spec/`: Model Context Protocol (MCP) Server for Claude & Agentic Runtimes  
> - `wasm-abi/`: Host-Guest linear memory definitions and staging buffers

---

## 🎯 Purpose
The `api/` directory declares strict type-first protocols that cross system boundaries. Neither Substrate storage nor runtime hosts implement business serialization; all RPC interfaces, tool schemas, and FFI buffers are governed here.

---

## 🏛️ Domain Boundaries (Khichdi-Zero)
- **CAN:** Define JSON-RPC 2.0 schemas, MCP tool definitions, guest buffer layouts, and zero-copy slices.
- **CANNOT:** Perform direct LMDB disk operations or spawn persistent server worker threads.
