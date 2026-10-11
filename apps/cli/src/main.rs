use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use cluaizdb_cdql::{build_plan, execute_plan, parse, QueryContext};
use cluaizdb_group_commit::{GroupCommitConfig, GroupCommitter};
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb_mcp::{JsonRpcRequest, McpHandler};
use cluaizdb_server_http::{create_router, AppState};
use cluaizdb_substrate::{
    EdgeMetadata, StorageSubstrate, SubstrateEnv, TraversalConfig, TraversalDirection,
};
use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};

#[derive(Parser)]
#[command(
    name = "cluaizdb",
    about = "CluaizDB 0.1.0 Alpha CLI — Sovereign Market-Standard Database Substrate",
    version = "0.1.0"
)]
struct Cli {
    /// Path to physical database directory hosting data.mdb and lock.mdb
    #[arg(short, long, default_value = "./data/cluaizdb", global = true)]
    data_dir: PathBuf,

    /// Maximum virtual memory-map allocation in bytes (default: 10 GB)
    #[arg(long, default_value_t = 10 * 1024 * 1024 * 1024, global = true)]
    map_size: usize,

    /// Format output as machine-readable JSON for tooling and CI/CD scripts
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the sovereign CluaizDB server daemon (Axum REST & MCP)
    Serve {
        #[arg(short, long, default_value = "0.0.0.0")]
        host: String,

        #[arg(short, long, default_value_t = 7331)]
        port: u16,
    },

    /// Launch interactive CDQL query shell and admin console (REPL)
    Repl {
        #[arg(short, long, default_value_t = 1)]
        col: u32,
    },

    /// Execute a one-shot CDQL pipeline query
    Query {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        query: String,
    },

    /// Durably store a record into Substrate using batched group-commit
    Put {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        payload: String,
    },

    /// Retrieve a record by ID directly from memory-mapped page-cache
    Get {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,
    },

    /// Delete a record by ID from document and primary index tables
    Delete {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,
    },

    /// Scan sequential records from a collection
    Scan {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },

    /// Index a search term into composite secondary index
    IndexTerm {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        term: String,

        #[arg(short, long)]
        id: String,
    },

    /// Seek record IDs matching an indexed term via prefix scan
    SeekTerm {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        term: String,
    },

    /// Add a directional, weighted graph edge between two record nodes
    EdgeAdd {
        #[arg(long)]
        from: String,

        #[arg(long, default_value_t = 1)]
        type_id: u32,

        #[arg(long)]
        to: String,

        #[arg(long, default_value_t = 1.0)]
        weight: f32,
    },

    /// Delete a directional graph edge between two record nodes
    EdgeDel {
        #[arg(long)]
        from: String,

        #[arg(long, default_value_t = 1)]
        type_id: u32,

        #[arg(long)]
        to: String,
    },

    /// Trace multi-hop graph traversal from a source node via BFS
    EdgeTrace {
        #[arg(long)]
        from: String,

        #[arg(long, default_value_t = 3)]
        max_depth: usize,

        #[arg(long, default_value_t = 0.0)]
        min_weight: f32,

        #[arg(long, default_value_t = 50)]
        limit: usize,
    },

    /// Insert dense embedding vector for a record node
    VecInsert {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,

        /// Float vector formatted as JSON array (e.g. "[0.12, 0.45, -0.67]")
        #[arg(short, long)]
        vector: String,
    },

    /// Perform Approximate Nearest Neighbor (ANN) search across vectors
    VecSearch {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        /// Float query vector as JSON array (e.g. "[0.12, 0.45, -0.67]")
        #[arg(short, long)]
        vector: String,

        #[arg(short, long, default_value_t = 5)]
        top_k: usize,
    },

    /// Bulk import JSONL records into a collection via GroupCommitter
    Import {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        file: PathBuf,
    },

    /// Export records from a collection to a JSONL file
    Export {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        file: PathBuf,

        #[arg(short, long, default_value_t = 100000)]
        limit: usize,
    },

    /// Inspect database telemetry, disk allocation, and active collections
    Stats,

    /// Execute in-process hardware latency profiler (reads & writes)
    Bench {
        #[arg(short, long, default_value_t = 1000)]
        iterations: usize,
    },

    /// List all native Model Context Protocol (MCP) tools
    McpList,

