// AI/gemini.rs
use std::{env, fs, path::Path};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::io::Write;
use chrono::Utc; 

pub async fn call_gemini_api_with_txts(questions: &[String]) -> Result<serde_json::Value>
{
    use serde_json::json;
    use std::io::Write;
    use chrono::Utc; // Add chrono = "0.4" under [dependencies] if you haven't

    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| anyhow!("GEMINI_KEY not found in env"))?;

    // Path to the single policy.txt file
    let policy_path = Path::new("pdfs/policy.txt");
    if !policy_path.exists() {
        return Err(anyhow!("File {:?} does not exist", policy_path));
    }
    let policy_content = fs::read_to_string(policy_path)?;

    let client = Client::new();
    let mut answers = Vec::new();

    for (i, question) in questions.iter().enumerate() {
        // Build prompt
        let prompt = format!(
            "{}\n\nReferring to the context in this .txt file respond to this question:\n{}",
            policy_content,
            question
        );

        // ---- LOGGING HERE ----
        let logs_dir = Path::new("logs");
        if !logs_dir.exists() {
            fs::create_dir_all(logs_dir)?;
        }
        let logs_path = logs_dir.join("prompt_sent_logs.txt");
        let log_entry = format!(
            "-----\nTime: {}\nQuestion #{}:\n{}\n\n",
            Utc::now().to_rfc3339(),
            i + 1,
            prompt
        );
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&logs_path)?;
        log_file.write_all(log_entry.as_bytes())?;
        // ---- END LOGGING ----

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
        println!("Raw Gemini response for question {} (status: {}):\n{}", i + 1, status, raw_text);

        if !status.is_success() {
            answers.push(format!("Error for question: {} - Gemini API request failed: {} - {}", question, status, raw_text));
            continue;
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&raw_text)
            .map_err(|e| anyhow!("Error deserializing Gemini response: {}\nRaw response: {}", e, raw_text))?;

        let first_answer = gemini_response
            .candidates
            .get(0)
            .and_then(|c| c.content.parts.get(0))
            .map(|part| part.text.clone())
            .unwrap_or_else(|| "<no response>".to_string());

        answers.push(first_answer);
    }

    let json_response = json!({ "answers": answers });
    println!("{}", serde_json::to_string_pretty(&json_response).unwrap());

    Ok(json_response)
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


