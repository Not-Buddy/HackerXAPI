use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::pdf::extract_file_text;
use crate::pdf::download_file;
use crate::ai::gemini::{call_gemini_api_with_txts};
use crate::ai::embed::{get_policy_chunk_embeddings, rewrite_policy_with_context}; // Fixed import
use std::{env, time::Instant, fs};

#[derive(Deserialize)]
pub struct QuestionRequest {
    pub documents: String,
    pub questions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AnswersResponse {
    pub answers: Vec<String>,
}

pub async fn answer_questions(_pdf_text: &str, questions: &[String], pdf_filename: &str) -> Result<AnswersResponse, Box<dyn std::error::Error>> {
    let answers = call_gemini_api_with_txts(&questions, pdf_filename).await?;
    Ok(AnswersResponse { answers })
}


pub async fn hackrx_run(
    headers: HeaderMap,
    Json(body): Json<QuestionRequest>,
) -> Result<Json<AnswersResponse>, Response> {
    let start_time = Instant::now();
    println!("Received request with documents URL: {}", body.documents);

    // Authorization check
    let auth = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if auth.is_none() || !auth.unwrap().starts_with("Bearer ") {
        println!("Request rejected: Missing or invalid Authorization token");
        return Err((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization token",
        )
            .into_response());
    }

    println!("Authorization token accepted, starting PDF download...");


    println!("Authorization token accepted, processing document...");

    // Generate filename from URL
    let filename = generate_filename_from_url(&body.documents).map_err(|e| {
        println!("Failed to generate filename from URL: {}", e);
    
        // Create error response in the same format as successful responses
        let error_response = AnswersResponse {
            answers: vec!["Sorry we do not support the file format that you uploaded".to_string()]
        };
    
        (
        StatusCode::BAD_REQUEST,
        Json(error_response),
        )
        .into_response()
    })?;


    let permpath = format!("pdfs/{}", filename);
    println!("Target file path: {}", permpath);

    // Check if file already exists
    let file_exists = Path::new(&permpath).exists();
    
    if file_exists {
        println!("File already exists at {}, skipping download", permpath);
    } else {
        println!("File not found, downloading from: {}", body.documents);
        
        // Ensure pdfs directory exists
        if let Some(parent) = Path::new(&permpath).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                println!("Failed to create pdfs directory: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Directory creation error: {}", e),
                )
                .into_response()
            })?;
        }

        download_file(&body.documents, &permpath)
            .await
            .map_err(|e| {
                println!("Failed to download FILE: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("FILE download error: {}", e),
                )
                .into_response()
            })?;
        
        println!("FILE downloaded successfully to {}", permpath);
    }

    println!("FILE downloaded successfully to {}", permpath);

    // Extract PDF text - this creates pdfs/{permapath}.txt
    let _pdf_text = extract_file_text(&permpath).await.map_err(|e| {
        println!("Failed to extract PDF text: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("PDF extraction error: {}", e),
        )
        .into_response()
    })?;

    // Get API key and embedding AFTER text extraction
    dotenvy::dotenv().ok();
    let api_key = env::var("GEMINI_KEY").map_err(|_| {
        println!("GEMINI_KEY not found in environment variables");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "GEMINI_KEY environment variable not found",
        )
            .into_response()
    })?;

    let pdf_filename = std::path::Path::new(&permpath)
    .file_stem()
    .and_then(|name| name.to_str())
    .unwrap_or("document");

    let chunk_embeddings = get_policy_chunk_embeddings(&api_key, pdf_filename).await.map_err(|e| {
        println!("Failed to get policy chunk embeddings: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Embedding error: {}", e),
        )
            .into_response()
    })?;

    println!("Got chunk embeddings for {} chunks", chunk_embeddings.len());
    println!("Processing questions and preparing answers...");

    // Rewrite filename.txt with relevant context for questions
    rewrite_policy_with_context(&api_key, &body.questions, &chunk_embeddings, pdf_filename)
        .await
        .map_err(|e| {
            println!("Failed to rewrite policy with context: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Context rewriting error: {}", e),
            )
                .into_response()
        })?;

    println!("Policy file rewritten with question contexts");


    // Generate the contextfiltered filename based on the PDF filename
    let pdf_filename = std::path::Path::new(&permpath)
    .file_stem()
    .and_then(|name| name.to_str())
    .unwrap_or("document");
    let contextfiltered_filename = format!("pdfs/{}_contextfiltered.txt", pdf_filename);

    // Now call your answer function with the rewritten context
    let updated_pdf_text = fs::read_to_string(&contextfiltered_filename).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read updated policy: {}", e),
        )
            .into_response()
    })?;

    let answers_response = answer_questions(&updated_pdf_text, &body.questions, pdf_filename)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Answering questions error: {}", e),
            )
                .into_response()
        })?;

    println!("Request processed successfully in {:?}. Sending response.", start_time.elapsed());

    Ok(Json(answers_response))
}

use std::path::Path;
use url::Url;

// Add this helper function to generate filename from URL
fn generate_filename_from_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed_url = Url::parse(url)?;
    
    // Get the last segment of the path
    let filename = parsed_url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("document")
        .to_string();
    
    // Remove query parameters and fragments if they got included
    let clean_filename = filename.split('?').next().unwrap_or(&filename).to_string();
    
    // Check for unsupported file types first
    let unsupported_exts = ["zip", "bin"];
    let has_unsupported_ext = unsupported_exts.iter().any(|ext| {
        clean_filename.to_lowercase().ends_with(&format!(".{}", ext))
    });
    
    if has_unsupported_ext {
        return Err("We don't support this file type. ZIP and BIN files are not supported.".into());
    }
    
    // Define allowed extensions
    let allowed_exts = ["jpeg", "pptx", "docx", "xlsx", "png", "pdf"];
    
    // Check if filename ends with any allowed extension
    let has_allowed_ext = allowed_exts.iter().any(|ext| clean_filename.to_lowercase().ends_with(ext));
    
    // Generate final filename based on presence of allowed extension
    let final_filename = if has_allowed_ext {
        clean_filename
    } else if clean_filename.is_empty() || clean_filename == "document" {
        // Generate a hash-based filename for unclear URLs
        format!("document_{}.pdf", hash_url(url))
    } else {
        // Append .pdf as default if no allowed extension present
        format!("{}.pdf", clean_filename)
    };
    
    // Sanitize filename for filesystem safety
    let sanitized = final_filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
        .collect();
    
    Ok(sanitized)
}



// Simple hash function for generating unique filenames
fn hash_url(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
