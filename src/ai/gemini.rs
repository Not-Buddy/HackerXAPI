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

fn parse_gemini_response_to_answers(text: &str) -> Vec<String> {
    // Regex to match triple backticks with optional 'json' and capture inner content
    let re = Regex::new(r"(?s)^``````$").unwrap();

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


pub async fn call_gemini_api_with_txts(questions: &[String]) -> Result<Vec<String>> {
    // Start measuring time
    let start_time = Instant::now();

    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // Path to the single policy.txt file
    let policy_path = Path::new("pdfs/policy.txt");
    if !policy_path.exists() {
        return Err(anyhow!("File {:?} does not exist", policy_path));
    }
    let policy_content = fs::read_to_string(policy_path)?;

    let client = Client::new();

    // Construct the single prompt:
    let questions_joined = questions.join(", ");
    let prompt = format!(
    "{}\n\nPlease answer the following questions one by one. Respond strictly with a JSON array of answer strings only. \
    Do not include the questions or any other text or formatting. Do not include code blocks, markdown, or any other formatting—only a plain JSON array. \
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
    
    // Stop measuring time
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

    // Parse answers
    let answers = parse_gemini_response_to_answers(&first_answer);

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
}

#[derive(Serialize)]
struct ContentsPart {
    parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize)]
struct TextPart {
    text: String,
}


