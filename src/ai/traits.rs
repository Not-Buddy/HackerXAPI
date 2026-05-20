use async_trait::async_trait;
use crate::error::Result;
use serde_json::Value;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str, schema: Option<Value>) -> Result<String>;
}
