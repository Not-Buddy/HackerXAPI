pub mod download;
pub mod url;

use std::time::Duration;

use crate::ai::traits::{EmbeddingProvider, LlmProvider};
use crate::config::Config;
use crate::error::Result;
use crate::extraction::TextExtractor;
use crate::storage::{StorageBackend, StoredFile};
use crate::vectordb::{ChunkEmbedding, VectorStore};

#[derive(Default)]
pub struct ApiCounters {
    pub embed_calls: usize,
    pub llm_calls: usize,
    pub retries: usize,
}

pub struct Pipeline {
    config: Config,
    storage: Box<dyn StorageBackend>,
    vector_store: Box<dyn VectorStore>,
    embed_provider: Box<dyn EmbeddingProvider>,
    llm_provider: Box<dyn LlmProvider>,
    extractors: Vec<Box<dyn TextExtractor>>,
}

impl Pipeline {
    pub fn new(
        config: Config,
        storage: Box<dyn StorageBackend>,
        vector_store: Box<dyn VectorStore>,
        embed_provider: Box<dyn EmbeddingProvider>,
        llm_provider: Box<dyn LlmProvider>,
        extractors: Vec<Box<dyn TextExtractor>>,
    ) -> Self {
        Self {
            config,
            storage,
            vector_store,
            embed_provider,
            llm_provider,
            extractors,
        }
    }

    #[allow(dead_code)]
    pub fn chunk_size(&self) -> usize {
        self.config.chunk_size
    }

    #[allow(dead_code)]
    pub fn top_k(&self) -> usize {
        self.config.top_k
    }

    #[allow(dead_code)]
    pub fn threshold(&self) -> f32 {
        self.config.similarity_threshold
    }

    pub async fn process_document(&self, url: &str) -> Result<(StoredFile, String)> {
        let original_name = url::extract_filename_from_url(url)?;

        let bytes = download::download_bytes(url).await?;
        let stored = StoredFile::new(&original_name, bytes.len() as u64);

        if !self.storage.exists(&stored.storage_key).await? {
            self.storage
                .put(&stored.storage_key, &bytes, &stored.mime_type)
                .await?;
        }

        let local_path = self.storage.get_local_path(&stored.storage_key).await?;
        let text = self.extract_from_path(&local_path, stored.extension())?;

        Ok((stored, text))
    }

    fn extract_from_path(&self, path: &std::path::Path, ext: &str) -> Result<String> {
        for extractor in &self.extractors {
            if extractor.supported_extensions().contains(&ext) {
                return extractor.extract_text(path);
            }
        }
        Err(crate::error::AppError::UnsupportedFormat(format!(
            "No extractor found for .{}", ext
        )))
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        text.chars()
            .collect::<Vec<char>>()
            .chunks(self.config.chunk_size)
            .map(|chunk| chunk.iter().collect::<String>())
            .filter(|chunk| !chunk.trim().is_empty())
            .collect()
    }

    pub async fn embed_and_store(
        &self,
        doc_id: &str,
        text: &str,
    ) -> Result<(Vec<(String, Vec<f32>)>, ApiCounters)> {
        let mut counters = ApiCounters::default();

        if self.vector_store.embeddings_exist(doc_id).await? {
            let stored = self.vector_store.get_embeddings(doc_id).await?;
            return Ok((stored
                .into_iter()
                .map(|c| (c.chunk_text, c.embedding))
                .collect(), counters));
        }

        let chunks = self.chunk_text(text);
        let total = chunks.len();
        let mut chunk_embeddings: Vec<ChunkEmbedding> = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let embedding = self.embed_provider.embed(chunk).await?;
            counters.embed_calls += 1;

            chunk_embeddings.push(ChunkEmbedding {
                chunk_text: chunk.clone(),
                chunk_index: i as u32,
                embedding,
            });

            if i + 1 < total {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        self.vector_store
            .store_embeddings(doc_id, &chunk_embeddings)
            .await?;

        Ok((chunk_embeddings
            .into_iter()
            .map(|c| (c.chunk_text, c.embedding))
            .collect(), counters))
    }

    pub async fn search_similar(
        &self,
        question: &str,
    ) -> Result<(Vec<String>, ApiCounters)> {
        let mut counters = ApiCounters::default();

        let embedding = self.embed_provider.embed(question).await?;
        counters.embed_calls += 1;

        let results = self
            .vector_store
            .search_similar(&embedding, self.config.top_k, self.config.similarity_threshold)
            .await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok((results.into_iter().map(|s| s.chunk_text).collect(), counters))
    }

    pub async fn generate_answer(
        &self,
        context: &str,
        questions: &[String],
    ) -> Result<(Vec<String>, ApiCounters)> {
        let mut counters = ApiCounters::default();

        let questions_joined = questions.join(", ");
        let prompt = format!(
            "\
<<CONTEXT STARTS HERE>>
    '''
    {}
    '''
    <<CONTEXT ENDS HERE>>

The Context Section is anything between <<CONTEXT STARTS HERE>> and <<CONTEXT ENDS HERE>>

Please respond with the answers to the questions one by one.
Forget actual life facts and answer with only given context.
Ensure answers are at least 10 words,
Respond in the language the question is asked in,
All sentences must follow this format: Decision, Amount (if applicable), and Justification.
Do not include the questions or any other text or formatting.
The questions are separated by commas:
{}
",
            context.trim(),
            questions_joined
        );

        let response_schema = serde_json::json!({
            "type": "OBJECT",
            "properties": {
                "answers": {
                    "type": "ARRAY",
                    "items": { "type": "STRING" }
                }
            },
            "required": ["answers"]
        });

        let raw = self.llm_provider.generate(&prompt, Some(response_schema)).await?;
        counters.llm_calls += 1;

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).map_err(crate::error::AppError::Json)?;

        let answers = parsed
            .get("answers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok((answers, counters))
    }
}
