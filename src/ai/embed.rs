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
    chunk_embeddings: &[(String, Vec<f32>)], // Add this parameter
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
        .take(2)
        .filter(|(similarity, _)| *similarity > 0.6) // Lower threshold since we're combining questions
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
    
    // Write the new content to contextfilered.txt
    let context_path = Path::new("pdfs/contextfiltered.txt");
    fs::write(context_path, new_content)?;

    println!("Successfully wrote relevant context to pdfs/contextfiltered.txt");
    Ok(())
}
