use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::pipeline::{ApiCounters, Pipeline};
use crate::server::auth;

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

pub async fn query(
    headers: HeaderMap,
    Json(body): Json<QuestionRequest>,
) -> Result<Json<AnswersResponse>, Response> {
    let start_time = Instant::now();
    let mut total = ApiCounters::default();

    println!(
        "[req   ]  START  url={}  questions={}",
        body.documents,
        body.questions.len()
    );

    if auth::extract_bearer_token(&headers).is_none() {
        println!("[req   ]  REJECT  Missing Authorization token");
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

    let (stored_file, extracted_text) = pipeline
        .process_document(&body.documents)
        .await
        .map_err(|e| {
            println!("[req   ]  FAIL  Process: {}", e);
            let error_response = AnswersResponse {
                answers: vec!["Sorry we do not support the file format that you uploaded"
                    .to_string()],
            };
            (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
        })?;

    let doc_id = stored_file.doc_id();
    println!(
        "[req   ]  doc_id={}  name={}  size={}  chars={}",
        doc_id,
        stored_file.original_name,
        stored_file.size_bytes,
        extracted_text.len()
    );

    let (chunk_embeddings, c) = pipeline
        .embed_and_store(&doc_id, &extracted_text)
        .await
        .map_err(|e| {
            println!("[req   ]  FAIL  Embed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Embedding error: {}", e),
            )
                .into_response()
        })?;
    total.embed_calls += c.embed_calls;
    total.retries += c.retries;

    let combined_questions = body.questions.join(" ");
    let (relevant_chunks, c) = pipeline
        .search_similar(&combined_questions)
        .await
        .map_err(|e| {
            println!("[req   ]  FAIL  Search: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Search error: {}", e),
            )
                .into_response()
        })?;
    total.embed_calls += c.embed_calls;

    let context = if relevant_chunks.is_empty() {
        extracted_text.clone()
    } else {
        relevant_chunks.join("\n\n---\n\n")
    };

    let (answers, c) = pipeline
        .generate_answer(&context, &body.questions)
        .await
        .map_err(|e| {
            println!("[req   ]  FAIL  LLM: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Answering questions error: {}", e),
            )
                .into_response()
        })?;
    total.llm_calls += c.llm_calls;
    total.retries += c.retries;

    let elapsed = start_time.elapsed();
    println!(
        "[req   ]  DONE  doc={}  chars={}  chunks={}  embed={}  llm={}  retries={}  {:?}",
        doc_id,
        extracted_text.len(),
        chunk_embeddings.len(),
        total.embed_calls,
        total.llm_calls,
        total.retries,
        elapsed,
    );

    Ok(Json(AnswersResponse { answers }))
}
