use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;

use crate::error::{AppError, Result};
use crate::ai::traits::LlmProvider;
use super::client::{retry_delay, parse_retry_after};
use super::safety::sanitize_policy;
use super::types::{ContentsPart, GeminiRequest, GenerationConfig, TextPart};

#[derive(Clone)]
pub struct LlmClient {
    pub api_key: String,
    pub client: reqwest::Client,
    pub model: String,
}

#[async_trait]
impl LlmProvider for LlmClient {
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
            "https://generativelanguage.googleapis.com/v1beta/{}:generateContent",
            self.model
        );

        let mut attempts: u32 = 0;
        let total_start = Instant::now();
        let info = format!("{}B prompt (model: {})", prompt.len(), self.model);

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
                let delay = retry_delay(attempts, &headers);
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
