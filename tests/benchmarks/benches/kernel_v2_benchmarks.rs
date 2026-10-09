use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tempfile::tempdir;
use cluaizdb_substrate::{StorageSubstrate, SubstrateEnv};
use cluaizdb_group_commit::{GroupCommitter, GroupCommitConfig};
use cluaizdb_index_mvhsnw::{CosineDistance, HnswIndex};
use cluaizdb::{cluaizdb_open, cluaizdb_put, cluaizdb_get, cluaizdb_close};
use std::ffi::CString;

fn bench_substrate_point_read(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = SubstrateEnv::open(dir.path(), 50 * 1024 * 1024).unwrap();

    // 1. Seed 1,000 records
    for i in 0..1000u128 {
        let payload = format!("bench-payload-record-data-#{}", i);
        db.put_record(1, i, payload.as_bytes()).unwrap();
    }

    let target_id = 500u128;

    // Benchmark: In-Process Point Read (< 5 µs target)
    c.bench_function("substrate_point_read_latency", |b| {
        b.iter(|| {
            let res = db.get_record(black_box(1), black_box(target_id)).unwrap();
            black_box(res);
        })
    });
}

fn bench_c_ffi_point_read(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let path = dir.path().to_str().unwrap();
    let c_path = CString::new(path).unwrap();

    unsafe {
        let handle = cluaizdb_open(c_path.as_ptr(), 50 * 1024 * 1024);
        assert!(!handle.is_null());

        for i in 0..1000u64 {
            let payload = format!("ffi-bench-record-#{}", i);
            cluaizdb_put(handle, 1, 0, i, payload.as_ptr(), payload.len());
        }

        let mut out_buf = [0u8; 128];
        let mut out_len = 0usize;

        c.bench_function("c_ffi_point_read_latency", |b| {
            b.iter(|| {
                let rc = cluaizdb_get(
                    black_box(handle),
                    black_box(1),
                    black_box(0),
                    black_box(500),
                    out_buf.as_mut_ptr(),
                    out_buf.len(),
                    &mut out_len as *mut usize,
                );
                black_box(rc);
            })
        });

        cluaizdb_close(handle);
    }
}

fn bench_vector_knn_search(c: &mut Criterion) {
    let index = Arc::new(HnswIndex::<CosineDistance>::new());

    // Seed 500 vectors of 32 dimensions
    for i in 0..500u128 {
        let vec: Vec<f32> = (0..32).map(|x| ((x as f32) + (i as f32) * 0.1).sin()).collect();
        index.insert(i, vec);
    }

    let query: Vec<f32> = (0..32).map(|x| (x as f32).sin()).collect();

    c.bench_function("vector_hnsw_knn_search_k5", |b| {
        b.iter(|| {
            let results = index.search_knn(black_box(&query), black_box(5));
            black_box(results);
        })
    });
}

fn bench_group_commit_durable_write(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let dir = tempdir().unwrap();
    let db = SubstrateEnv::open(dir.path(), 50 * 1024 * 1024).unwrap();
    let committer = GroupCommitter::new(db, GroupCommitConfig::default());

    let mut counter = 0u128;

    c.bench_function("group_commit_durable_write", |b| {
        b.to_async(&rt).iter(|| {
            counter += 1;
            let comm = &committer;
            async move {
                let res = comm.write(1, counter, b"durable-ssd-commit-payload".to_vec()).await;
                black_box(res.unwrap());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_substrate_point_read,
    bench_c_ffi_point_read,
    bench_vector_knn_search,
    bench_group_commit_durable_write
);
criterion_main!(benches);
