use reqwest::Client;

use crate::error::{AppError, Result};
use super::types::{ModelInfo, ModelListResponse};

/// Fetch available models from Gemini API.
pub async fn discover_models(api_key: &str) -> Result<Vec<ModelInfo>> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let response = client.get(&url).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await?;
        return Err(AppError::Llm(format!(
            "Failed to discover models: {} - {}",
            status, body
        )));
    }

    let list: ModelListResponse = response.json().await.map_err(|e| {
        AppError::Llm(format!("Failed to parse model list: {}", e))
    })?;

    Ok(list.models)
}

/// Auto-select the best embedding model: prefer text-embedding-004, fallback to embedding-001.
pub fn pick_embed_model(models: &[ModelInfo]) -> Option<&ModelInfo> {
    let embed_models: Vec<_> = models.iter().filter(|m| m.supports_embed()).collect();

    // Prefer text-embedding-004 (768 dims, newer, reliable rate limits)
    if let Some(m) = embed_models.iter().find(|m| m.name.contains("text-embedding-004")) {
        return Some(m);
    }
    // Fallback: any embedding model
    embed_models.first().copied()
}

/// Print available generation models and prompt user to pick one.
pub fn pick_llm_model_interactive(models: &[ModelInfo]) -> Option<String> {
    use std::io::{self, Write};

    let gen_models: Vec<_> = models
        .iter()
        .filter(|m| m.supports_generate())
        .collect();

    if gen_models.is_empty() {
        eprintln!("No generation-capable models found!");
        return None;
    }

    println!("\nAvailable LLM models:");
    for (i, m) in gen_models.iter().enumerate() {
        println!(
            "  {}. {:38} ({:.1}M tokens)",
            i + 1,
            m.name,
            m.input_token_limit as f64 / 1_000_000.0
        );
    }

    let default_idx = gen_models
        .iter()
        .position(|m| m.name.contains("2.5-flash-lite"))
        .map(|i| i + 1)
        .unwrap_or(1);

    print!("\nSelect LLM model [1-{}] (default {}): ", gen_models.len(), default_idx);
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    let idx: usize = if input.is_empty() {
        default_idx
    } else {
        input.parse().unwrap_or(default_idx)
    };

    let chosen = gen_models
        .get(idx.saturating_sub(1))
        .or_else(|| gen_models.get(default_idx.saturating_sub(1)));

    if let Some(m) = chosen {
        println!("Selected: {}\n", m.name);
        Some(m.name.clone())
    } else {
        eprintln!("Invalid selection, using default.");
        gen_models
            .get(default_idx.saturating_sub(1))
            .map(|m| m.name.clone())
    }
}

/// Find a specific model by name in the discovered list.
pub fn find_model<'a>(models: &'a [ModelInfo], name: &str) -> Option<&'a ModelInfo> {
    models.iter().find(|m| m.name == name || m.name.ends_with(name))
}
