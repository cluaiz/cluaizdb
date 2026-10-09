use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use bytes::Bytes;
use cluaizdb_types::{PayloadType, UniversalNeuron};
use engine_lmdb::{read_neuron, write_neuron, LmdbEnv};

fn bench_in_process_lmdb(c: &mut Criterion) {
    let tmp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let env = LmdbEnv::open(tmp_dir.path(), 100 * 1024 * 1024).expect("Failed to open LMDB env");

    // 1. Seed 1,000 neurons into LMDB
    let mut seeded_ids = Vec::with_capacity(1000);
    for i in 0..1000 {
        let payload = Bytes::from(format!("Neuron payload test content #{}", i));
        let vector = vec![0.05f32; 16];
        let hash = [0u8; 32];
        let neuron = UniversalNeuron::new(payload, vector, hash, PayloadType::Text);
        seeded_ids.push(neuron.id);
        write_neuron(&env, &neuron).expect("Seed write failed");
    }

    let target_id = seeded_ids[500];

    // Benchmark 1: In-Process Point Read (LMDB B+Tree + zero-copy / codec decode)
    c.bench_function("lmdb_in_process_point_read", |b| {
        b.iter(|| {
            let n = read_neuron(black_box(&env), black_box(target_id), None).expect("Read failed");
            black_box(n);
        })
    });

    // Benchmark 2: In-Process Single Point Write
    let mut write_counter = 0u64;
    c.bench_function("lmdb_in_process_point_write", |b| {
        b.iter_batched(
            || {
                write_counter += 1;
                let payload = Bytes::from(format!("Write benchmark payload #{}", write_counter));
                UniversalNeuron::new(payload, vec![0.1f32; 16], [0u8; 32], PayloadType::Text)
            },
            |neuron| {
                write_neuron(black_box(&env), black_box(&neuron)).expect("Write failed");
            },
            BatchSize::SmallInput,
        );
    });

    // Benchmark 3: Group Commit Write
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let group_dir = tmp_dir.path().join("group_env");
    let group_env = LmdbEnv::open(&group_dir, 100 * 1024 * 1024).expect("Failed to open LMDB group env");
    let committer = engine_lmdb::GroupCommitter::new_with_handle(
        group_env,
        engine_lmdb::GroupCommitConfig {
            max_batch_size: 128,
            max_latency: std::time::Duration::from_millis(2),
            channel_capacity: 10_000,
        },
        rt.handle(),
    );

    c.bench_function("lmdb_group_commit_write", |b| {
        b.to_async(&rt).iter(|| {
            let committer = committer.clone();
            async move {
                let payload = Bytes::from("Group commit benchmark item");
                let neuron = UniversalNeuron::new(payload, vec![0.1f32; 16], [0u8; 32], PayloadType::Text);
                committer.write(neuron).await.expect("Group write failed");
            }
        })
    });
}

criterion_group!(benches, bench_in_process_lmdb);
criterion_main!(benches);
