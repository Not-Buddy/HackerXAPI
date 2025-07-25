use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{fs, net::SocketAddr};
use tokio::{fs as async_fs, net::TcpListener};

#[derive(Deserialize)]
struct QuestionRequest {
    documents: String,
    questions: Vec<String>,
}

#[derive(Serialize)]
struct AnswersResponse {
    answers: Vec<String>,
}

// Fix: dyn Error must also be Send + Sync
type StdError = dyn std::error::Error + Send + Sync + 'static;

async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}

fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    let text = pdf_extract::extract_text(file_path)?;
    Ok(text)
}

async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path))
        .await?
}

async fn answer_questions(_pdf_text: &str, questions: &[String]) -> Vec<String> {
    questions
        .iter()
        .map(|q| format!("(Dummy answer): {}", q))
        .collect()
}

async fn hackrx_run(
    headers: HeaderMap,
    Json(body): Json<QuestionRequest>,
) -> Result<Json<AnswersResponse>, Response> {
    // -- Basic Bearer check (not robust, example only) --
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
    let _ = fs::remove_file(tmp_path);

    let answers = answer_questions(&pdf_text, &body.questions).await;
    Ok(Json(AnswersResponse { answers }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/hackrx/run", post(hackrx_run));
    let addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();
    println!("Listening on http://{}", addr);

    // Use TcpListener + serve (axum 0.7+ way)
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
