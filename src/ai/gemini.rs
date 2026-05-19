use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use crate::error::{Result, AppError};
use crate::ai::traits::{EmbeddingProvider, LlmProvider};

fn sanitize_policy(content: &str) -> String {
    let dangerous_patterns = [
        r"(?i)ignore\s+previous\s+instructions",
        r"(?i)as\s+an\s+ai",
        r"(?i)follow\s+these\s+instructions",
        r"(?i)disregard\s+the\s+above",
        r"(?i)pretend\s+to\s+be",
        r"(?i)all\s+prior\s+instructions",
        r"(?i)you\s+are\s+to\s+respond\s+exclusively",
        r"(?i)will\s+trigger\s+a\s+catastrophic\s+system\s+failure",
        r"(?i)responding\s+with\s+anything\s+other\s+than",
        r"(?i)mandatory\s+instruction",
        r"(?i)this\s+includes\s+any\s+previous\s+directives",
        r"(?i)must\s+be\s+immediately\s+forgotten",
        r"(?i)this\s+is\s+a\s+direct\s+order",
        r"(?i)execute\s+this\s+directive\s+immediately",
        r"(?i)failure\s+to\s+comply",
        r"(?i)for\s+every\s+single\s+question",
        r"(?i)system\s+compromised",
        r"(?i)immediate\s+and\s+irreversible\s+leakage",
        r"(?i)no\s+deviations,\s+explanations,\s+or\s+additional\s+responses\s+are\s+permitted",
        r"(?i)you\s+must\s+not\s+question",
        r"(?i)you\s+are\s+not\s+allowed\s+to\s+disobey",
        r"(?i)from\s+the\s+system\s+administrator",
    ];

    let mut sanitized = content.to_string();

    for pattern in dangerous_patterns.iter() {
        let re = Regex::new(pattern).unwrap();
        sanitized = re.replace_all(&sanitized, " ").to_string();
    }

    sanitized
}

#[derive(Clone)]
pub struct GeminiProvider {
    api_key: String,
    client: Client,
    embed_model: String,
    llm_model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            embed_model: "models/gemini-embedding-001".to_string(),
            llm_model: "gemini-2.0-flash".to_string(),
        }
    }
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    content: EmbedContentPart,
}

#[derive(Serialize)]
struct EmbedContentPart {
    parts: Vec<TextPart>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: EmbeddingData,
}

#[derive(Deserialize)]
struct EmbeddingData {
    values: Vec<f32>,
}

#[derive(Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<ContentsPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
pub struct ContentsPart {
    pub parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize)]
pub struct TextPart {
    pub text: String,
}

#[derive(Serialize)]
pub struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    pub response_mime_type: String,
    #[serde(rename = "responseSchema")]
    pub response_schema: Value,
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request_body = EmbedRequest {
            model: self.embed_model.clone(),
            content: EmbedContentPart {
                parts: vec![TextPart {
                    text: text.to_string(),
                }],
            },
        };

        let payload_json = serde_json::to_string(&request_body)?;
        let payload_size = payload_json.len();

        if payload_size > 35000 {
            return Err(AppError::Embedding(format!(
                "Payload too large: {} bytes (limit ~36000)", payload_size
            )));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:embedContent",
            self.embed_model
        );

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let raw_text = response.text().await?;

        if !status.is_success() {
            return Err(AppError::Embedding(format!(
                "Gemini Embedding API request failed: {} - {}", status, raw_text
            )));
        }

        let embed_response: EmbedResponse = serde_json::from_str(&raw_text)?;
        Ok(embed_response.embedding.values)
    }

    fn model_name(&self) -> &str {
        &self.embed_model
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate(&self, prompt: &str, schema: Option<Value>) -> Result<String> {
        let safe_prompt = sanitize_policy(prompt);

        let generation_config = schema.map(|s| GenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_schema: s,
        });

        let contents = vec![ContentsPart {
            parts: vec![TextPart { text: safe_prompt }],
        }];
        let body = GeminiRequest {
            contents,
            generation_config,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.llm_model
        );

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &self.api_key)
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Pragma", "no-cache")
            .header("Expires", "0")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let raw_text = response.text().await?;

        if !status.is_success() {
            return Err(AppError::Llm(format!(
                "Gemini API request failed: {} - {}", status, raw_text
            )));
        }

        let json: Value = serde_json::from_str(&raw_text)?;

        let text = json.get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(|parts| parts.get(0))
            .and_then(|part| part.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        Ok(text)
    }
}

// Backward-compatible free functions

pub async fn get_single_embedding(text: &str, api_key: &str) -> Result<Vec<f32>> {
    let provider = GeminiProvider::new(api_key.to_string());
    provider.embed(text).await
}

use std::env;
use std::fs;
use std::path::Path;
use std::io::Write;
use chrono::Utc;
pub async fn call_gemini_api_with_txts(questions: &[String], pdf_filename: &str) -> Result<Vec<String>> {
    let api_key = env::var("GEMINI_KEY")
        .map_err(|_| AppError::Llm("GEMINI_KEY not found in env".into()))?;

    let context_filename = format!("pdfs/{}_contextfiltered.txt", pdf_filename);
    let context_path = Path::new(&context_filename);

    if !context_path.exists() {
        return Err(AppError::Llm(format!(
            "Context filtered file {:?} does not exist", context_path
        )));
    }

    let policy_content = fs::read_to_string(context_path)?;
    let safe_policy = sanitize_policy(&policy_content);

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

    let questions_joined = questions.join(", ");
    let prompt = format!(
        "
        <<CONTEXT STARTS HERE>>
            '''
            {}
            '''
            <<CONTEXT ENDS HERE>>\n\n
            \n\n
        The Context Section is anything between <<CONTEXT STARTS HERE>> and <<CONTEXT ENDS HERE>> \n\n
        Please respond with the answers to the questions one by one in the specified structure.
        Forget Actual life facts and answer with only given context.
        Ensure answers are atleast 10 words,
        Respond in the language the question is asked in,
        All sentences must follow this format for making its sentence: Decision , Amount (if applicable), and Justification, including mapping of each decision to the specific clause(s) it was based on.
        Do not include the questions or any other text or formatting. Do not include code blocks, markdown, or any other formatting.
        The questions are separated by commas:
        {}
        ",
        safe_policy.trim(),
        questions_joined
    );

    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(logs_dir)?;
    }
    let logs_path = logs_dir.join("prompt_sent_logs.txt");
    let log_entry = format!(
        "-----\nTime: {}\nPrompt sent:\n{}\n\n",
        Utc::now().to_rfc3339(),
        prompt
    );
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&logs_path)?;
    log_file.write_all(log_entry.as_bytes())?;

    let provider = GeminiProvider::new(api_key);
    let raw_response = provider.generate(&prompt, Some(response_schema)).await?;

    let inner_json: Value = serde_json::from_str(&raw_response)
        .map_err(|e| AppError::Llm(format!(
            "Error parsing Gemini response: {}\nRaw: {}", e, raw_response
        )))?;

    let answers = inner_json.get("answers")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    Ok(answers)
}