    /// Execute an MCP tool call directly from CLI
    McpCall {
        #[arg(short, long)]
        tool: String,

        #[arg(short, long, default_value = "{}")]
        args: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    std::fs::create_dir_all(&cli.data_dir)
        .with_context(|| format!("Failed to create data directory {:?}", cli.data_dir))?;

    match &cli.command {
        Commands::Serve { host, port } => handle_serve(&cli, host, *port).await?,
        Commands::Repl { col } => handle_repl(&cli, *col).await?,
        Commands::Query { col, query } => handle_query(&cli, *col, query)?,
        Commands::Put { col, id, payload } => handle_put(&cli, *col, id, payload).await?,
        Commands::Get { col, id } => handle_get(&cli, *col, id)?,
        Commands::Delete { col, id } => handle_delete(&cli, *col, id)?,
        Commands::Scan { col, limit } => handle_scan(&cli, *col, *limit)?,
        Commands::IndexTerm { col, term, id } => handle_index_term(&cli, *col, term, id)?,
        Commands::SeekTerm { col, term } => handle_seek_term(&cli, *col, term)?,
        Commands::EdgeAdd {
            from,
            type_id,
            to,
            weight,
        } => handle_edge_add(&cli, from, *type_id, to, *weight)?,
        Commands::EdgeDel { from, type_id, to } => handle_edge_del(&cli, from, *type_id, to)?,
        Commands::EdgeTrace {
            from,
            max_depth,
            min_weight,
            limit,
        } => handle_edge_trace(&cli, from, *max_depth, *min_weight, *limit)?,
        Commands::VecInsert { col, id, vector } => handle_vec_insert(&cli, *col, id, vector)?,
        Commands::VecSearch { col, vector, top_k } => {
            handle_vec_search(&cli, *col, vector, *top_k)?
        }
        Commands::Import { col, file } => handle_import(&cli, *col, file).await?,
        Commands::Export { col, file, limit } => handle_export(&cli, *col, file, *limit)?,
        Commands::Stats => handle_stats(&cli)?,
        Commands::Bench { iterations } => handle_bench(&cli, *iterations).await?,
        Commands::McpList => handle_mcp_list(&cli).await?,
        Commands::McpCall { tool, args } => handle_mcp_call(&cli, tool, args).await?,
    }

    Ok(())
}

// ── Command Handlers ────────────────────────────────────────────────────────

async fn handle_serve(cli: &Cli, host: &str, port: u16) -> Result<()> {
    println!("🚀 Starting CluaizDB 0.1.0 Alpha Server on {}:{}", host, port);
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
    let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
    let wasm_runner = Arc::new(WasmGenomeRunner::new(SandboxConfig::default())?);

    let state = Arc::new(AppState::new(db, committer, vector_index, wasm_runner));
    let router = create_router(state);

    let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "❌ Fatal: Failed to bind to port {}: {}. Auto-port hopping is disabled.",
                port, e
            );
            return Err(e.into());
        }
    };
    println!("📡 Listening at http://{}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn handle_repl(cli: &Cli, initial_col: u32) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let mut current_col = initial_col;

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║               CluaizDB Interactive CDQL Shell (v0.1.0)           ║");
    println!("║   Type CDQL queries or dot-commands:                             ║");
    println!("║   .help             - Show available shell commands              ║");
    println!("║   .stats            - View database & collection telemetry       ║");
    println!("║   .use <col_id>     - Switch active collection                   ║");
    println!("║   .collections      - List registered collections                ║");
    println!("║   .exit / .quit     - Exit shell                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    loop {
        print!("cluaizdb [col:{}]> ", current_col);
        stdout.flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == ".exit" || trimmed == ".quit" {
            println!("👋 Goodbye.");
            break;
        } else if trimmed == ".help" {
            println!("Commands:");
            println!("  .use <col_id>      Switch active collection (e.g. .use 2)");
            println!("  .collections       List collections registered in Substrate");
            println!("  .stats             View database size and active record counts");
            println!("  .exit / .quit      Exit interactive shell");
            println!("  <CDQL Query>       Execute pipeline (e.g. find status == 'active')");
            continue;
        } else if trimmed == ".collections" {
            let cols = db.list_collections()?;
            println!("Active Collections ({}):", cols.len());
            for (name, id) in cols {
                println!("  • ID: {} | Name: {}", id, name);
            }
            continue;
        } else if trimmed == ".stats" {
            let records = db.scan_records(current_col, 100000)?.len();
            println!("Collection {} Stats: {} record(s) indexed.", current_col, records);
            continue;
        } else if trimmed.starts_with(".use ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(c) = parts[1].parse::<u32>() {
                    current_col = c;
                    println!("Switched to collection {}", current_col);
                    continue;
                }
            }
            println!("Usage: .use <col_id>");
            continue;
        }

        // Execute as CDQL query
        let start = Instant::now();
        match parse(trimmed) {
            Ok(ast) => match build_plan(&ast) {
                Ok(plan) => {
                    let ctx = QueryContext::new(&db, current_col);
                    match execute_plan(&plan, &ctx) {
                        Ok(res) => {
                            let dur_us = start.elapsed().as_micros();
                            println!(
                                "⚡ [Executed in {} µs] Matched {} record(s):",
                                dur_us, res.count
                            );
                            for (idx, rec) in res.records.iter().enumerate() {
                                println!("  [{}] {}", idx + 1, serde_json::to_string(rec)?);
                            }
                        }
                        Err(e) => eprintln!("❌ Execution Error: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Planning Error: {}", e),
            },
            Err(e) => eprintln!("❌ Syntax Error: {}", e),
        }
    }

    Ok(())
}

