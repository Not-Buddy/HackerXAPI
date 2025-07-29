// AI/gemini.rs
use std::{env, fs, path::Path};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::io::Write;
use chrono::Utc;
use std::time::Instant;
use regex::Regex;
use serde_json;
// I've added these:
use serde_json::json;
//use crate::pdf::StdError;
use crate::ai::embed::embed_pdf_chunks;
use crate::ai::embed::find_relevant_chunks;

fn parse_gemini_response_to_answers(text: &str) -> Vec<String> {
    // Regex to match triple backticks with optional 'json' and capture inner content
    let re = Regex::new(r"(?s)```(?:json)?\n(.*?)```").unwrap();

    // If the text matches the fenced JSON block, extract the inside content, else use as is
    let json_str = if let Some(caps) = re.captures(text) {
        caps.get(1).map_or(text, |m| m.as_str())
    } else {
        text
    };

    // Parse the extracted string as a JSON array of strings
    match serde_json::from_str::<Vec<String>>(json_str) {
        Ok(answers) => answers,
        Err(err) => {
            eprintln!("Warning: failed to parse JSON array answers: {}", err);
            // On error, fallback to returning the entire string as a single-element vector
            vec![text.to_string()]
        }
    }
}


pub async fn call_gemini_api_with_txts(
    pdf_text: &str,
    questions: &[String],
) -> Result<Vec<String>> {
    let start_time = Instant::now();

    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // 1. Embed all PDF chunks
    let max_paragraphs = 5;
    let top_n = 3;
    let chunk_embeddings = embed_pdf_chunks(pdf_text, &api_key, max_paragraphs).await?;
    println!("Embedded {} chunks from PDF text", chunk_embeddings.len());

    let client = Client::new();
    let mut answers = Vec::new();

    for question in questions {
        // 2. Find relevant chunks for this question
        let relevant_chunks = find_relevant_chunks(question, &chunk_embeddings, &api_key, top_n).await?;
        let context = relevant_chunks.iter().map(|c| c.chunk.as_str()).collect::<Vec<&str>>().join("\n\n");
        let prompt = format!(
            "{}\n\nQuestion: {}\n\nPlease answer strictly as a JSON string.",
            context, question
        );

        // Logging
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
            .append(true)
            .open(&logs_path)?;
        log_file.write_all(log_entry.as_bytes())?;

        // Prepare Gemini API request
        let contents = vec![
            ContentsPart {
                parts: vec![TextPart { text: prompt.clone() }],
            }
        ];
        let body = GeminiRequest { contents };

        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent")
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &api_key)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let raw_text = response.text().await?;

        // Timing/logging
        let duration = start_time.elapsed();
        println!("Raw Gemini response (status: {}):\n{}", status, raw_text);
        println!("Time taken for Gemini API call and response: {:.2?}", duration);

        if !status.is_success() {
            return Err(anyhow!("Gemini API request failed: {} - {}", status, raw_text));
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&raw_text)
            .map_err(|e| anyhow!("Error deserializing Gemini response: {}\nRaw response: {}", e, raw_text))?;

        let first_answer = gemini_response
            .candidates
            .get(0)
            .and_then(|c| c.content.parts.get(0))
            .map(|part| part.text.clone())
            .unwrap_or_else(|| "<no response>".to_string());

        let mut parsed_answers = parse_gemini_response_to_answers(&first_answer);
        answers.push(parsed_answers.pop().unwrap_or_default());
    }

    Ok(answers)
}

pub async fn embed_text_google(chunk: &str, api_key: &str) -> Result<Vec<f32>, anyhow::Error> {
    let client = Client::new();
    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent";
    let body = serde_json::json!({
        "model": "models/gemini-embedding-001",
        "content": {
            "parts": [
                { "text": chunk }
            ]
        }
    });

    let resp = client
        .post(url)
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    println!("{:?}", resp); 


    let embedding = resp["embedding"]["values"]
        .as_array()
        .ok_or(anyhow!("No embedding in response"))? 
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    parts: Vec<TextPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<ContentsPart>,
}

#[derive(Serialize)]
struct ContentsPart {
    parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize)]
struct TextPart {
    text: String,
}


