// AI/gemini.rs
use std::{env, fs, path::Path};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio;
use anyhow::{Result, anyhow};
use regex::Regex;

fn parse_gemini_response_to_answers(text: &str) -> Vec<String> {
    let re = Regex::new(r"\n\d+\.\s").unwrap();

    // Find the start of the numbered list
    let start = re.find(text).map(|m| m.start()).unwrap_or(0);
    let numbered_part = &text[start..];

    let parts: Vec<String> = re
        .split(numbered_part)
        .filter(|part| !part.trim().is_empty())
        .map(|s| {
            let mut cleaned = s.trim().to_string();
            cleaned = cleaned.replace("**", "");
            if let Some(colon_pos) = cleaned.find(':') {
                cleaned = cleaned[colon_pos + 1..].trim_start().to_string();
            }
            cleaned
        })
        .collect();

    parts
}

pub fn get_gemini_key() -> String 
{

    dotenvy::dotenv().ok();

    env::var("GEMINI_KEY").expect("GEMINI_KEY not set in .env file")
}


pub async fn call_gemini_api_with_txts(questions: &[String]) -> Result<String> {
    // Load env variables including GEMINI_KEY
    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // Path where .txt files are stored
    let txt_dir = Path::new("pdfs/text");
    if !txt_dir.exists() {
        return Err(anyhow!("Text directory {:?} does not exist", txt_dir));
    }

    // Read and concatenate all .txt files
    let mut combined_texts = Vec::new();
    for entry in fs::read_dir(txt_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            let content = fs::read_to_string(&path)?;
            combined_texts.push(content);
        }
    }

    if combined_texts.is_empty() {
        return Err(anyhow!("No .txt files found in {:?}", txt_dir));
    }

    // Combine PDF text and all questions into one prompt string
    let mut prompt = combined_texts.join("\n\n");
    prompt.push_str("\n\nQuestions:\n");
    for (i, question) in questions.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, question));
    }

    // Create a single ContentsPart for the entire prompt
    let contents = vec![
        ContentsPart {
            parts: vec![TextPart {
                text: prompt,
            }],
        }
    ];

    // Build request body
    let body = GeminiRequest { contents };

    // Call Gemini API
    let client = Client::new();
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent")
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", api_key)
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let raw_text = response.text().await?;

    // Print the full raw response to the terminal
    println!("Raw Gemini response (status: {}):\n{}", status, raw_text);

    // If not success, return error with body
    if !status.is_success() {
        return Err(anyhow!("Gemini API request failed: {} - {}", status, raw_text));
    }use regex::Regex;



    let gemini_response: GeminiResponse = serde_json::from_str(&raw_text)
        .map_err(|e| anyhow!("Error deserializing Gemini response: {}\nRaw response: {}", e, raw_text))?;

    // Extract the text from the new response structure
    let first_answer = gemini_response
        .candidates
        .get(0)
        .and_then(|c| c.content.parts.get(0))
        .map(|part| part.text.clone())
        .unwrap_or_else(|| "<no response>".to_string());

    // Parse into a vector of answers
    let answers = parse_gemini_response_to_answers(&first_answer);

    let json_response = serde_json::json!({
        "answers": answers,
    });

    println!("{}", serde_json::to_string_pretty(&json_response).unwrap());

    // Optionally, return the JSON string instead of the raw text
    Ok(json_response.to_string())
}


#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
    finishReason: Option<String>,
    avgLogprobs: Option<f64>,
}

#[derive(Deserialize)]
struct Content {
    parts: Vec<TextPart>,
    role: Option<String>,
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


