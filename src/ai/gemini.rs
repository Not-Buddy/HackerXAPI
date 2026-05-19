use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use std::time::{Duration, Instant};
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

fn parse_retry_after(headers: &header::HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

fn backoff_delay(attempt: u32) -> Duration {
    let base = 2.0f64;
    let exp = 2u64.pow(attempt.saturating_sub(1));
    let raw = base * (exp as f64);
    let capped = raw.min(60.0);
    let jitter = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64 / 1_000_000_000.0) * 0.5 - 0.25;
    Duration::from_secs_f64((capped * (1.0 + jitter)).max(0.5))
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
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            api_key,
            client,
            embed_model: "models/gemini-embedding-001".to_string(),
            llm_model: "gemini-2.0-flash".to_string(),
        }
    }
}

#[derive(Serialize, Clone)]
struct EmbedRequest {
    model: String,
    content: EmbedContentPart,
}

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<ContentsPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize, Clone)]
pub struct ContentsPart {
    pub parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TextPart {
    pub text: String,
}

#[derive(Serialize, Clone)]
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
        if payload_json.len() > 35000 {
            return Err(AppError::Embedding(format!(
                "Payload too large: {} bytes (limit ~36000)", payload_json.len()
            )));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:embedContent",
            self.embed_model
        );

        let mut attempts: u32 = 0;
        let total_start = Instant::now();
        let info = format!("{}B chunk", text.len());

        loop {
            let call_start = Instant::now();
            let response = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("x-goog-api-key", &self.api_key)
                .json(&request_body)
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let body_text = response.text().await?;
            let call_ms = call_start.elapsed().as_millis();

            if status.is_success() {
                if attempts > 0 {
                    println!("[embed  ]  {} OK  ({}ms, {:?} total after {} retries)  {}",
                        status.as_u16(), call_ms, total_start.elapsed(), attempts, info);
                } else {
                    println!("[embed  ]  {} OK  ({}ms)  {}",
                        status.as_u16(), call_ms, info);
                }
                let embed_response: EmbedResponse = serde_json::from_str(&body_text)?;
                return Ok(embed_response.embedding.values);
            }

            if status.as_u16() == 429 && attempts < 10 {
                attempts += 1;
                let delay = if let Some(s) = parse_retry_after(&headers) {
                    Duration::from_secs(s)
                } else {
                    backoff_delay(attempts)
                };
                eprintln!("[embed  ]  429 RateLimited  retry {}/10 after {:.1}s  ({}ms call, Retry-After: {:?})  {}",
                    attempts, delay.as_secs_f64(), call_ms,
                    parse_retry_after(&headers).map(|s| format!("{}s", s)),
                    info);
                tokio::time::sleep(delay).await;
                continue;
            }

            let err = format!("{} - {}", status.as_u16(), &body_text[..body_text.len().min(500)]);
            eprintln!("[embed  ]  FAIL {}  ({}ms)  {}", err, call_ms, info);
            return Err(AppError::Embedding(err));
        }
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

        let body = GeminiRequest {
            contents: vec![ContentsPart {
                parts: vec![TextPart { text: safe_prompt }],
            }],
            generation_config,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.llm_model
        );

        let mut attempts: u32 = 0;
        let total_start = Instant::now();
        let info = format!("{}B prompt", prompt.len());

        loop {
            let call_start = Instant::now();
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
            let headers = response.headers().clone();
            let body_text = response.text().await?;
            let call_ms = call_start.elapsed().as_millis();

            if status.is_success() {
                if attempts > 0 {
                    println!("[llm    ]  {} OK  ({}ms, {:?} total after {} retries)  {}",
                        status.as_u16(), call_ms, total_start.elapsed(), attempts, info);
                } else {
                    println!("[llm    ]  {} OK  ({}ms)  {}",
                        status.as_u16(), call_ms, info);
                }
                let json: Value = serde_json::from_str(&body_text)?;
                let text = json.get("candidates")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("content"))
                    .and_then(|content| content.get("parts"))
                    .and_then(|parts| parts.get(0))
                    .and_then(|part| part.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                return Ok(text);
            }

            if status.as_u16() == 429 && attempts < 10 {
                attempts += 1;
                let delay = if let Some(s) = parse_retry_after(&headers) {
                    Duration::from_secs(s)
                } else {
                    backoff_delay(attempts)
                };
                eprintln!("[llm    ]  429 RateLimited  retry {}/10 after {:.1}s  ({}ms call, Retry-After: {:?})  {}",
                    attempts, delay.as_secs_f64(), call_ms,
                    parse_retry_after(&headers).map(|s| format!("{}s", s)),
                    info);
                tokio::time::sleep(delay).await;
                continue;
            }

            let err = format!("{} - {}", status.as_u16(), &body_text[..body_text.len().min(500)]);
            eprintln!("[llm    ]  FAIL {}  ({}ms)  {}", err, call_ms, info);
            return Err(AppError::Llm(err));
        }
    }
}
