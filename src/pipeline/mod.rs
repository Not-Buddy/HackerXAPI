pub mod download;
pub mod url;

use crate::ai::traits::{EmbeddingProvider, LlmProvider};
use crate::config::Config;
use crate::error::Result;
use crate::extraction::TextExtractor;
use crate::vectordb::{ChunkEmbedding, VectorStore};

const CHUNK_SIZE: usize = 33000;

pub struct Pipeline {
    config: Config,
    vector_store: Box<dyn VectorStore>,
    embed_provider: Box<dyn EmbeddingProvider>,
    llm_provider: Box<dyn LlmProvider>,
    extractors: Vec<Box<dyn TextExtractor>>,
}

impl Pipeline {
    pub fn new(
        config: Config,
        vector_store: Box<dyn VectorStore>,
        embed_provider: Box<dyn EmbeddingProvider>,
        llm_provider: Box<dyn LlmProvider>,
        extractors: Vec<Box<dyn TextExtractor>>,
    ) -> Self {
        Self {
            config,
            vector_store,
            embed_provider,
            llm_provider,
            extractors,
        }
    }

    pub async fn process_document(&self, url: &str) -> Result<String> {
        let filename = url::generate_filename_from_url(url).await?;
        let file_path = format!("pdfs/{}", filename);

        if !std::path::Path::new(&file_path).exists() {
            if let Some(parent) = std::path::Path::new(&file_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            download::download_file(url, &file_path).await?;
        }

        let path = std::path::Path::new(&file_path);
        let mut extracted = None;

        for extractor in &self.extractors {
            let exts = extractor.supported_extensions();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if exts.contains(&ext) {
                    extracted = Some(extractor.extract_text(path)?);
                    break;
                }
            }
        }

        extracted.ok_or_else(|| {
            crate::error::AppError::UnsupportedFormat(
                "No extractor found for file format".into(),
            )
        })
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        text.chars()
            .collect::<Vec<char>>()
            .chunks(CHUNK_SIZE)
            .map(|chunk| chunk.iter().collect::<String>())
            .filter(|chunk| !chunk.trim().is_empty())
            .collect()
    }

    pub async fn embed_and_store(&self, doc_id: &str, text: &str) -> Result<Vec<(String, Vec<f32>)>> {
        if self.vector_store.embeddings_exist(doc_id).await? {
            let stored = self.vector_store.get_embeddings(doc_id).await?;
            return Ok(stored
                .into_iter()
                .map(|c| (c.chunk_text, c.embedding))
                .collect());
        }

        let chunks = self.chunk_text(text);
        let mut chunk_embeddings: Vec<ChunkEmbedding> = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let embedding = self.embed_provider.embed(chunk).await?;
            chunk_embeddings.push(ChunkEmbedding {
                chunk_text: chunk.clone(),
                chunk_index: i as u32,
                embedding,
            });
        }

        self.vector_store
            .store_embeddings(doc_id, &chunk_embeddings)
            .await?;

        Ok(chunk_embeddings
            .into_iter()
            .map(|c| (c.chunk_text, c.embedding))
            .collect())
    }

    pub async fn search_similar(
        &self,
        question: &str,
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<String>> {
        let embedding = self.embed_provider.embed(question).await?;
        let results = self
            .vector_store
            .search_similar(&embedding, top_k, threshold)
            .await?;

        Ok(results.into_iter().map(|s| s.chunk_text).collect())
    }

    pub async fn generate_answer(
        &self,
        context: &str,
        questions: &[String],
    ) -> Result<Vec<String>> {
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
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| crate::error::AppError::Json(e))?;

        let answers = parsed
            .get("answers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(answers)
    }
}
