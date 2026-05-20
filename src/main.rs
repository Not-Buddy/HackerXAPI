mod config;
mod error;
mod server;
mod ai;
mod ocr;
mod extraction;
mod vectordb;
mod pipeline;
mod storage;

use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::ai::gemini::GeminiProvider;
use crate::config::Config;
use crate::extraction::docx::DocxExtractor;
use crate::extraction::image::{self, ImageExtractor};
use crate::extraction::libreoffice::LibreOfficeExtractor;
use crate::extraction::pdf::PdfExtractor;
use crate::extraction::pptx::PptxExtractor;
use crate::extraction::text::PlainTextExtractor;
use crate::extraction::xlsx::XlsxExtractor;
use crate::ocr::paddle::{self, PaddleOcrEngine};
use crate::pipeline::Pipeline;
use crate::storage::StorageBackend;
use crate::vectordb::qdrant::QdrantStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tracing_subscriber::fmt::init();

    let config = Config::from_env().map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

    println!("Initializing Gemini provider...");
    let (gemini, vector_size) = GeminiProvider::from_config(
        config.gemini_key.clone(),
        config.embed_model_override.clone(),
        config.llm_model_override.clone(),
        config.auto_discover_models,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Gemini init failed: {}", e))?;

    println!("Initializing Qdrant vector store (dim={})...", vector_size);
    let vector_store = QdrantStore::new(
        &config.qdrant_url,
        &config.qdrant_api_key,
        &config.qdrant_collection,
        vector_size,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to initialize Qdrant: {}", e))?;

    let model_dir = std::env::var("OCR_MODEL_DIR")
        .unwrap_or_else(|_| "models".to_string());
    paddle::download_models_if_needed(std::path::Path::new(&model_dir)).await?;
    let ocr_engine = PaddleOcrEngine::new()
        .map_err(|e| anyhow::anyhow!("OCR engine init failed: {}", e))?;
    image::set_ocr_engine(Box::new(ocr_engine));

    let extractors: Vec<Box<dyn extraction::TextExtractor>> = vec![
        Box::new(PdfExtractor),
        Box::new(DocxExtractor),
        Box::new(XlsxExtractor),
        Box::new(PptxExtractor),
        Box::new(PlainTextExtractor),
        Box::new(ImageExtractor),
        Box::new(LibreOfficeExtractor),
    ];

    let storage = ask_storage(&config).await?;

    let pipeline = Arc::new(Pipeline::new(
        config.clone(),
        storage,
        Box::new(vector_store),
        Box::new(gemini.clone()),
        Box::new(gemini),
        extractors,
    ));

    server::handlers::set_pipeline(pipeline);

    let port = config.server_port;
    println!(
        "Starting server on http://0.0.0.0:{} ... Press Ctrl+C to stop.",
        port
    );

    let app = server::create_router();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    println!("Server running... Press Ctrl+C to stop.");

    let server_instance = axum::serve(listener, app);

    tokio::select! {
        res = server_instance => {
            if let Err(err) = res {
                eprintln!("Server error: {}", err);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl+C received, shutting down...");
        }
    }

    Ok(())
}

fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

async fn ask_storage(config: &Config) -> anyhow::Result<Box<dyn StorageBackend>> {
    // If STORAGE_BACKEND is explicitly set, use it without prompting
    if let Ok(backend) = std::env::var("STORAGE_BACKEND") {
        if !backend.is_empty() && backend != "prompt" {
            return match backend.as_str() {
                "r2" => {
                    let account_id = config.r2_account_id.as_deref()
                        .ok_or_else(|| anyhow::anyhow!("R2_ACCOUNT_ID not set"))?;
                    let access_key = config.r2_access_key.as_deref()
                        .ok_or_else(|| anyhow::anyhow!("R2_ACCESS_KEY_ID not set"))?;
                    let secret_key = config.r2_secret_key.as_deref()
                        .ok_or_else(|| anyhow::anyhow!("R2_SECRET_ACCESS_KEY not set"))?;
                    let bucket = config.r2_bucket.as_deref()
                        .ok_or_else(|| anyhow::anyhow!("R2_BUCKET not set"))?;
                    println!("Initializing R2 storage (bucket={})...", bucket);
                    Ok(Box::new(storage::r2::R2Storage::new(account_id, access_key, secret_key, bucket).await))
                }
                _ => {
                    println!("Initializing local storage (dir={})...", config.storage_local_dir);
                    Ok(Box::new(storage::local::LocalStorage::new(&config.storage_local_dir)))
                }
            };
        }
    }

    // Interactive prompt
    println!("\nStorage backend:");
    println!("  1. local  — files on disk ({})", config.storage_local_dir);
    println!("  2. R2     — Cloudflare R2 (S3-compatible)");
    let choice = read_line("\nSelect [1-2] (default 1): ");

    match choice.as_str() {
        "2" => {
            let account_id = read_line("  R2 Account ID: ");
            let access_key = read_line("  R2 Access Key ID: ");
            let secret_key = read_line("  R2 Secret Access Key: ");
            let bucket = read_line("  R2 Bucket name [ragx-files]: ");
            let bucket = if bucket.is_empty() { "ragx-files".to_string() } else { bucket };

            if account_id.is_empty() || access_key.is_empty() || secret_key.is_empty() {
                anyhow::bail!("R2 credentials required");
            }

            let backend = storage::r2::R2Storage::new(&account_id, &access_key, &secret_key, &bucket).await;
            println!("R2 storage initialized (bucket={})\n", bucket);
            Ok(Box::new(backend))
        }
        _ => {
            println!("Local storage initialized (dir={})\n", config.storage_local_dir);
            Ok(Box::new(storage::local::LocalStorage::new(&config.storage_local_dir)))
        }
    }
}
