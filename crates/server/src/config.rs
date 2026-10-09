use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub database: DatabaseSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSection {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSection {
    pub concurrency_mode: String, // "dashmap" | "mutex"
    pub payload_format: String,    // "json" | "protobuf" | "flatbuffers"
}

impl ServerConfig {
    pub fn load() -> Self {
        std::fs::read_to_string("cluaizdb.toml")
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_else(|| {
                tracing::warn!("Could not read cluaizdb.toml, using defaults");
                ServerConfig {
                    server: ServerSection {
                        port: 7331,
                        host: "0.0.0.0".into(),
                    },
                    database: DatabaseSection {
                        concurrency_mode: "dashmap".into(),
                        payload_format: "json".into(),
                    },
                }
            })
    }
}
