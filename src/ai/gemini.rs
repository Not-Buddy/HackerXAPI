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


pub async fn call_gemini_api_with_txts(questions: &[String], pdf_filename: &str) -> Result<Vec<String>> {
    // Start measuring time
    let start_time = Instant::now();

    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // Path to the filtered context file (dynamic based on PDF filename)
    let context_filename = format!("pdfs/{}_contextfiltered.txt", pdf_filename);
    let context_path = Path::new(&context_filename);

    if !context_path.exists() {
    return Err(anyhow!("Context filtered file {:?} does not exist", context_path));
    }

    let policy_content = fs::read_to_string(context_path)?;

    let client = Client::new();


    // This is the structre that Gemini will send the output in
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

    let generation_config = GenerationConfig {
        responseMimeType: "application/json".to_string(),
        responseSchema: response_schema,
    };

    // Construct the single prompt:
    let questions_joined = questions.join(", ");
    let prompt = format!(
        "{}\n\nPlease answer the following questions one by one with this form
        Respond with the answers to the questions one by one in the specified structure.
        Ensure answers are atleast 12 words,
        Decision (e.g., approved or rejected), Amount (if applicable), and Justification, including mapping of each decision to the specific clause(s) it was based on.
        Do not include the questions or any other text or formatting. Do not include code blocks, markdown, or any other formatting\
        The questions are separated by commas:\n{}",
        policy_content.trim(),
        questions_joined
    );

    // Log the prompt as before
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

    let contents = vec![
        ContentsPart {
            parts: vec![TextPart { text: prompt }],
        }
    ];
    let body = GeminiRequest { contents, generationConfig: Some(generation_config) };

    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent")
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", &api_key)
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let raw_text = response.text().await?;
    
    // Stop measuring time
    let duration = start_time.elapsed();
    // println!("Raw Gemini response (status: {}):\n{}", status, raw_text);
    println!("Time taken for Gemini API call and response: {:.2?}", duration);

    if !status.is_success() {
        return Err(anyhow!("Gemini API request failed: {} - {}", status, raw_text));
    }

    use serde_json::Value;
    // Try to parse the raw response as JSON
    let json: Value = serde_json::from_str(&raw_text)
        .map_err(|e| anyhow!("Error deserializing Gemini response: {}\nRaw response: {}", e, raw_text))?;

    // Extract the inner JSON string
    let inner_json_str = json.get("candidates")
    .and_then(|c| c.get(0))
    .and_then(|c| c.get("content"))
    .and_then(|content| content.get("parts"))
    .and_then(|parts| parts.get(0))
    .and_then(|part| part.get("text"))
    .and_then(|t| t.as_str());
    
    let answers = if let Some(inner_json_str) = inner_json_str {
    // Parse the string as JSON
        let inner_json: Value = serde_json::from_str(inner_json_str)
            .map_err(|e| anyhow!("Error parsing inner Gemini JSON: {}\nInner: {}", e, inner_json_str))?;
        inner_json.get("answers")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec![])
    } else {
        vec![]
    };

    println!("{:#?}", answers);

    Ok(answers)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    generationConfig: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct ContentsPart {
    parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize)]
struct TextPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    responseMimeType: String,
    responseSchema: serde_json::Value,
}
