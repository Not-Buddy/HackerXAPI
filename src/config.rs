use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub gemini_key: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
    pub qdrant_collection: String,
    pub server_port: u16,
    pub chunk_size: usize,
    pub top_k: usize,
    pub similarity_threshold: f32,
    pub embed_model_override: Option<String>,
    pub llm_model_override: Option<String>,
    pub auto_discover_models: bool,

    // Storage
    pub storage_backend: String,        // "local" or "r2"
    pub storage_local_dir: String,      // default "./data/files"
    pub r2_account_id: Option<String>,
    pub r2_access_key: Option<String>,
    pub r2_secret_key: Option<String>,
    pub r2_bucket: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let gemini_key = env::var("GEMINI_KEY")
            .map_err(|_| "GEMINI_KEY environment variable not set".to_string())?;

        let mut qdrant_url = env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6334".to_string());

        if qdrant_url.contains("cloud.qdrant.io") && !qdrant_url.contains(":633") {
            qdrant_url.push_str(":6334");
        }

        let qdrant_api_key = env::var("QDRANT_API_KEY").unwrap_or_default();
        let qdrant_collection = env::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "rag_embeddings".to_string());

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid SERVER_PORT: {}", e))?;

        let chunk_size = env::var("CHUNK_SIZE")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<usize>()
            .unwrap_or(8000);

        let top_k = env::var("TOP_K")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .unwrap_or(10);

        let similarity_threshold = env::var("SIMILARITY_THRESHOLD")
            .unwrap_or_else(|_| "0.3".to_string())
            .parse::<f32>()
            .unwrap_or(0.3);

        let embed_model_override = env::var("EMBED_MODEL").ok()
            .filter(|s| s != "auto" && !s.is_empty());

        let llm_model_override = env::var("LLM_MODEL").ok()
            .filter(|s| s != "prompt" && !s.is_empty());

        let auto_discover_models = env::var("AUTO_DISCOVER_MODELS")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let storage_backend = env::var("STORAGE_BACKEND")
            .unwrap_or_else(|_| "local".to_string());
        let storage_local_dir = env::var("STORAGE_LOCAL_DIR")
            .unwrap_or_else(|_| "./data/files".to_string());
        let r2_account_id = env::var("R2_ACCOUNT_ID").ok();
        let r2_access_key = env::var("R2_ACCESS_KEY_ID").ok();
        let r2_secret_key = env::var("R2_SECRET_ACCESS_KEY").ok();
        let r2_bucket = env::var("R2_BUCKET").ok();

        Ok(Self {
            gemini_key,
            qdrant_url,
            qdrant_api_key,
            qdrant_collection,
            server_port,
            chunk_size,
            top_k,
            similarity_threshold,
            embed_model_override,
            llm_model_override,
            auto_discover_models,
            storage_backend,
            storage_local_dir,
            r2_account_id,
            r2_access_key,
            r2_secret_key,
            r2_bucket,
        })
    }
}
