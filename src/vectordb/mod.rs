use async_trait::async_trait;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct ChunkEmbedding {
    pub chunk_text: String,
    pub chunk_index: u32,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk_text: String,
    pub chunk_index: u32,
    pub score: f32,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_embeddings(&self, doc_id: &str, chunks: &[ChunkEmbedding]) -> Result<()>;
    async fn get_embeddings(&self, doc_id: &str) -> Result<Vec<ChunkEmbedding>>;
    async fn embeddings_exist(&self, doc_id: &str) -> Result<bool>;
    async fn search_similar(
        &self,
        embedding: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<ScoredChunk>>;
}

pub mod qdrant;
