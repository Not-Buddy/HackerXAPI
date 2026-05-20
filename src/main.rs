mod config;
mod error;
mod server;
mod ai;
mod ocr;
mod extraction;
mod vectordb;
mod pipeline;

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
use crate::vectordb::qdrant::QdrantStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let pipeline = Arc::new(Pipeline::new(
        config.clone(),
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

    let server = axum::serve(listener, app);

    tokio::select! {
        res = server => {
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