fn handle_query(cli: &Cli, col: u32, query: &str) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let ast = parse(query).map_err(|e| anyhow::anyhow!("CDQL syntax error: {}", e))?;
    let plan = build_plan(&ast).map_err(|e| anyhow::anyhow!("CDQL plan error: {}", e))?;
    let ctx = QueryContext::new(&db, col);
    let result = execute_plan(&plan, &ctx).map_err(|e| anyhow::anyhow!("CDQL execution failed: {}", e))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "⚡ [CDQL Execution in {} µs] Matched {} record(s):",
            result.execution_time_us, result.count
        );
        for (idx, rec) in result.records.iter().enumerate() {
            println!("[{}] {}", idx + 1, serde_json::to_string(rec)?);
        }
    }
    Ok(())
}

async fn handle_put(cli: &Cli, col: u32, id: &str, payload: &str) -> Result<()> {
    let record_id: u128 = id.parse().context("Invalid record_id: must be 128-bit decimal")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let committer = GroupCommitter::new(db, GroupCommitConfig::default());

    committer.write(col, record_id, payload.as_bytes().to_vec()).await?;

    if cli.json {
        println!("{}", serde_json::json!({ "status": "committed", "col": col, "id": id }));
    } else {
        println!("✅ [OK] Record {} durably written to collection {}", id, col);
    }
    Ok(())
}

fn handle_get(cli: &Cli, col: u32, id: &str) -> Result<()> {
    let record_id: u128 = id.parse().context("Invalid record_id: must be 128-bit decimal")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

    match db.get_record(col, record_id)? {
        Some(bytes) => {
            let payload_str = String::from_utf8(bytes).unwrap_or_else(|_| "<binary>".to_string());
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({ "found": true, "col": col, "id": id, "payload": payload_str })
                );
            } else {
                println!("⚡ Record {}:", id);
                println!("{}", payload_str);
            }
        }
        None => {
            if cli.json {
                println!("{}", serde_json::json!({ "found": false, "id": id }));
            } else {
                println!("❌ Record {} not found", id);
            }
        }
    }
    Ok(())
}

fn handle_delete(cli: &Cli, col: u32, id: &str) -> Result<()> {
    let record_id: u128 = id.parse().context("Invalid record_id")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let deleted = db.delete_record(col, record_id)?;

    if cli.json {
        println!("{}", serde_json::json!({ "deleted": deleted, "id": id }));
    } else {
        if deleted {
            println!("🗑️ [OK] Deleted record {}", id);
        } else {
            println!("❌ Record {} not found", id);
        }
    }
    Ok(())
}

fn handle_scan(cli: &Cli, col: u32, limit: usize) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let records = db.scan_records(col, limit)?;

    if cli.json {
        let items: Vec<serde_json::Value> = records
            .iter()
            .map(|(id, bytes)| {
                let s = String::from_utf8_lossy(bytes);
                serde_json::json!({ "id": id.to_string(), "payload": s })
            })
            .collect();
        println!("{}", serde_json::json!({ "count": items.len(), "records": items }));
    } else {
        println!("🗂️ Scanned {} record(s) from collection {}:", records.len(), col);
        for (id, bytes) in records {
            let payload = String::from_utf8_lossy(&bytes);
            println!("  • ID {}: {}", id, payload);
        }
    }
    Ok(())
}

