use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv};
use cluaizdb_group_commit::{GroupCommitter, GroupCommitConfig};
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb_mcp::{McpHandler, JsonRpcRequest};
use cluaizdb_server_http::{create_router, AppState};
use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};

#[derive(Parser)]
#[command(
    name = "cluaizdb",
    about = "CluaizDB Kernel v2 CLI — Microsecond programmable database substrate",
    version = "0.2.0"
)]
struct Cli {
    /// Path to physical database directory
    #[arg(short, long, default_value = "./data/cluaizdb", global = true)]
    data_dir: PathBuf,

    /// Maximum memory-map size in bytes (default: 10 GB)
    #[arg(long, default_value_t = 10 * 1024 * 1024 * 1024, global = true)]
    map_size: usize,

    /// Output results as JSON
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the sovereign CluaizDB server daemon (REST & MCP)
    Serve {
        #[arg(short, long, default_value = "0.0.0.0")]
        host: String,

        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },

    /// Durably store a record into Substrate
    Put {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        payload: String,
    },

    /// Retrieve a record by ID directly from memory page-cache (< 5 µs recall)
    Get {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,
    },

    /// Delete a record by ID
    Delete {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        id: String,
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

    /// Seek record IDs matching an indexed term (< 5 µs recall)
    SeekTerm {
        #[arg(short, long, default_value_t = 1)]
        col: u32,

        #[arg(short, long)]
        term: String,
    },

    /// List all native Model Context Protocol (MCP) tools
    McpList,

    /// Execute an MCP tool call
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

    match cli.command {
        Commands::Serve { host, port } => {
            println!("🚀 Starting CluaizDB Kernel v2 Server on {}:{}", host, port);
            let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
            let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
            let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
            let wasm_runner = Arc::new(WasmGenomeRunner::new(SandboxConfig::default())?);

            let state = Arc::new(AppState::new(db, committer, vector_index, wasm_runner));
            let router = create_router(state);

            let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()?;
            let listener = tokio::net::TcpListener::bind(addr).await?;
            println!("📡 Listening at http://{}", addr);
            axum::serve(listener, router).await?;
        }

        Commands::Put { col, id, payload } => {
            let record_id: u128 = id.parse().context("Invalid record_id: must be 128-bit decimal")?;
            let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
            let committer = GroupCommitter::new(db, GroupCommitConfig::default());

            committer.write(col, record_id, payload.into_bytes()).await?;

            if cli.json {
                println!("{}", serde_json::json!({ "status": "committed", "col": col, "id": id }));
            } else {
                println!("✅ [OK] Record {} durably written to collection {}", id, col);
            }
        }

        Commands::Get { col, id } => {
            let record_id: u128 = id.parse().context("Invalid record_id: must be 128-bit decimal")?;
            let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;

            match db.get_record(col, record_id)? {
                Some(bytes) => {
                    let payload_str = String::from_utf8(bytes).unwrap_or_else(|_| "<binary>".to_string());
                    if cli.json {
                        println!("{}", serde_json::json!({ "found": true, "col": col, "id": id, "payload": payload_str }));
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
        }

        Commands::Delete { col, id } => {
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
        }

        Commands::IndexTerm { col, term, id } => {
            let record_id: u128 = id.parse().context("Invalid record_id")?;
            let db = SubstrateEnv::open(&cli.data_dir, cli.map_size)?;
            db.index_term(col, term.as_bytes(), record_id)?;

            if cli.json {
                println!("{}", serde_json::json!({ "status": "indexed", "term": term, "id": id }));
            } else {
                println!("🔍 [OK] Indexed term '{}' -> {}", term, id);
            }
        }

        Commands::SeekTerm { col, term } => {
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
        }

        Commands::McpList => {
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
        }

        Commands::McpCall { tool, args } => {
            let parsed_args: serde_json::Value = serde_json::from_str(&args)
                .context("Failed to parse args as JSON object")?;

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
        }
    }

    Ok(())
}
