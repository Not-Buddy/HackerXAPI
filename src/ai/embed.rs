use crate::pdf::chunk_paras;
use crate::ai::gemini::embed_text_google; // You should implement this in gemini.rs as described earlier
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkEmbedding {
    pub chunk: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
}

/// Given the PDF text, chunk it and embed each chunk using Gemini's embedding API.
/// Returns a vector of ChunkEmbedding structs.
pub async fn embed_pdf_chunks(
    pdf_text: &str,
    api_key: &str,
    max_paragraphs: usize,
) -> Result<Vec<ChunkEmbedding>, anyhow::Error> {
    let chunks = chunk_paras(pdf_text, max_paragraphs);
    let mut results = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let embedding = embed_text_google(chunk, api_key).await?;
        let metadata = serde_json::json!({
            "chunk_index": i,
            "length": chunk.len(),
        });
        results.push(ChunkEmbedding {
            chunk: chunk.clone(),
            embedding,
            metadata,
        });
    }

    Ok(results)
}

// Embed each question - used in - fn find_relevant_chunks
pub async fn embed_question(question: &str, api_key: &str) -> Result<Vec<f32>, anyhow::Error> {
    embed_text_google(question, api_key).await
}

// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// input the question and the chunk embeddings, return the top N relevant chunks
pub async fn find_relevant_chunks<'a>(
    question: &str,
    chunk_embeddings: &'a [ChunkEmbedding],
    api_key: &str,
    top_n: usize,
) -> Result<Vec<&'a ChunkEmbedding>, anyhow::Error> {
    let q_embedding = embed_question(question, api_key).await?;
    let mut scored: Vec<(&ChunkEmbedding, f32)> = chunk_embeddings
        .iter()
        .map(|chunk| (chunk, cosine_similarity(&q_embedding, &chunk.embedding)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(scored.into_iter().take(top_n).map(|(chunk, _)| chunk).collect())
}