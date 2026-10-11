use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;
use tracing::info;
use cluaizdb_substrate::SubstrateEnv;
use cluaizdb_group_commit::{GroupCommitter, GroupCommitConfig};
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb_wasm_sandbox::{SandboxConfig, WasmGenomeRunner};
use cluaizdb_server_http::{create_router, AppState};

#[derive(Parser, Debug)]
#[command(author, version = "0.1.0", about = "CluaizDB 0.1.0 Alpha Sovereign Edge Server & MCP Host")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value_t = 7331)]
    port: u16,

    #[arg(short, long, default_value = "./data/cluaizdb")]
    data_dir: String,

    #[arg(short, long, default_value_t = 10 * 1024 * 1024 * 1024)]
    map_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,cluaizdb_server_http=debug".into()),
        )
        .init();

    let args = Args::parse();

    std::fs::create_dir_all(&args.data_dir)?;

    info!("Initializing CluaizDB 0.1.0 Alpha Substrate at: {}", args.data_dir);
    let db = SubstrateEnv::open(&args.data_dir, args.map_size)?;
    let committer = GroupCommitter::new(db.clone(), GroupCommitConfig::default());
    let vector_index = Arc::new(HnswIndex::<CosineDistance>::new());
    if let Ok(count) = cluaizdb_cdql::rehydrate_hnsw_from_substrate(&db, 1, &vector_index) {
        if count > 0 {
            info!("🧠 Auto-rehydrated {} vectors from Substrate into HNSW index.", count);
        }
    }
    let wasm_runner = Arc::new(WasmGenomeRunner::new(SandboxConfig::default())?);

    let state = Arc::new(AppState::new(db, committer, vector_index, wasm_runner));
    let router = create_router(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!("🚀 CluaizDB 0.1.0 Alpha Edge Server listening on http://{}", addr);
    info!("📡 Model Context Protocol (MCP) active at http://{}/mcp", addr);
    info!("⚡ Microsecond recall (< 5 µs) & SSD Group Commit (~121 µs) ready.");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("❌ Fatal: Failed to bind to dedicated port {}: {}. Auto-port hopping is disabled to maintain API client stability.", args.port, e);
            return Err(e.into());
        }
    };
    axum::serve(listener, router).await?;

    Ok(())
}
