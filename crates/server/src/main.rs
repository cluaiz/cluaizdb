use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod routes;
mod dreamer;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging.
    // Set RUST_LOG=debug to see all log output.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = config::ServerConfig::load();
    info!("Loaded configuration from cluaizdb.toml: {:?}", config);

    info!("Cluaizdb CLUAIZD Server v0.0.1 — starting");

    let shards_path = Path::new("data/shards");
    let sensory_path = Path::new("data/sensory");
    let wal_path = Path::new("data/wal");

    // ─────────────────────────────────────────────────────────
    // STEP 1: WAL Boot Recovery (Crash Safety Guarantee)
    // Replay any uncommitted WAL entries from a previous crash
    // into LMDB before we accept any new requests.
    // ─────────────────────────────────────────────────────────
    info!("Running WAL crash recovery (replaying uncommitted entries)...");
    let recovery_env = engine_lmdb::LmdbEnv::open(shards_path, 128 * 1024 * 1024 * 1024);
    match recovery_env {
        Ok(env) => {
            let mut replayed_count = 0usize;
            let result = wal::recover_from_wal(wal_path, &mut |entry| {
                if let wal::WalOperation::Write { payload } = &entry.operation {
                    if let Ok(neuron) = serde_json::from_slice::<cluaizdb_types::UniversalNeuron>(payload) {
                        // Check if this neuron is already in LMDB (idempotent replay)
                        if engine_lmdb::read_neuron(&env, neuron.id, None).is_err() {
                            engine_lmdb::write_neuron(&env, &neuron)?;
                            replayed_count += 1;
                        }
                    }
                }
                Ok(())
            });
            match result {
                Ok(r) => info!(
                    total = r.total_entries,
                    replayed = r.replayed,
                    new_writes = replayed_count,
                    corrupt_skipped = r.skipped_corrupt,
                    "WAL boot recovery complete ✓"
                ),
                Err(e) => warn!("WAL recovery encountered errors (non-fatal): {}", e),
            }
        }
        Err(e) => warn!("Could not open default shard for WAL recovery (clean boot?): {}", e),
    }

    info!("Starting Cluaizdb-HEART Autonomic Telemetry Engine");
    let data_dir = Path::new("data");
    let heart = heart::Heart::new(data_dir);
    heart.start_heartbeat();

    info!("Initializing CLUAIZD Shard Manager at {:?}", shards_path);
    let shard_manager = routes::ShardManager::new(
        shards_path,
        Arc::clone(&heart.telemetry),
        config.database.concurrency_mode.clone(),
        config.database.payload_format.clone(),
    );

    info!("Opening CLUAIZD Sensory Shard at {:?}", sensory_path);
    // Sensory shard capacity: 10,000 entries (ring buffer limit)
    let sensory_shard = engine_lmdb::SensoryShard::open(sensory_path, 10000)
        .map_err(|e| anyhow::anyhow!("Failed to open sensory shard: {}", e))?;

    info!("Loading Dynamic Genomes from ./genomes");
    let genome_registry = genome::GenomeRegistry::new();
    if let Err(e) = genome_registry.load_from_directory("genomes") {
        tracing::warn!("Failed to load some genomes: {}", e);
    }

    // ─────────────────────────────────────────────────────────
    // STEP 2: WASM DNA Hot-Reload Cache Setup
    // Ensure `active_dnas` directory exists, preload existing WASM into RAM,
    // and spawn the file watcher to hot-reload future WASM updates.
    // ─────────────────────────────────────────────────────────
    let active_dnas_path = Path::new("active_dnas");
    if !active_dnas_path.exists() {
        if let Err(e) = std::fs::create_dir_all(active_dnas_path) {
            tracing::warn!("Could not create active_dnas directory: {}", e);
        }
    }
    tracing::info!("Pre-loading WASM DNAs from {:?} into RAM cache...", active_dnas_path);
    genome::wasm_executor::WasmExecutor::preload_cache(active_dnas_path);
    
    // Start background watcher for hot-reloading
    genome::wasm_executor::start_dna_watcher(active_dnas_path.to_path_buf()).await;

    // Heart is already started above

    // Build the shared app state
    let state = routes::AppState::new(shard_manager, sensory_shard, genome_registry, Arc::clone(&heart.telemetry), Arc::clone(&heart.booster_state));

    // Start the Subconscious Dreaming Engine background task
    dreamer::spawn_dreamer(state.clone());

    // Build the Axum router with all routes.
    let app = routes::build_router(state);

    // Bind to address configured in cluaizdb.toml.
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr = %addr, "Server listening — ready to accept requests");

    axum::serve(listener, app).await?;

    Ok(())
}
