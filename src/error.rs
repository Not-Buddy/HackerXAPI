use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Vector store error: {0}")]
    VectorStore(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("OCR error: {0}")]
    Ocr(String),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Any(#[from] anyhow::Error),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
