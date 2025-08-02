use std::fs;
use std::path::Path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

const CHUNK_SIZE: usize = 30000;
const PARALLEL_REQS: usize = 100;
const RELEVANT_CHUNKS: usize = 10;

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

/// Chunk text into pieces of exactly max_chars size (may cut words)
fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    text.chars()
        .collect::<Vec<char>>()
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect::<String>())
        .filter(|chunk| !chunk.trim().is_empty())
        .collect()
}


async fn get_single_embedding(text: &str, api_key: &str) -> Result<Vec<f32>> {
    let request_body = EmbedRequest {
        model: "models/gemini-embedding-001".to_string(),
        content: ContentPart {
            parts: vec![TextPart {
                text: text.to_string(),
            }],
        },
    };

    // Check payload size before sending
    let payload_json = serde_json::to_string(&request_body)?;
    let payload_size = payload_json.len();
    
    if payload_size > 35000 { // Leave some buffer
        return Err(anyhow!("Payload too large: {} bytes (limit ~36000)", payload_size));
    }
    
    println!("Sending payload of {} bytes", payload_size);

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


/// Alternative: Return all chunk embeddings instead of averaging
use futures::stream::{self, StreamExt};

pub async fn get_policy_chunk_embeddings(api_key: &str, pdf_filename: &str) -> Result<Vec<(String, Vec<f32>)>> {
    let txt_filename = format!("pdfs/{}.txt", pdf_filename);
    let policy_path = Path::new(&txt_filename);
    if !policy_path.exists() {
        return Err(anyhow!("File {:?} does not exist", policy_path));
    }
    
    let policy_content = fs::read_to_string(policy_path)?;
    let chunks = chunk_text(&policy_content, CHUNK_SIZE);
    
    println!("Processing {} chunks with controlled parallelism", chunks.len());
    
    // Process chunks in parallel with limited concurrency
    let chunk_embeddings: Vec<_> = stream::iter(chunks.into_iter().enumerate())
        .map(|(i, chunk)| async move {
            println!("Processing chunk {} of total", i + 1);
            
            let embedding = get_single_embedding(&chunk, api_key).await?;
            Ok::<(String, Vec<f32>), anyhow::Error>((chunk, embedding))
        })
        .buffer_unordered(PARALLEL_REQS)
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

/// Calculate cosine similarity between two vectors
fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    if vec1.len() != vec2.len() {
        return 0.0;
    }

    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
    let magnitude1: f32 = vec1.iter().map(|v| v * v).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|v| v * v).sum::<f32>().sqrt();

    if magnitude1 == 0.0 || magnitude2 == 0.0 {
        0.0
    } else {
        dot_product / (magnitude1 * magnitude2)
    }
}


pub async fn rewrite_policy_with_context(
    api_key: &str,
    questions: &[String],
    chunk_embeddings: &[(String, Vec<f32>)],
    pdf_filename: &str,

) -> Result<()> {
    // Combine all questions into a single text for embedding - this is already batched
    let combined_questions = questions.join(" ");
    println!("Getting combined embedding for all questions at once: {}", combined_questions);
    
    // Get a single embedding for all questions combined - this is one API call, not per question
    let questions_embedding = get_single_embedding(&combined_questions, api_key).await?;
    println!("Got questions embedding with {} dimensions", questions_embedding.len());
    
    // Use the passed chunk embeddings instead of computing them again
    println!("Using pre-computed chunk embeddings with {} chunks", chunk_embeddings.len());
    
    // Now find relevant chunks using the combined questions embedding
    let mut chunk_similarities = Vec::new();
    
    for (chunk_text, chunk_emb) in chunk_embeddings {
        let similarity = cosine_similarity(&questions_embedding, chunk_emb);
        chunk_similarities.push((similarity, chunk_text.clone()));
    }
    
    // Sort by similarity (highest first) and take top chunks
    chunk_similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let top_chunks: Vec<String> = chunk_similarities
        .into_iter()
        .take(RELEVANT_CHUNKS)
        .filter(|(similarity, _)| *similarity > 0.4) // Lower threshold since we're combining questions
        .map(|(_, text)| text)
        .collect();
    
    let mut new_content = String::new();

    
    // Add relevant context
    if !top_chunks.is_empty() {
        let context = top_chunks.join("\n\n---\n\n");
        new_content.push_str(&context);
        new_content.push_str("\n\n");
    } else {
        new_content.push_str("No highly relevant context found for these questions.\n\n");
    }
    
    let context_filename = format!("pdfs/{}_contextfiltered.txt", pdf_filename);
    let context_path = Path::new(&context_filename);
    fs::write(context_path, new_content)?;
    println!("Successfully wrote relevant context to {}", context_filename);
    Ok(())
}