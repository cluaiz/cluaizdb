use std::path::Path;
use std::sync::Arc;
use heed::types::{Bytes, Unit};
use heed::{Database, Env, EnvOpenOptions};
use parking_lot::RwLock;

use crate::error::Result;
use crate::schema::{
    encode_edge_key, encode_index_key, encode_index_prefix, encode_record_key,
    decode_edge_key, decode_index_key,
    EdgeMetadata, StatsMetadata,
};
use crate::traits::StorageSubstrate;

/// The physical storage engine manager for CluaizDB Kernel v2.
/// Wraps 6 dedicated LMDB/Heed tables.
#[derive(Clone)]
pub struct SubstrateEnv {
    env: Arc<Env>,
    records_db: Database<Bytes, Bytes>,
    indexes_db: Database<Bytes, Unit>,
    outbound_edges_db: Database<Bytes, Bytes>,
    inbound_edges_db: Database<Bytes, Unit>,
    genomes_db: Database<Bytes, Bytes>,
    stats_db: Database<Bytes, Bytes>,
    // Write lock ensures serialized mutation transactions without double-locking
    write_lock: Arc<RwLock<()>>,
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
        wtxn.commit()?;

        Ok(Self {
            env: Arc::new(env),
            records_db,
            indexes_db,
            outbound_edges_db,
            inbound_edges_db,
            genomes_db,
            stats_db,
            write_lock: Arc::new(RwLock::new(())),
        })
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
        Ok(())
    }

    fn delete_record(&self, col_id: u32, record_id: u128) -> Result<bool> {
        let _guard = self.write_lock.write();
        let mut wtxn = self.env.write_txn()?;
        let key = encode_record_key(col_id, record_id);
        let deleted = self.records_db.delete(&mut wtxn, &key)?;
        wtxn.commit()?;
        Ok(deleted)
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

    fn sync(&self) -> Result<()> {
        self.env.force_sync()?;
        Ok(())
    }
}
