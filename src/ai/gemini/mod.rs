pub mod client;
pub mod embed;
pub mod llm;
pub mod models;
pub mod safety;
pub mod types;

use async_trait::async_trait;
use crate::error::Result;
use super::traits::{EmbeddingProvider, LlmProvider};
use client::build_client;
use embed::EmbedClient;
use llm::LlmClient;
use models::{discover_models, pick_embed_model, pick_llm_model_interactive, find_model};

#[derive(Clone)]
pub struct GeminiProvider {
    pub embed_client: EmbedClient,
    pub llm_client: LlmClient,
}

impl GeminiProvider {
    /// Build provider with interactive model selection.
    /// - Queries Gemini API for available models
    /// - Auto-selects text-embedding-004 for embeddings
    /// - Prompts user to choose an LLM model
    /// - Returns (provider, vector_size_for_qdrant)
    pub async fn new_interactive(api_key: String) -> Result<(Self, u64)> {
        let client = build_client();

        println!("Discovering Gemini models...");
        let available_models = discover_models(&api_key).await?;

        let embed_count = available_models.iter().filter(|m| m.supports_embed()).count();
        let gen_count = available_models.iter().filter(|m| m.supports_generate()).count();
        println!("Found {} models, {} embedding, {} generation", available_models.len(), embed_count, gen_count);

        // Auto-pick embedding model
        let embed_model = pick_embed_model(&available_models)
            .ok_or_else(|| crate::error::AppError::Embedding(
                "No embedding-capable model found".into()
            ))?;
        let embed_dim = embed_model.output_dimensionality.unwrap_or(768);
        println!("Embedding: {} ({} dims) [auto-selected]", embed_model.name, embed_dim);

        let embed_client = EmbedClient {
            api_key: api_key.clone(),
            client: client.clone(),
            model: embed_model.name.clone(),
        };

        // Probe actual embedding dimension (API metadata can be stale)
        let probe_vec = embed_client.embed("dimension probe").await?;
        let actual_dim = probe_vec.len() as u64;
        if actual_dim != embed_dim as u64 {
            println!(
                "  Note: API reports {} dims but actual output is {} dims — using actual.",
                embed_dim, actual_dim
            );
        }

        // Interactive LLM selection
        let llm_name = pick_llm_model_interactive(&available_models)
            .ok_or_else(|| crate::error::AppError::Llm(
                "No generation-capable model found".into()
            ))?;

        Ok((Self {
            embed_client,
            llm_client: LlmClient {
                api_key: api_key.clone(),
                client,
                model: llm_name,
            },
        }, actual_dim))
    }

    /// Build provider with explicit model names (no API discovery).
    pub fn new_explicit(
        api_key: String,
        embed_model: String,
        llm_model: String,
    ) -> Self {
        let client = build_client();
        Self {
            embed_client: EmbedClient {
                api_key: api_key.clone(),
                client: client.clone(),
                model: embed_model,
            },
            llm_client: LlmClient {
                api_key: api_key.clone(),
                client,
                model: llm_model,
            },
        }
    }

    /// Build provider from config: if LLM_MODEL=prompt or AUTO_DISCOVER=true, use interactive mode.
    pub async fn from_config(
        api_key: String,
        embed_cfg: Option<String>,
        llm_cfg: Option<String>,
        auto_discover: bool,
    ) -> Result<(Self, u64)> {
        if auto_discover || llm_cfg.as_deref() == Some("prompt") || embed_cfg.as_deref() == Some("auto") {
            return Self::new_interactive(api_key).await;
        }

        let embed_model = embed_cfg.unwrap_or_else(|| "models/text-embedding-004".into());
        let llm_model = llm_cfg.unwrap_or_else(|| "gemini-2.5-flash-lite".into());

        // If not discovering, try a quick discovery just to get the embedding dimension
        let embed_dim = match discover_models(&api_key).await {
            Ok(models) => {
                find_model(&models, &embed_model)
                    .and_then(|m| m.output_dimensionality)
                    .unwrap_or(768)
            }
            Err(_) => 768,
        };

        println!("Embedding: {} ({} dims) [from config]", embed_model, embed_dim);
        println!("LLM: {} [from config]", llm_model);

        Ok((Self::new_explicit(api_key, embed_model, llm_model), embed_dim as u64))
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_client.embed(text).await
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate(&self, prompt: &str, schema: Option<serde_json::Value>) -> Result<String> {
        self.llm_client.generate(prompt, schema).await
    }
}
