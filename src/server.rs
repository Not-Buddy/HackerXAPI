use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::pdf::extract_pdf_text;
use crate::pdf::download_pdf;
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

pub async fn answer_questions(_pdf_text: &str, questions: &[String]) -> Result<AnswersResponse, anyhow::Error> {
    let answers = call_gemini_api_with_txts(questions).await?;
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

    let permpath = "pdfs/policy.pdf";

    download_pdf(&body.documents, permpath)
        .await
        .map_err(|e| {
            println!("Failed to download PDF: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PDF download error: {}", e),
            )
                .into_response()
        })?;

    println!("PDF downloaded successfully to {}", permpath);

    // Extract PDF text - this creates pdfs/policy.txt
    let _pdf_text = extract_pdf_text(permpath).await.map_err(|e| {
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

    let chunk_embeddings = get_policy_chunk_embeddings(&api_key).await.map_err(|e| {
        println!("Failed to get policy chunk embeddings: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Embedding error: {}", e),
        )
            .into_response()
    })?;

    println!("Got chunk embeddings for {} chunks", chunk_embeddings.len());
    println!("Processing questions and preparing answers...");

    // Rewrite policy.txt with relevant context for questions
    rewrite_policy_with_context(&api_key, &body.questions, &chunk_embeddings)
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

    // Now call your answer function with the rewritten content
    let updated_pdf_text = fs::read_to_string("pdfs/contextfiltered.txt").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read updated policy: {}", e),
        )
            .into_response()
    })?;

    let answers_response = answer_questions(&updated_pdf_text, &body.questions)
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
