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
    let re = Regex::new(r"(?s)```(?:json)?\n(.*?)```").unwrap();

    let extracted = re
    .captures(text)
    .and_then(|caps| caps.get(1))
    .map_or(text, |m| m.as_str());

// Try to parse the captured string as a JSON array of strings
let outer_array: Vec<String> = match serde_json::from_str(extracted) {
    Ok(arr) => arr,
    Err(err) => {
        eprintln!("Warning: failed to parse outer JSON array: {}", err);
        return vec![text.to_string()];
    }
};

// Parse each inner string as its own JSON array and extract the final sentence
outer_array
    .into_iter()
    .map(|s| {
        // Each s should be a string like "[\"Approved\", \"X\", \"Y\"]"
        match serde_json::from_str::<Vec<String>>(&s) {
            Ok(inner) => inner.get(2).cloned().unwrap_or_else(|| s),
            Err(_) => s, // fallback if it's not a valid array
        }
    })
    .collect()
}



pub async fn call_gemini_api_with_txts(questions: &[String]) -> Result<Vec<String>> {
    // Start measuring time
    let start_time = Instant::now();

    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // Path to the filtered context file
    let context_path = Path::new("pdfs/contextfiltered.txt");
    if !context_path.exists() {
        return Err(anyhow!("File {:?} does not exist", context_path));
    } 
let policy_content = fs::read_to_string(context_path)?;

let client = Client::new();

// Construct the single prompt:
let questions_joined = questions.join(", ");
let prompt = format!(
    "{}\n\nPlease answer the following questions one by one with this form
    Decision (e.g., approved or rejected), Amount (if applicable), and Justification, including mapping of each decision to the specific clause(s) it was based on.
    Respond strictly with a JSON array of answer strings only. 
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
    println!("Raw Gemini response (status: {}):\n{}", status, raw_text);
    println!("Time taken for Gemini API call and response: {:.2?}", duration);

    if !status.is_success() {
        return Err(anyhow!("Gemini API request failed: {} - {}", status, raw_text));
    }

    use serde_json::Value;

let gemini_response: GeminiResponse = serde_json::from_str(&raw_text)
.map_err(|e| anyhow!("Error deserializing Gemini response: {}\nRaw response: {}", e, raw_text))?;

let first_answer = gemini_response
.candidates
.get(0)
.and_then(|c| c.content.parts.get(0))
.map(|part| part.text.clone())
.unwrap_or_else(|| "<no response>".to_string());

// Try to extract the actual JSON content inside triple-backtick json ...
// This helps avoid parsing the outer markdown wrapper.
let re = Regex::new(r"(?s)(?:json)?\n(.*?)").unwrap();
let clean_json_text = re
.captures(&first_answer)
.and_then(|caps| caps.get(1))
.map(|m| m.as_str().trim())
.unwrap_or(&first_answer);

// Parse it as a Vec<String> — each element is a JSON string, we want to de-escape those.
let intermediate_array: Vec<String> = match serde_json::from_str(clean_json_text) {
Ok(val) => val,
Err(e) => {
eprintln!("Failed to parse JSON array from response: {}\nRaw: {}", e, clean_json_text);
vec![first_answer.clone()]
}
};

// Now remove the escaped quotes from each string (unescape once more)
let answers: Vec<String> = intermediate_array
.into_iter()
.map(|s| {
match serde_json::from_str::<Value>(&s) {
Ok(Value::Array(inner)) => {
inner.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(" | ")
}
_ => s,
}
})
.collect();

println!("{:#?}", answers);
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


