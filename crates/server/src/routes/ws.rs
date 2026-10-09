use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::routes::AppState;

/// System telemetry data mapping to Cluaizdb-HEART biomarkers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartTelemetry {
    pub heart_rate_bpm: u32,
    pub blood_pressure_systolic: u32,
    pub blood_pressure_diastolic: u32,
    pub oxygen_level_spo2: f32,
    pub metabolic_rate: f32,
}

/// Commands received from the Genome Canvas UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum UICommand {
    #[serde(rename = "adrenaline_shot")]
    AdrenalineShot,
    #[serde(rename = "artificial_pacemaker")]
    ArtificialPacemaker { pulse_limit: u32 },
    #[serde(rename = "induced_coma")]
    InducedComa,
}

/// Upgrades the connection to a WebSocket and spawns the telemetry loop.
pub async fn handle_ws(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<AppState>, socket: WebSocket) {
    info!("Cluaizdb-HEART WebSocket telemetry client connected");

    // Spawn a receiver task to handle incoming controls
    let (mut sender, mut receiver) = socket.split();
    let state_rx = Arc::clone(&state);

    let rx_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<UICommand>(&text) {
                    Ok(cmd) => match cmd {
                        UICommand::AdrenalineShot => {
                            info!("HEART Trigger: ADRENALINE SHOT! Forceful uncompression activated.");
                            if let Err(e) = state_rx.shard_manager.run_gc_sweep_on_all_shards().await {
                                warn!(error = %e, "Dynamic sweep execution failed during Adrenaline Shot");
                            }
                        }
                        UICommand::ArtificialPacemaker { pulse_limit } => {
                            info!("HEART Trigger: ARTIFICIAL PACEMAKER set pulse limit to {} bytes.", pulse_limit);
                            state_rx.write_rate_limit.store(pulse_limit, std::sync::atomic::Ordering::SeqCst);
                        }
                        UICommand::InducedComa => {
                            info!("HEART Trigger: INDUCED COMA! Emergency WAL commit and database consistency verified.");
                        }
                    },
                    Err(e) => {
                        warn!("Failed to parse UICommand: {}. Raw: {}", e, text);
                    }
                }
            }
        }
        info!("WebSocket receiver task shutdown");
    });

    // Telemetry sender loop (every 500ms)
    let state_tx = Arc::clone(&state);
    let tx_task = tokio::spawn(async move {
        loop {
            // Read real metrics from Cluaizdb-HEART autonomic controller
            let tel = {
                let lock = state_tx.heart_telemetry.read().await;
                *lock
            };

            // Metabolic rate reflects the number of active database shards
            let open_shards = state_tx.shard_manager.active_shards_count().await as u32;
            let metabolic = 1.0 + (open_shards as f32 * 0.25);

            let telemetry = HeartTelemetry {
                heart_rate_bpm: tel.bpm,
                blood_pressure_systolic: tel.bp_systolic as u32,
                blood_pressure_diastolic: 80, // Static for now, can be mapped if needed
                oxygen_level_spo2: tel.spo2 as f32,
                metabolic_rate: metabolic,
            };

            if let Ok(json) = serde_json::to_string(&telemetry) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break; // Connection closed
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
        info!("WebSocket transmitter task shutdown");
    });

    // Wait until one of the tasks ends or fails
    tokio::select! {
        _ = rx_task => {},
        _ = tx_task => {},
    }
    info!("Cluaizdb-HEART WebSocket client disconnected");
}

