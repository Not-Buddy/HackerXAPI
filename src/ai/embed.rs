use std::fs;
use std::path::Path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    content: ContentPart,
}

#[derive(Serialize)]
struct ContentPart {
    parts: Vec<TextPart>,
}

#[derive(Serialize)]
struct TextPart {
    text: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: EmbeddingData,
}

#[derive(Deserialize)]
struct EmbeddingData {
    values: Vec<f32>,
}

/// Chunk text into smaller pieces (by characters, not tokens - adjust as needed)
fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    
    for paragraph in text.split("\n\n") {
        // If adding this paragraph would exceed limit, save current chunk
        if !current_chunk.is_empty() && current_chunk.len() + paragraph.len() > max_chars {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }
        
        // If single paragraph is too large, split by sentences
        if paragraph.len() > max_chars {
            for sentence in paragraph.split(". ") {
                if !current_chunk.is_empty() && current_chunk.len() + sentence.len() > max_chars {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                }
                current_chunk.push_str(sentence);
                current_chunk.push_str(". ");
            }
        } else {
            current_chunk.push_str(paragraph);
            current_chunk.push_str("\n\n");
        }
    }
    
    // Add remaining chunk
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    
    chunks
}

/// Get embedding for a single text chunk
async fn get_single_embedding(text: &str, api_key: &str) -> Result<Vec<f32>> {
    let request_body = EmbedRequest {
        model: "models/gemini-embedding-001".to_string(),
        content: ContentPart {
            parts: vec![TextPart {
                text: text.to_string(),
            }],
        },
    };

    let client = Client::new();
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent")
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let raw_text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow!("Gemini Embeddings API request failed: {} - {}", status, raw_text));
    }

    let embed_response: EmbedResponse = serde_json::from_str(&raw_text)
        .map_err(|e| anyhow!("Error deserializing embedding response: {}\nRaw response: {}", e, raw_text))?;

    Ok(embed_response.embedding.values)
}

/// Average multiple embedding vectors
fn average_embeddings(embeddings: &[Vec<f32>]) -> Vec<f32> {
    if embeddings.is_empty() {
        return Vec::new();
    }
    
    let len = embeddings[0].len();
    let mut result = vec![0.0; len];
    
    for embedding in embeddings {
        for (i, &val) in embedding.iter().enumerate() {
            result[i] += val;
        }
    }
    
    // Average by dividing by number of embeddings
    for val in result.iter_mut() {
        *val /= embeddings.len() as f32;
    }
    
    result
}

fn concatenate_embeddings(embeddings: &[Vec<f32>]) -> Vec<f32> {
    let mut result = Vec::new();
    for emb in embeddings {
        result.extend_from_slice(emb);
    }
    result
}


/// Reads policy.txt file, chunks it, gets embeddings for each chunk, and returns averaged embedding
pub async fn get_policy_embedding(api_key: &str) -> Result<Vec<f32>> {
    let policy_path = Path::new("pdfs/policy.txt");
    if !policy_path.exists() {
        return Err(anyhow!("File {:?} does not exist", policy_path));
    }
    
    let policy_content = fs::read_to_string(policy_path)?;
    
    // Chunk the text (30KB limit, use ~25KB to be safe)
    let chunks = chunk_text(&policy_content, 25000);
    println!("Split policy into {} chunks", chunks.len());
    
    let mut embeddings = Vec::new();
    
    // Get embedding for each chunk with delay to avoid rate limits
    for (i, chunk) in chunks.iter().enumerate() {
        println!("Processing chunk {} of {}", i + 1, chunks.len());
        
        let embedding = get_single_embedding(chunk, api_key).await?;
        embeddings.push(embedding);
        
        // Add delay between requests to avoid rate limiting
        if i < chunks.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }
    
    // Average all embeddings to get a single representative embedding
    // Instead of average_embeddings(&embeddings)
    let concatenated_embedding = concatenate_embeddings(&embeddings);
    println!("Created concatenated embedding with {} dimensions", concatenated_embedding.len());
    Ok(concatenated_embedding)
}

/// Alternative: Return all chunk embeddings instead of averaging
use futures::stream::{self, StreamExt};

pub async fn get_policy_chunk_embeddings(api_key: &str) -> Result<Vec<(String, Vec<f32>)>> {
    let policy_path = Path::new("pdfs/policy.txt");
    if !policy_path.exists() {
        return Err(anyhow!("File {:?} does not exist", policy_path));
    }
    
    let policy_content = fs::read_to_string(policy_path)?;
    let chunks = chunk_text(&policy_content, 25000);
    
    println!("Processing {} chunks with controlled parallelism", chunks.len());
    
    // Process chunks in parallel with limited concurrency
    let chunk_embeddings: Vec<_> = stream::iter(chunks.into_iter().enumerate())
        .map(|(i, chunk)| async move {
            println!("Processing chunk {} of total", i + 1);
            
            let embedding = get_single_embedding(&chunk, api_key).await?;
            Ok::<(String, Vec<f32>), anyhow::Error>((chunk, embedding))
        })
        .buffer_unordered(2) // Process max 2 chunks concurrently
        .collect::<Vec<_>>()
        .await;
    
    // Handle any errors
    let mut results = Vec::new();
    for result in chunk_embeddings {
        results.push(result?);
    }
    
    println!("Successfully processed {} chunks", results.len());
    Ok(results)
}

