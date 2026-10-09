use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use cluaizdb_errors::StorageError;
use cluaizdb_types::UniversalNeuron;
use engine_lmdb::LmdbEnv;
use wal::{WalWriter, WalOperation};

/// Local collection configuration (Method B)
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CollectionConfig {
    pub concurrency_mode: String, // "dashmap" | "mutex"
    pub payload_format: String,    // "json" | "protobuf" | "flatbuffers"
}

/// Represents an active, open shard with its associated LMDB environment, WAL writer, and configuration.
pub struct ActiveShard {
    /// The memory-mapped LMDB environment wrapped in an Arc.
    pub env: Arc<LmdbEnv>,
    /// The active WAL writer protected by a tokio Mutex.
    pub wal_writer: Arc<Mutex<WalWriter>>,
    /// The RAM ring-buffer for extreme high-frequency writes before LMDB.
    pub transit_lounge: crate::routes::transit::TransitLounge,
    /// Active collection config
    pub config: CollectionConfig,
}

/// Thread-safe concurrent registry wrapper supporting Mutex and DashMap options
pub enum ShardRegistry {
    Dash(dashmap::DashMap<String, Arc<ActiveShard>>),
    Mutex(tokio::sync::Mutex<HashMap<String, Arc<ActiveShard>>>),
}

/// Dynamic registry managing multi-tenant database environments.
pub struct ShardManager {
    base_path: PathBuf,
    shards: ShardRegistry,
    telemetry: Arc<RwLock<heart::Telemetry>>,
    default_concurrency_mode: String,
    default_payload_format: String,
}

impl ShardManager {
    /// Create a new ShardManager under the specified directory path with default options.
    pub fn new(
        base_path: &Path,
        telemetry: Arc<RwLock<heart::Telemetry>>,
        default_concurrency_mode: String,
        default_payload_format: String,
    ) -> Self {
        let registry = if default_concurrency_mode == "mutex" {
            info!("Initializing ShardManager with global MUTEX concurrency registry");
            ShardRegistry::Mutex(tokio::sync::Mutex::new(HashMap::new()))
        } else {
            info!("Initializing ShardManager with global DASHMAP concurrent lock-free registry");
            ShardRegistry::Dash(dashmap::DashMap::new())
        };

        Self {
            base_path: base_path.to_path_buf(),
            shards: registry,
            telemetry,
            default_concurrency_mode,
            default_payload_format,
        }
    }

    /// Retrieve an already open shard or lazily initialize a new one.
    ///
    /// Running recovery on the target shard is handled automatically before returning
    /// to guarantee zero data loss.
    pub async fn get_or_open_shard(&self, tenant_id: &str) -> Result<Arc<ActiveShard>, StorageError> {
        match &self.shards {
            ShardRegistry::Dash(map) => {
                if let Some(shard) = map.get(tenant_id) {
                    return Ok(Arc::clone(shard.value()));
                }
                // Initialize the new shard
                let shard = self.open_shard_internal(tenant_id).await?;
                map.insert(tenant_id.to_string(), Arc::clone(&shard));
                Ok(shard)
            }
            ShardRegistry::Mutex(mutex) => {
                let mut guard = mutex.lock().await;
                if let Some(shard) = guard.get(tenant_id) {
                    return Ok(Arc::clone(shard));
                }
                // Initialize the new shard
                let shard = self.open_shard_internal(tenant_id).await?;
                guard.insert(tenant_id.to_string(), Arc::clone(&shard));
                Ok(shard)
            }
        }
    }

