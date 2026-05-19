use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub gemini_key: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
    pub qdrant_collection: String,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let gemini_key = env::var("GEMINI_KEY")
            .map_err(|_| "GEMINI_KEY environment variable not set".to_string())?;

        let mut qdrant_url = env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6334".to_string());

        // Qdrant Cloud gRPC requires port 6334; append if missing on cloud hosts
        if qdrant_url.contains("cloud.qdrant.io") && !qdrant_url.contains(":633") {
            qdrant_url.push_str(":6334");
        }

        let qdrant_api_key = env::var("QDRANT_API_KEY")
            .unwrap_or_default();

        let qdrant_collection = env::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "rag_embeddings".to_string());

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid SERVER_PORT: {}", e))?;

        Ok(Self {
            gemini_key,
            qdrant_url,
            qdrant_api_key,
            qdrant_collection,
            server_port,
        })
    }
}