fn handle_index_term(cli: &Cli, col: u32, term: &str, id: &str) -> Result<()> {
    let record_id: u128 = id.parse().context("Invalid record_id")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    db.index_term(col, term.as_bytes(), record_id)?;

    if cli.json {
        println!("{}", serde_json::json!({ "status": "indexed", "term": term, "id": id }));
    } else {
        println!("🔍 [OK] Indexed term '{}' -> {}", term, id);
    }
    Ok(())
}

fn handle_seek_term(cli: &Cli, col: u32, term: &str) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let results = db.seek_index(col, term.as_bytes())?;

    if cli.json {
        let id_strings: Vec<String> = results.iter().map(|id| id.to_string()).collect();
        println!("{}", serde_json::json!({ "term": term, "matches": id_strings }));
    } else {
        println!("🔍 Term '{}' matched {} record(s):", term, results.len());
        for id in results {
            println!("   • {}", id);
        }
    }
    Ok(())
}

fn handle_edge_add(cli: &Cli, from: &str, type_id: u32, to: &str, weight: f32) -> Result<()> {
    let src_id: u128 = from.parse().context("Invalid 'from' ID")?;
    let dst_id: u128 = to.parse().context("Invalid 'to' ID")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let meta = EdgeMetadata {
        weight,
        created_at: now,
        valid_until: 0,
    };

    db.add_edge(src_id, type_id, dst_id, meta)?;

    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "status": "edge_added", "from": from, "type_id": type_id, "to": to, "weight": weight })
        );
    } else {
        println!("🔗 [OK] Added Edge: {} --(type:{}, w:{:.2})--> {}", from, type_id, weight, to);
    }
    Ok(())
}

fn handle_edge_del(cli: &Cli, from: &str, type_id: u32, to: &str) -> Result<()> {
    let src_id: u128 = from.parse().context("Invalid 'from' ID")?;
    let dst_id: u128 = to.parse().context("Invalid 'to' ID")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

    let deleted = db.delete_edge(src_id, type_id, dst_id)?;

    if cli.json {
        println!("{}", serde_json::json!({ "deleted": deleted, "from": from, "to": to }));
    } else {
        if deleted {
            println!("🗑️ [OK] Deleted Edge: {} ----> {}", from, to);
        } else {
            println!("❌ Edge not found");
        }
    }
    Ok(())
}

fn handle_edge_trace(
    cli: &Cli,
    from: &str,
    max_depth: usize,
    min_weight: f32,
    limit: usize,
) -> Result<()> {
    let src_id: u128 = from.parse().context("Invalid 'from' ID")?;
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

    let config = TraversalConfig {
        max_depth,
        min_weight,
        limit,
        edge_type: None,
        direction: TraversalDirection::Outbound,
    };

    let nodes = db.traverse_graph(src_id, &config)?;

    if cli.json {
        let items: Vec<serde_json::Value> = nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "node_id": n.node_id.to_string(),
                    "depth": n.depth,
                    "cumulative_weight": n.cumulative_weight
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "source": from, "count": items.len(), "nodes": items }));
    } else {
        println!("🕸️ Traversed graph from {} (Found {} nodes):", from, nodes.len());
        for n in nodes {
            println!(
                "   [Depth {}] Node: {} (Weight: {:.2})",
                n.depth, n.node_id, n.cumulative_weight
            );
        }
    }
    Ok(())
}

fn handle_vec_insert(cli: &Cli, col: u32, id: &str, vector_json: &str) -> Result<()> {
    let record_id: u128 = id.parse().context("Invalid record_id")?;
    let vector: Vec<f32> = serde_json::from_str(vector_json)
        .context("Vector must be a valid JSON array of floats (e.g. '[0.1, 0.2]')")?;

    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    db.put_vector(col, record_id, &vector)?;

    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "status": "vector_inserted", "col": col, "id": id, "dimensions": vector.len() })
        );
    } else {
        println!(
            "📐 [OK] Stored vector for Record {} in collection {} (Dimensions: {})",
            id,
            col,
            vector.len()
        );
    }
    Ok(())
}

