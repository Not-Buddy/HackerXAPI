use futures::stream::{self, StreamExt};
use crate::ai::traits::EmbeddingProvider;
use crate::ai::chunking::chunk_text;
use crate::vectordb::{VectorStore, ChunkEmbedding};
use crate::error::{Result, AppError};

pub async fn process_document_embeddings(
    doc_id: &str,
    text: &str,
    chunk_size: usize,
    parallel_requests: usize,
    embed_provider: &dyn EmbeddingProvider,
    vector_store: &dyn VectorStore,
) -> Result<Vec<(String, Vec<f32>)>> {
    if vector_store.embeddings_exist(doc_id).await? {
        let chunks = vector_store.get_embeddings(doc_id).await?;
        return Ok(chunks.into_iter()
            .map(|c| (c.chunk_text, c.embedding))
            .collect());
    }

    let chunks = chunk_text(text, chunk_size);

    let chunk_embeddings: Vec<_> = stream::iter(chunks.into_iter().enumerate())
        .map(|(i, chunk)| async move {
            let embedding = embed_provider.embed(&chunk).await?;
            Ok::<(usize, String, Vec<f32>), AppError>((i, chunk, embedding))
        })
        .buffer_unordered(parallel_requests)
        .collect::<Vec<_>>()
        .await;

    let mut results = Vec::new();
    let mut store_chunks = Vec::new();

    for chunk_result in chunk_embeddings {
        let (index, chunk_text, embedding) = chunk_result?;
        results.push((chunk_text.clone(), embedding.clone()));
        store_chunks.push(ChunkEmbedding {
            chunk_text,
            chunk_index: index as u32,
            embedding,
        });
    }

    vector_store.store_embeddings(doc_id, &store_chunks).await?;

    Ok(results)
}
