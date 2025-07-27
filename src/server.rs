use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::pdf::delete_file;
use crate::pdf::extract_pdf_text;
use crate::pdf::download_pdf;
use crate::ai::gemini::call_gemini_api_with_txts;
use std::time::Instant;

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
    Json(body): Json<QuestionRequest>,) -> Result<Json<AnswersResponse>, Response> {
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

    let tmp_path = "/tmp/policy.pdf";
    let permpath = "pdfs/policy.pdf";
    download_pdf(&body.documents, tmp_path)
        .await
        .map_err(|e| {
            println!("Failed to download PDF: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PDF download error: {}", e),
            )
                .into_response()
        })?;

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


    println!("PDF downloaded successfully to {}", tmp_path);

    let pdf_text = extract_pdf_text(tmp_path).await.map_err(|e| {
        println!("Failed to extract PDF text: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("PDF extraction error: {}", e),
        )
            .into_response()
    })?;

    // Clean up temp file
    delete_file(tmp_path).ok();

    println!("Processing questions and preparing answers...");

    let answers_response = answer_questions(&pdf_text, &body.questions)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Answering questions error: {}", e),
            )
                .into_response()
        })?;

    println!("Request processed successfully. Sending response.");
    println!("Request processed successfully in {:?}. Sending response.", start_time.elapsed());

    Ok(Json(answers_response))
}