fn handle_vec_search(cli: &Cli, col: u32, query_json: &str, top_k: usize) -> Result<()> {
    let query: Vec<f32> = serde_json::from_str(query_json)
        .context("Query vector must be a valid JSON array of floats")?;

    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let vectors = db.scan_vectors(col, 50000)?;

    // Build in-process HNSW index for similarity lookup
    let index = HnswIndex::<CosineDistance>::new();
    for (id, vec) in vectors {
        index.insert(id, vec);
    }

    let results = index.search_knn(&query, top_k);

    if cli.json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|(id, dist)| {
                serde_json::json!({ "id": id.to_string(), "distance": dist })
            })
            .collect();
        println!("{}", serde_json::json!({ "col": col, "top_k": top_k, "results": items }));
    } else {
        println!("📐 Vector ANN Search (Top-{} Results):", results.len());
        for (rank, (id, dist)) in results.iter().enumerate() {
            println!("   [{}] Record ID: {} | Distance: {:.4}", rank + 1, id, dist);
        }
    }
    Ok(())
}

async fn handle_import(cli: &Cli, col: u32, file: &Path) -> Result<()> {
    let f = std::fs::File::open(file)
        .with_context(|| format!("Failed to open import file {:?}", file))?;
    let reader = std::io::BufReader::new(f);

    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let committer = GroupCommitter::new(db, GroupCommitConfig::default());

    let mut imported = 0usize;
    let start = Instant::now();

    for (line_num, line_res) in reader.lines().enumerate() {
        let line = line_res?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Expected format: {"id": 123, ...} or raw JSON with generated UUID
        let val: serde_json::Value = serde_json::from_str(trimmed)
            .with_context(|| format!("Malformed JSON at line {}", line_num + 1))?;

        let id = if let Some(id_val) = val.get("id") {
            if let Some(n) = id_val.as_u64() {
                n as u128
            } else if let Some(s) = id_val.as_str() {
                s.parse::<u128>().unwrap_or((line_num + 1) as u128)
            } else {
                (line_num + 1) as u128
            }
        } else {
            (line_num + 1) as u128
        };

        committer.write(col, id, trimmed.as_bytes().to_vec()).await?;
        imported += 1;
    }

    let elapsed = start.elapsed();
    let ips = if elapsed.as_secs_f64() > 0.0 {
        imported as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "status": "imported", "count": imported, "elapsed_ms": elapsed.as_millis(), "ops_per_sec": ips })
        );
    } else {
        println!(
            "📥 [OK] Imported {} record(s) into collection {} in {:.2?} ({:.0} records/sec)",
            imported, col, elapsed, ips
        );
    }
    Ok(())
}

fn handle_export(cli: &Cli, col: u32, file: &Path, limit: usize) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let records = db.scan_records(col, limit)?;

    let mut out = std::fs::File::create(file)
        .with_context(|| format!("Failed to create export file {:?}", file))?;

    for (_id, bytes) in &records {
        let line_content = String::from_utf8_lossy(bytes);
        writeln!(out, "{}", line_content)?;
    }

    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "status": "exported", "col": col, "count": records.len(), "file": file.to_string_lossy() })
        );
    } else {
        println!("📤 [OK] Exported {} record(s) to {:?}", records.len(), file);
    }
    Ok(())
}

fn handle_stats(cli: &Cli) -> Result<()> {
    let db_path = &cli.data_dir;
    let data_file = db_path.join("data.mdb");
    let lock_file = db_path.join("lock.mdb");

    let data_size = std::fs::metadata(&data_file).map(|m| m.len()).unwrap_or(0);
    let lock_size = std::fs::metadata(&lock_file).map(|m| m.len()).unwrap_or(0);

    let db = SubstrateEnv::open(db_path, cli.map_size)?;
    let collections = db.list_collections()?;

    let mut col_stats = Vec::new();
    let mut total_records = 0usize;

    for (name, id) in &collections {
        let count = db.scan_records(*id, 1000000)?.len();
        total_records += count;
        col_stats.push(serde_json::json!({
            "id": id,
            "name": name,
            "record_count": count
        }));
    }

    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "data_dir": db_path.to_string_lossy(),
                "data_mdb_bytes": data_size,
                "lock_mdb_bytes": lock_size,
                "configured_map_size_bytes": cli.map_size,
                "total_collections": collections.len(),
                "total_records": total_records,
                "collections": col_stats
            })
        );
    } else {
        println!("📊 ─── CluaizDB Telemetry & Statistics ───");
        println!("  • Data Directory   : {:?}", db_path);
        println!(
            "  • Disk Footprint   : data.mdb ({:.2} MB) | lock.mdb ({:.2} KB)",
            data_size as f64 / (1024.0 * 1024.0),
            lock_size as f64 / 1024.0
        );
        println!(
            "  • Virtual Map Size : {:.2} GB configured",
            cli.map_size as f64 / (1024.0 * 1024.0 * 1024.0)
        );
        println!("  • Active DBIs      : 9 Dedicated Tables (B+Tree Substrate)");
        println!("  • Collections ({}):", collections.len());
        for (name, id) in &collections {
            let count = db.scan_records(*id, 1000000)?.len();
            println!("      - Col {} ('{}') : {} record(s)", id, name, count);
        }
        println!("  • Total Records    : {}", total_records);
    }
    Ok(())
}

