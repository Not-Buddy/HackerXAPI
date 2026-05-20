use async_trait::async_trait;
use std::time::Instant;

use crate::error::{AppError, Result};
use crate::ai::traits::EmbeddingProvider;
use super::client::{retry_delay, parse_retry_after};
use super::types::{EmbedContentPart, EmbedRequest, EmbedResponse, TextPart};

#[derive(Clone)]
pub struct EmbedClient {
    pub api_key: String,
    pub client: reqwest::Client,
    pub model: String,
}

#[async_trait]
impl EmbeddingProvider for EmbedClient {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request_body = EmbedRequest {
            model: self.model.clone(),
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
            self.model
        );

        let mut attempts: u32 = 0;
        let total_start = Instant::now();
        let info = format!("{}B chunk (model: {})", text.len(), self.model);

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
                let delay = retry_delay(attempts, &headers);
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
        &self.model
    }
}
