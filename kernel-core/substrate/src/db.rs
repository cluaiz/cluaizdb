use std::path::Path;
use std::sync::Arc;
use byteorder::{BigEndian, ByteOrder};
use heed::types::{Bytes, Unit};
use heed::{Database, Env, EnvOpenOptions};
use parking_lot::RwLock;

use crate::error::Result;
use crate::schema::{
    encode_edge_key, encode_index_key, encode_index_prefix, encode_record_key,
    decode_edge_key, decode_index_key, decode_record_key,
    encode_vector, decode_vector, encode_timeseries_key, decode_timeseries_key,
    EdgeMetadata, StatsMetadata,
};
use crate::traits::{StorageSubstrate, SubstrateHook};

/// The physical storage engine manager for CluaizDB 0.1.0 Alpha.
/// Wraps 9 dedicated LMDB/Heed tables.
#[derive(Clone)]
pub struct SubstrateEnv {
    env: Arc<Env>,
    records_db: Database<Bytes, Bytes>,
    indexes_db: Database<Bytes, Unit>,
    outbound_edges_db: Database<Bytes, Bytes>,
    inbound_edges_db: Database<Bytes, Unit>,
    genomes_db: Database<Bytes, Bytes>,
    stats_db: Database<Bytes, Bytes>,
    vectors_db: Database<Bytes, Bytes>,
    timeseries_db: Database<Bytes, Bytes>,
    collections_db: Database<Bytes, Bytes>,
    // Write lock ensures serialized mutation transactions without double-locking
    write_lock: Arc<RwLock<()>>,
    // Reactive in-engine mutation triggers
    hooks: Arc<RwLock<Vec<Arc<dyn SubstrateHook>>>>,
}

impl SubstrateEnv {
    /// Opens or creates the physical Substrate environment at the specified directory.
    pub fn open<P: AsRef<Path>>(path: P, map_size: usize) -> Result<Self> {
        std::fs::create_dir_all(&path)?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(map_size)
                .max_dbs(16)
                .open(&path)?
        };

        let mut wtxn = env.write_txn()?;
        let records_db = env.create_database(&mut wtxn, Some("records"))?;
        let indexes_db = env.create_database(&mut wtxn, Some("indexes"))?;
        let outbound_edges_db = env.create_database(&mut wtxn, Some("outbound_edges"))?;
        let inbound_edges_db = env.create_database(&mut wtxn, Some("inbound_edges"))?;
        let genomes_db = env.create_database(&mut wtxn, Some("genomes"))?;
        let stats_db = env.create_database(&mut wtxn, Some("stats"))?;
        let vectors_db = env.create_database(&mut wtxn, Some("vectors"))?;
        let timeseries_db = env.create_database(&mut wtxn, Some("timeseries"))?;
        let collections_db = env.create_database(&mut wtxn, Some("collections"))?;
        wtxn.commit()?;

        Ok(Self {
            env: Arc::new(env),
            records_db,
            indexes_db,
            outbound_edges_db,
            inbound_edges_db,
            genomes_db,
            stats_db,
            vectors_db,
            timeseries_db,
            collections_db,
            write_lock: Arc::new(RwLock::new(())),
            hooks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Registers a reactive in-engine hook invoked upon storage mutations.
    pub fn register_hook(&self, hook: Arc<dyn SubstrateHook>) {
        self.hooks.write().push(hook);
    }

    /// Access the underlying raw Heed environment.
    pub fn raw_env(&self) -> &Env {
        &self.env
    }
}

impl StorageSubstrate for SubstrateEnv {
    fn get_record(&self, col_id: u32, record_id: u128) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        let key = encode_record_key(col_id, record_id);
        match self.records_db.get(&rtxn, &key)? {
            Some(bytes) => Ok(Some(bytes.to_vec())),
            None => Ok(None),
        }
    }

    fn put_record(&self, col_id: u32, record_id: u128, payload: &[u8]) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_record_key(col_id, record_id);
        self.records_db.put(&mut wtxn, &key, payload)?;
        wtxn.commit()?;

        // Trigger registered in-engine reactive hooks
        let active_hooks = self.hooks.read().clone();
        for hook in active_hooks {
            hook.on_record_put(col_id, record_id, payload);
        }

        Ok(())
    }