async fn handle_bench(cli: &Cli, iterations: usize) -> Result<()> {
    println!("🧪 Starting In-Process Hardware Latency Benchmark ({} ops)...", iterations);
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

    let bench_col = 9999u32;
    let mut write_latencies = Vec::with_capacity(iterations);
    let mut read_latencies = Vec::with_capacity(iterations);

    let payload = b"{\"bench_key\":\"bench_value_payload_for_hardware_calibration\"}";

    // 1. Measure Point Writes
    let write_start = Instant::now();
    for i in 0..iterations {
        let t0 = Instant::now();
        db.put_record(bench_col, i as u128, payload)?;
        write_latencies.push(t0.elapsed().as_nanos());
    }
    let total_write_time = write_start.elapsed();

    // 2. Measure Point Reads (LMDB Memory-Mapped Page Cache)
    let read_start = Instant::now();
    for i in 0..iterations {
        let t0 = Instant::now();
        let _ = db.get_record(bench_col, i as u128)?;
        read_latencies.push(t0.elapsed().as_nanos());
    }
    let total_read_time = read_start.elapsed();

    // Cleanup benchmark records
    for i in 0..iterations {
        let _ = db.delete_record(bench_col, i as u128);
    }

    write_latencies.sort_unstable();
    read_latencies.sort_unstable();

    let p50_write_us = write_latencies[iterations / 2] as f64 / 1000.0;
    let p99_write_us = write_latencies[(iterations * 99) / 100] as f64 / 1000.0;
    let p50_read_us = read_latencies[iterations / 2] as f64 / 1000.0;
    let p99_read_us = read_latencies[(iterations * 99) / 100] as f64 / 1000.0;

    let write_throughput = iterations as f64 / total_write_time.as_secs_f64();
    let read_throughput = iterations as f64 / total_read_time.as_secs_f64();

    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "iterations": iterations,
                "read_p50_us": p50_read_us,
                "read_p99_us": p99_read_us,
                "read_throughput_ops_sec": read_throughput,
                "write_p50_us": p50_write_us,
                "write_p99_us": p99_write_us,
                "write_throughput_ops_sec": write_throughput
            })
        );
    } else {
        println!("🏁 ─── Hardware Benchmark Results ───");
        println!(
            "  • In-Process Point Reads  : p50 = {:.2} µs | p99 = {:.2} µs ({:.0} ops/sec)",
            p50_read_us, p99_read_us, read_throughput
        );
        println!(
            "  • In-Process Direct Writes: p50 = {:.2} µs | p99 = {:.2} µs ({:.0} ops/sec)",
            p50_write_us, p99_write_us, write_throughput
        );
        println!("  • Hardware Grounding      : 100% Measured Locally on Disk (Zero Synthetic Numbers)");
    }
    Ok(())
}

async fn handle_mcp_list(cli: &Cli) -> Result<()> {
    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
    let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
    let handler = McpHandler::new(db, committer, vector_index);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/list".to_string(),
        params: None,
    };

    let resp = handler.handle_request(req).await;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

async fn handle_mcp_call(cli: &Cli, tool: &str, args: &str) -> Result<()> {
    let parsed_args: serde_json::Value =
        serde_json::from_str(args).context("Failed to parse args as JSON object")?;

    let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
    let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
    let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
    let handler = McpHandler::new(db, committer, vector_index);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": tool,
            "arguments": parsed_args
        })),
    };

    let resp = handler.handle_request(req).await;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
