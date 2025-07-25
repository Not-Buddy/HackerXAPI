use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::pdf::delete_file;
use crate::pdf::extract_pdf_text;
use crate::pdf::download_pdf;


#[derive(Deserialize)]
pub struct QuestionRequest {
    pub documents: String,
    pub questions: Vec<String>,
}

#[derive(Serialize)]
pub struct AnswersResponse {
    pub answers: Vec<String>,
}

pub async fn answer_questions(_pdf_text: &str, questions: &[String]) -> Vec<String> {
    questions.iter().map(|q| format!("(Dummy answer): {}", q)).collect()
}


pub async fn hackrx_run(
    headers: HeaderMap,
    Json(body): Json<QuestionRequest>,
) -> Result<Json<AnswersResponse>, Response> {
    // Authorization check
    let auth = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    if auth.is_none() || !auth.unwrap().starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization token",
        )
            .into_response());
    }

    let tmp_path = "/tmp/policy.pdf";

    download_pdf(&body.documents, tmp_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PDF download error: {}", e),
            )
                .into_response()
        })?;

    let pdf_text = extract_pdf_text(tmp_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("PDF extraction error: {}", e),
        )
            .into_response()
    })?;

    // Clean up temp file
    delete_file(tmp_path).ok();

    let answers = answer_questions(&pdf_text, &body.questions).await;
    Ok(Json(AnswersResponse { answers }))
}