    /// Internal logic to open LMDB, recover WAL, and load/save configuration overrides.
    async fn open_shard_internal(&self, tenant_id: &str) -> Result<Arc<ActiveShard>, StorageError> {
        let tenant_dir = self.base_path.join(tenant_id);
        if !tenant_dir.exists() {
            std::fs::create_dir_all(&tenant_dir)
                .map_err(|e| StorageError::EnvOpenFailed {
                    path: tenant_dir.to_string_lossy().into_owned(),
                    reason: e.to_string(),
                })?;
        }

        let db_path = tenant_dir.join("db");
        let wal_path = tenant_dir.join("wal");
        let config_path = tenant_dir.join("collection_config.json");

        // Method B: Load local overrides or fallback to Method A defaults
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| StorageError::DeserializationFailed(e.to_string()))?;
            serde_json::from_str::<CollectionConfig>(&content)
                .map_err(|e| StorageError::DeserializationFailed(e.to_string()))?
        } else {
            let default_cfg = CollectionConfig {
                concurrency_mode: self.default_concurrency_mode.clone(),
                payload_format: self.default_payload_format.clone(),
            };
            let content = serde_json::to_string_pretty(&default_cfg)
                .map_err(|e| StorageError::SerializationFailed(e.to_string()))?;
            std::fs::write(&config_path, content)
                .map_err(|e| StorageError::EnvOpenFailed {
                    path: config_path.to_string_lossy().into_owned(),
                    reason: e.to_string(),
                })?;
            default_cfg
        };

        info!(
            tenant = %tenant_id,
            concurrency = %config.concurrency_mode,
            format = %config.payload_format,
            "Loading collection configuration"
        );

        // 1. Open LMDB Environment (Map size configured to 1 GB) and wrap in Arc
        let env = Arc::new(LmdbEnv::open(&db_path, 1024 * 1024 * 1024)?);

        // 2. Perform WAL recovery
        wal::recover_from_wal(&wal_path, &mut |entry| {
            match entry.operation {
                WalOperation::Write { payload } => {
                    let neuron: UniversalNeuron = serde_json::from_slice(&payload)
                        .map_err(|e| StorageError::DeserializationFailed(e.to_string()))?;
                    engine_lmdb::write_neuron(&env, &neuron)?;
                }
                WalOperation::Delete { .. } => {}
            }
            Ok(())
        })?;

        // 3. Open WalWriter for subsequent transactions
        let wal_writer = Arc::new(Mutex::new(WalWriter::open(&wal_path)?));

        // 4. Spawn the GC thread
        engine_lmdb::spawn_biological_gc(Arc::clone(&env), Arc::clone(&self.telemetry));

        // 5. Initialize the Volatile Synaptic Transit Lounge
        let transit_lounge = crate::routes::transit::TransitLounge::new(100_000, Arc::clone(&env), Arc::clone(&wal_writer));

        Ok(Arc::new(ActiveShard {
            env,
            wal_writer,
            transit_lounge,
            config,
        }))
    }

    /// Return the count of currently cached active shards.
    pub async fn active_shards_count(&self) -> usize {
        match &self.shards {
            ShardRegistry::Dash(map) => map.len(),
            ShardRegistry::Mutex(mutex) => mutex.lock().await.len(),
        }
    }

    /// Return a snapshot of all active shards for background tasks like the Dreamer.
    pub async fn get_all_shards(&self) -> Vec<Arc<ActiveShard>> {
        match &self.shards {
            ShardRegistry::Dash(map) => map.iter().map(|r| Arc::clone(r.value())).collect(),
            ShardRegistry::Mutex(mutex) => {
                let guard = mutex.lock().await;
                guard.values().cloned().collect()
            }
        }
    }

    /// Forcefully run a biological GC sweep cycle on all open database shards.
    pub async fn run_gc_sweep_on_all_shards(&self) -> Result<(), StorageError> {
        let shards = self.get_all_shards().await;
        let spo2 = {
            let tel = self.telemetry.read().await;
            tel.spo2
        };

        for shard in shards {
            if let Err(e) = engine_lmdb::run_gc_sweep(&shard.env, spo2) {
                warn!(error = %e, "GC Sweep failed on shard");
            }
        }
        Ok(())
    }
}