    fn delete_record(&self, col_id: u32, record_id: u128) -> Result<bool> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_record_key(col_id, record_id);
        let deleted = self.records_db.delete(&mut wtxn, &key)?;

        if deleted {
            // Atomic cleanup: purge vector and all secondary index terms pointing to deleted record
            let _ = self.vectors_db.delete(&mut wtxn, &key);

            let prefix = col_id.to_be_bytes();
            let rec_bytes = record_id.to_be_bytes();
            let mut ghost_keys = Vec::new();
            {
                let mut iter = self.indexes_db.iter(&wtxn)?;
                while let Some(res) = iter.next() {
                    let (k, ()) = res?;
                    if !k.starts_with(&prefix) {
                        if k > prefix.as_slice() {
                            break;
                        }
                        continue;
                    }
                    if k.ends_with(&rec_bytes) {
                        ghost_keys.push(k.to_vec());
                    }
                }
            }
            for k in ghost_keys {
                self.indexes_db.delete(&mut wtxn, &k)?;
            }

            let active_hooks = self.hooks.read().clone();
            for hook in active_hooks {
                hook.on_record_delete(col_id, record_id);
                hook.on_vector_delete(col_id, record_id);
            }
        }

        wtxn.commit()?;
        Ok(deleted)
    }

    fn scan_records(&self, col_id: u32, limit: usize) -> Result<Vec<(u128, Vec<u8>)>> {
        let rtxn = self.env.read_txn()?;
        let prefix = col_id.to_be_bytes();
        let mut results = Vec::new();
        let mut iter = self.records_db.iter(&rtxn)?;
        while let Some(res) = iter.next() {
            let (key, val) = res?;
            if !key.starts_with(&prefix) {
                if key > prefix.as_slice() {
                    break;
                }
                continue;
            }
            if let Some((_, rec_id)) = decode_record_key(key) {
                results.push((rec_id, val.to_vec()));
                if results.len() >= limit {
                    break;
                }
            }
        }
        Ok(results)
    }

    fn index_term(&self, col_id: u32, term: &[u8], record_id: u128) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_index_key(col_id, term, record_id);
        self.indexes_db.put(&mut wtxn, &key, &())?;
        wtxn.commit()?;
        Ok(())
    }

    fn remove_index_term(&self, col_id: u32, term: &[u8], record_id: u128) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_index_key(col_id, term, record_id);
        self.indexes_db.delete(&mut wtxn, &key)?;
        wtxn.commit()?;
        Ok(())
    }

    fn seek_index(&self, col_id: u32, term: &[u8]) -> Result<Vec<u128>> {
        let rtxn = self.env.read_txn()?;
        let prefix = encode_index_prefix(col_id, term);
        let mut matches = Vec::new();

        // Scan B+Tree starting at prefix
        let mut iter = self.indexes_db.iter(&rtxn)?;
        while let Some(result) = iter.next() {
            let (key, ()) = result?;
            if !key.starts_with(&prefix) {
                if key > prefix.as_slice() {
                    break;
                }
                continue;
            }
            if let Some((_, _, rec_id)) = decode_index_key(key) {
                matches.push(rec_id);
            }
        }
        Ok(matches)
    }

    fn add_edge(&self, src_id: u128, type_id: u32, dst_id: u128, meta: EdgeMetadata) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let out_key = encode_edge_key(src_id, type_id, dst_id);
        let in_key = encode_edge_key(dst_id, type_id, src_id);

        let meta_bytes = meta.to_bytes();
        self.outbound_edges_db.put(&mut wtxn, &out_key, &meta_bytes)?;
        self.inbound_edges_db.put(&mut wtxn, &in_key, &())?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete_edge(&self, src_id: u128, type_id: u32, dst_id: u128) -> Result<bool> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let out_key = encode_edge_key(src_id, type_id, dst_id);
        let in_key = encode_edge_key(dst_id, type_id, src_id);

        let deleted = self.outbound_edges_db.delete(&mut wtxn, &out_key)?;
        self.inbound_edges_db.delete(&mut wtxn, &in_key)?;
        wtxn.commit()?;
        Ok(deleted)
    }

    fn get_outbound_edges(&self, src_id: u128, type_id: Option<u32>) -> Result<Vec<(u128, EdgeMetadata)>> {
        let rtxn = self.env.read_txn()?;
        let mut edges = Vec::new();
        let mut iter = self.outbound_edges_db.iter(&rtxn)?;

        while let Some(result) = iter.next() {
            let (key, val) = result?;
            if let Some((src, t, dst)) = decode_edge_key(key) {
                if src == src_id {
                    if let Some(target_type) = type_id {
                        if t != target_type {
                            continue;
                        }
                    }
                    if let Some(meta) = EdgeMetadata::from_bytes(val) {
                        edges.push((dst, meta));
                    }
                } else if src > src_id {
                    break;
                }
            }
        }
        Ok(edges)
    }

    fn get_inbound_edges(&self, dst_id: u128, type_id: Option<u32>) -> Result<Vec<u128>> {
        let rtxn = self.env.read_txn()?;
        let mut sources = Vec::new();
        let mut iter = self.inbound_edges_db.iter(&rtxn)?;

        while let Some(result) = iter.next() {
            let (key, ()) = result?;
            if let Some((dst, t, src)) = decode_edge_key(key) {
                if dst == dst_id {
                    if let Some(target_type) = type_id {
                        if t != target_type {
                            continue;
                        }
                    }
                    sources.push(src);
                } else if dst > dst_id {
                    break;
                }
            }
        }
        Ok(sources)
    }

    fn put_genome(&self, wasm_id: u64, bytecode: &[u8]) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = wasm_id.to_be_bytes();
        self.genomes_db.put(&mut wtxn, &key, bytecode)?;
        wtxn.commit()?;
        Ok(())
    }

    fn get_genome(&self, wasm_id: u64) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        let key = wasm_id.to_be_bytes();
        match self.genomes_db.get(&rtxn, &key)? {
            Some(bytes) => Ok(Some(bytes.to_vec())),
            None => Ok(None),
        }
    }

    fn update_stats(&self, record_id: u128, stats: StatsMetadata) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = record_id.to_be_bytes();
        let val = stats.to_bytes();
        self.stats_db.put(&mut wtxn, &key, &val)?;
        wtxn.commit()?;
        Ok(())
    }

    fn get_stats(&self, record_id: u128) -> Result<Option<StatsMetadata>> {
        let rtxn = self.env.read_txn()?;
        let key = record_id.to_be_bytes();
        match self.stats_db.get(&rtxn, &key)? {
            Some(bytes) => Ok(StatsMetadata::from_bytes(bytes)),
            None => Ok(None),
        }
    }

    fn put_vector(&self, col_id: u32, record_id: u128, vector: &[f32]) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_record_key(col_id, record_id);
        let val = encode_vector(vector);
        self.vectors_db.put(&mut wtxn, &key, &val)?;

        let active_hooks = self.hooks.read().clone();
        for hook in active_hooks {
            hook.on_vector_put(col_id, record_id, vector);
        }

        wtxn.commit()?;
        Ok(())
    }

    fn get_vector(&self, col_id: u32, record_id: u128) -> Result<Option<Vec<f32>>> {
        let rtxn = self.env.read_txn()?;
        let key = encode_record_key(col_id, record_id);
        match self.vectors_db.get(&rtxn, &key)? {
            Some(bytes) => Ok(decode_vector(bytes)),
            None => Ok(None),
        }
    }

    fn delete_vector(&self, col_id: u32, record_id: u128) -> Result<bool> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_record_key(col_id, record_id);
        let deleted = self.vectors_db.delete(&mut wtxn, &key)?;

        if deleted {
            let active_hooks = self.hooks.read().clone();
            for hook in active_hooks {
                hook.on_vector_delete(col_id, record_id);
            }
        }

        wtxn.commit()?;
        Ok(deleted)
    }

    fn scan_vectors(&self, col_id: u32, limit: usize) -> Result<Vec<(u128, Vec<f32>)>> {
        let rtxn = self.env.read_txn()?;
        let prefix = col_id.to_be_bytes();
        let mut results = Vec::new();
        let mut iter = self.vectors_db.iter(&rtxn)?;
        while let Some(res) = iter.next() {
            let (key, val) = res?;
            if !key.starts_with(&prefix) {
                if key > prefix.as_slice() {
                    break;
                }
                continue;
            }
            if let Some((_, rec_id)) = decode_record_key(key) {
                if let Some(vec) = decode_vector(val) {
                    results.push((rec_id, vec));
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(results)
    }

    fn put_timeseries_chunk(&self, series_id: u64, start_time_ms: u64, chunk: &[u8]) -> Result<()> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_timeseries_key(series_id, start_time_ms);
        self.timeseries_db.put(&mut wtxn, &key, chunk)?;
        wtxn.commit()?;
        Ok(())
    }

    fn scan_timeseries_chunks(
        &self,
        series_id: u64,
        start_time_ms: u64,
        end_time_ms: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>> {
        let rtxn = self.env.read_txn()?;
        let prefix = series_id.to_be_bytes();
        let mut results = Vec::new();
        let mut iter = self.timeseries_db.iter(&rtxn)?;
        while let Some(res) = iter.next() {
            let (key, val) = res?;
            if !key.starts_with(&prefix) {
                if key > prefix.as_slice() {
                    break;
                }
                continue;
            }
            if let Some((s_id, ts)) = decode_timeseries_key(key) {
                if s_id == series_id && ts >= start_time_ms && ts <= end_time_ms {
                    results.push((ts, val.to_vec()));
                }
            }
        }
        Ok(results)
    }

    fn get_or_create_collection(&self, name: &str) -> Result<u32> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(1); // Default collection
        }

        let name_key = format!("N:{}", name);
        // Fast-path read
        {
            let rtxn = self.env.read_txn()?;
            if let Some(bytes) = self.collections_db.get(&rtxn, name_key.as_bytes())? {
                if bytes.len() == 4 {
                    return Ok(BigEndian::read_u32(bytes));
                }
            }
        }

        // Mutation path: allocate next sequential ID
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;

        // Re-check under lock
        if let Some(bytes) = self.collections_db.get(&wtxn, name_key.as_bytes())? {
            if bytes.len() == 4 {
                return Ok(BigEndian::read_u32(bytes));
            }
        }

        let counter_key = b"__NEXT_COL_ID__";
        let next_id = match self.collections_db.get(&wtxn, counter_key)? {
            Some(bytes) if bytes.len() == 4 => BigEndian::read_u32(bytes),
            _ => 1u32,
        };

        let mut id_buf = [0u8; 4];
        BigEndian::write_u32(&mut id_buf, next_id);

        let id_key = format!("I:{}", next_id);

        // Bi-directional mappings
        self.collections_db.put(&mut wtxn, name_key.as_bytes(), &id_buf)?;
        self.collections_db.put(&mut wtxn, id_key.as_bytes(), name.as_bytes())?;

        let mut next_counter_buf = [0u8; 4];
        BigEndian::write_u32(&mut next_counter_buf, next_id + 1);
        self.collections_db.put(&mut wtxn, counter_key, &next_counter_buf)?;

        wtxn.commit()?;
        Ok(next_id)
    }

    fn get_collection_id(&self, name: &str) -> Result<Option<u32>> {
        let rtxn = self.env.read_txn()?;
        let name_key = format!("N:{}", name.trim());
        match self.collections_db.get(&rtxn, name_key.as_bytes())? {
            Some(bytes) if bytes.len() == 4 => Ok(Some(BigEndian::read_u32(bytes))),
            _ => Ok(None),
        }
    }

    fn get_collection_name(&self, col_id: u32) -> Result<Option<String>> {
        let rtxn = self.env.read_txn()?;
        let id_key = format!("I:{}", col_id);
        match self.collections_db.get(&rtxn, id_key.as_bytes())? {
            Some(bytes) => Ok(String::from_utf8(bytes.to_vec()).ok()),
            _ => Ok(None),
        }
    }

    fn list_collections(&self) -> Result<Vec<(String, u32)>> {
        let rtxn = self.env.read_txn()?;
        let prefix = b"N:";
        let mut results = Vec::new();
        let mut iter = self.collections_db.iter(&rtxn)?;
        while let Some(res) = iter.next() {
            let (k, v) = res?;
            if !k.starts_with(prefix) {
                if k > prefix.as_slice() {
                    break;
                }
                continue;
            }
            if let Ok(name) = std::str::from_utf8(&k[2..]) {
                if v.len() == 4 {
                    let id = BigEndian::read_u32(v);
                    results.push((name.to_string(), id));
                }
            }
        }
        results.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(results)
    }

    fn sync(&self) -> Result<()> {
        self.env.force_sync()?;
        Ok(())
    }
}
