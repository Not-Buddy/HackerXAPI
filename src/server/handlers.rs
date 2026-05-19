use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::pipeline::Pipeline;

static PIPELINE: OnceLock<Arc<Pipeline>> = OnceLock::new();

pub fn set_pipeline(pipeline: Arc<Pipeline>) {
    let _ = PIPELINE.set(pipeline);
}

fn get_pipeline() -> Option<&'static Arc<Pipeline>> {
    PIPELINE.get()
}

#[derive(Deserialize)]
pub struct QuestionRequest {
    pub documents: String,
    pub questions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AnswersResponse {
    pub answers: Vec<String>,
}

pub async fn hackrx_run(
    headers: HeaderMap,
    Json(body): Json<QuestionRequest>,
) -> Result<Json<AnswersResponse>, Response> {
    let start_time = Instant::now();
    println!("Received request with documents URL: {}", body.documents);

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

    let pipeline = get_pipeline().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pipeline not initialized",
        )
            .into_response()
    })?;

    let permpath = crate::pipeline::url::generate_filename_from_url(&body.documents)
        .await
        .map(|filename| format!("pdfs/{}", filename))
        .map_err(|e| {
            println!("Failed to generate filename from URL: {}", e);
            let error_response = AnswersResponse {
                answers: vec!["Sorry we do not support the file format that you uploaded".to_string()],
            };
            (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
        })?;

    let doc_id = std::path::Path::new(&permpath)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    println!("Target file path: {}, doc_id: {}", permpath, doc_id);

    let extracted_text = pipeline.process_document(&body.documents).await.map_err(|e| {
        println!("Failed to process document: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Document processing error: {}", e),
        )
            .into_response()
    })?;

    println!("Extracted {} characters of text", extracted_text.len());

    pipeline.embed_and_store(doc_id, &extracted_text).await.map_err(|e| {
        println!("Failed to embed document: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Embedding error: {}", e),
        )
            .into_response()
    })?;

    println!("Processing questions and preparing answers...");

    let combined_questions = body.questions.join(" ");
    let relevant_chunks = pipeline
        .search_similar(&combined_questions, 10, 0.3)
        .await
        .map_err(|e| {
            println!("Failed to search context: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Search error: {}", e),
            )
                .into_response()
        })?;

    let context = if relevant_chunks.is_empty() {
        extracted_text
    } else {
        relevant_chunks.join("\n\n---\n\n")
    };

    let contextfiltered_filename = format!("pdfs/{}_contextfiltered.txt", doc_id);
    let _ = std::fs::write(&contextfiltered_filename, &context);

    println!("Policy file rewritten with question contexts");

    let answers = pipeline
        .generate_answer(&context, &body.questions)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Answering questions error: {}", e),
            )
                .into_response()
        })?;

    println!(
        "Request processed successfully in {:?}. Sending response.",
        start_time.elapsed()
    );

    Ok(Json(AnswersResponse { answers }))
}
