use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use serde_json::json;
use std::time::Duration;
use reqwest::blocking::Client;

const BASE_URL: &str = "http://localhost:7331";

fn check_server() {
    let client = Client::new();
    let mut attempts = 0;
    loop {
        if client.get(&format!("{}/health", BASE_URL)).send().is_ok() {
            break;
        }
        attempts += 1;
        if attempts > 30 {
            panic!("Server did not start! Run `cargo run --bin cluaizdb-server` before benchmarking.");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn bench_write_latency(c: &mut Criterion) {
    check_server();
    let client = Client::builder().pool_max_idle_per_host(100).build().unwrap();
    let payload = json!({
        "raw_payload": "Benchmarking Payload",
        "vector_data": vec![0.1_f32; 16],
        "model_creator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "payload_type": "text",
    });

    c.bench_function("write_neuron_latency", |b| {
        b.iter_batched(
            || payload.clone(),
            |payload| {
                client.post(&format!("{}/neuron", BASE_URL))
                    .json(&payload)
                    .send()
                    .unwrap();
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_read_latency(c: &mut Criterion) {
    check_server();
    let client = Client::builder().pool_max_idle_per_host(100).build().unwrap();
    let payload = json!({
        "raw_payload": "Benchmarking Payload for Read",
        "vector_data": vec![0.1_f32; 16],
        "model_creator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "payload_type": "text",
    });

    // Write one to read
    let res: serde_json::Value = client.post(&format!("{}/neuron", BASE_URL))
        .json(&payload)
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    let neuron_id = res["neuron_id"].as_str().unwrap().to_string();

    c.bench_function("read_neuron_latency", |b| {
        b.iter(|| {
            client.get(&format!("{}/neuron/{}", BASE_URL, neuron_id))
                .send()
                .unwrap();
        })
    });
}

criterion_group!(benches, bench_write_latency, bench_read_latency);
criterion_main!(benches);
