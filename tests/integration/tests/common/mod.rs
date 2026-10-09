use reqwest::Client;
use std::time::Duration;

pub mod logger;

pub const BASE_URL: &str = "http://localhost:7331";

pub fn get_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build reqwest client")
}

pub async fn wait_for_server() {
    let client = get_client();
    let mut attempts = 0;
    loop {
        if client.get(&format!("{}/health", BASE_URL)).send().await.is_ok() {
            break;
        }
        attempts += 1;
        if attempts > 30 {
            panic!("Server did not start at {} after 3 seconds. Please run `cargo run --bin cluaizdb-server` before running tests.", BASE_URL);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
