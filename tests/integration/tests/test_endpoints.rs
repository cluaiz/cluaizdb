use cluaizdb_types::NeuronId;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_write_and_query_neuron() {
    common::wait_for_server().await;
    let client = common::get_client();

    println!("Testing POST /neuron...");
    
    // Create dummy payload matching Python test
    let vector_data = vec![0.1; 16];
    let payload = json!({
        "raw_payload": "Hello from Native Rust API Tests!",
        "vector_data": vector_data,
        "model_creator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "payload_type": "text",
        "dna": null,
        "adjacency": null
    });

    let res = client.post(&format!("{}/neuron", common::BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST request");

    assert!(res.status().is_success(), "Failed to write neuron, status: {}", res.status());

    let res_json: serde_json::Value = res.json().await.expect("Failed to parse response JSON");
    println!("SUCCESS: {}", res_json);

    let neuron_id_str = res_json["neuron_id"].as_str().expect("No neuron_id in response");
    
    // Wait briefly for WAL flush (Durability::Lite)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("Testing POST /query (CDQL)...");
    
    let query_payload = json!({
        "cdql": "find *"
    });

    let res = client.post(&format!("{}/query", common::BASE_URL))
        .json(&query_payload)
        .send()
        .await
        .expect("Failed to send query request");

    assert!(res.status().is_success(), "Failed to query neurons, status: {}", res.status());

    let results: Vec<serde_json::Value> = res.json().await.expect("Failed to parse query results");
    
    let found = results.iter().any(|r| {
        r["neuron"]["id"].as_str().unwrap_or("") == neuron_id_str
    });

    assert!(found, "Neuron {} not found in query results!", neuron_id_str);
    println!("SUCCESS: Neuron {} found in query results!", neuron_id_str);
}
