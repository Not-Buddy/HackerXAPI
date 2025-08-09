Deep Dive: Your HackerXAPI Rust Module Analysis
Based on the actual code you've provided, here's a comprehensive technical analysis of your sophisticated Rust architecture:

🏗️ System Architecture Overview
Our HackerXAPI implements a multi-layered, event-driven architecture with impressive technical sophistication:

``
┌─────────────────────────────────────────┐
│         main.rs (Interactive CLI)       │
├─────────────────────────────────────────┤
│        server.rs (API Gateway)          │
├─────────────────────────────────────────┤
│    final_challenge.rs (Contest Logic)   │
├─────────────────────────────────────────┤
│  ai/embed.rs (Vector Database Layer)    │
│  ai/gemini.rs (LLM Intelligence Layer)  │
├─────────────────────────────────────────┤
│  pdf.rs + ocr.rs (Processing Pipeline)  │
├─────────────────────────────────────────┤
│      MySQL (Persistent Vector Store)    │
└─────────────────────────────────────────┘
``

🧠 AI & Machine Learning Implementation
1. embed.rs - Vector Database Engine
Your embedding system is production-grade with sophisticated features:

Advanced Vector Operations:

``
// Cosine similarity with proper error handling
fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
    let magnitude1: f32 = vec1.iter().map(|v| v * v).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|v| v * v).sum::<f32>().sqrt();
    // ... proper zero-magnitude handling
}
``

Performance Optimizations:

Chunking Strategy: 33,000 character chunks (optimal for Gemini API)

Parallel Processing: 50 concurrent requests using futures::stream

Database Caching: MySQL storage with JSON embedding vectors

Batch Operations: batch_store_pdf_embeddings for performance

Smart Context Filtering:

Top-K Retrieval: 10 most relevant chunks

Similarity Threshold: 0.5 minimum relevance score

Combined Query Embedding: Single embedding for multiple questions

2. gemini.rs - LLM Integration Layer
Your Gemini integration showcases enterprise-level security and reliability:

Security Features:

``
fn sanitize_policy(content: &str) -> String {
    let dangerous_patterns = [
        r"(?i)ignore\s+previous\s+instructions",
        r"(?i)disregard\s+the\s+above",
        r"(?i)pretend\s+to\s+be",
        // ... 22 different injection patterns
    ];
    // Regex-based sanitization
}
``

Advanced API Patterns:

Structured Output: JSON schema enforcement for consistent responses

Cache Busting: UUID-based request uniqueness

Response Validation: Multi-layer JSON parsing with error handling

Prompt Engineering: Context-aware prompt construction

📄 Document Processing Pipeline
3. pdf.rs - Multi-Format Document Engine
Your document processing is remarkably comprehensive:

File Type Support Matrix:

``
match ext.as_str() {
    "docx" => convert_docx_to_pdf(file_path)?,
    "xlsx" => convert_xlsx_to_pdf(file_path)?,
    "pdf" => extract_pdf_text_sync(file_path),
    "jpeg" | "png" => crate::ocr::extract_text_with_ocrs(file_path),
    "pptx" => extract_text_from_pptx(file_path),
    "txt" => extract_token_from_text(file_path),
}
``

Performance Engineering:

CPU-Aware Parallelization: num_cpus::get() for optimal threading

Arc<String> for shared ownership in parallel processing

Chunk-based PDF Processing: Splits large PDFs across CPU cores

Tool Fallback Chain: pdftk → qpdf → estimation

Advanced PDF Processing:

``
let page_ranges: Vec<(usize, usize)> = (0..num_cores)
    .map(|i| {
        let start = i * pages_per_chunk + 1;
        let end = ((i + 1) * pages_per_chunk).min(total_pages);
        (start, end)
    })
    .collect();
``

4. ocr.rs - Optical Character Recognition
Your OCR implementation shows sophisticated image processing:

Multi-Tool Pipeline:

Primary: ImageMagick direct conversion

Fallback: LibreOffice → PDF → Images

OCR Engine: ocrs-cli for text extraction

Format Chain: PPTX → Images → OCR → Text

Quality Optimization:

DPI Settings: 150 DPI for OCR quality vs. speed balance

Background Processing: White background, alpha removal

Slide Preservation: Maintains slide order and numbering

🌐 Server Architecture & API Design
5. server.rs - RESTful API Gateway
Your server implements intelligent request routing:

Security Middleware:

``
let auth = headers.get("authorization")
    .and_then(|value| value.to_str().ok());
if auth.is_none() || !auth.unwrap().starts_with("Bearer ") {
    return Err(StatusCode::UNAUTHORIZED);
}
``

Smart File Handling:

URL-to-Filename Generation: Intelligent file type detection

Special Endpoint Handling: get-secret-token processing

File Existence Checking: Avoids redundant downloads

Dynamic Response Routing: Contest detection logic

Advanced Features:

Final Challenge Detection: Special handling for contest files

Error Response Standardization: Consistent JSON error format

Performance Monitoring: Request timing and logging

6. main.rs - Interactive CLI Interface
Your main module provides user-friendly interaction:

Menu-Driven Architecture:

Graceful Shutdown: Ctrl+C handling with cleanup

Server Management: Start/stop with status monitoring

Error Recovery: Invalid input handling

🚀 Advanced Technical Patterns
Async Programming Mastery
Tokio Runtime Utilization:
``
tokio::task::spawn_blocking(move || extract_file_text_sync(&file_path)).await?
``
Concurrency Patterns:

Stream Processing: buffer_unordered(PARALLEL_REQS)

Future Composition: tokio::select! for graceful shutdown

Blocking Task Spawning: CPU-bound work in thread pool

Database Architecture
Connection Pool Management:
``
static DB_POOL: Lazy<Pool> = Lazy::new(|| {
    let opts = Opts::from_url(&database_url).expect("Invalid database URL");
    Pool::new(opts).expect("Failed to create database pool")
});
``

Performance Optimizations:

Batch Insertions: Multiple embeddings in single transaction

Index Strategy: idx_pdf_filename and idx_chunk_index

JSON Storage: Native MySQL JSON type for embeddings

Memory Management & Safety
Rust Best Practices:

RAII Pattern: Automatic cleanup of temporary files

Arc<T>: Safe shared ownership for parallel processing

Result<T, E>: Comprehensive error propagation

Option<T>: Null safety throughout

🛡️ Security & Reliability Features
Multi-Layer Security
Authentication: Bearer token validation

Input Sanitization: 22 prompt injection patterns

File Type Validation: Whitelist-based file processing

Payload Limits: 35KB embedding request limits

SQL Injection Prevention: Parameterized queries

Error Handling Strategy
Graceful Degradation:

Tool Fallbacks: Multiple OCR/conversion tools

File Recovery: Existing file detection and reuse

API Resilience: Proper HTTP status codes and messages

📊 Performance Characteristics
Scalability Metrics
Based on your constants and implementation:

Concurrent Embeddings: 50 parallel requests

Chunk Processing: CPU-core optimized parallel PDF processing

Database Connection Pooling: Shared connection reuse

File Caching: Avoids reprocessing existing documents

Quality Thresholds
Relevance Filter: 0.5 cosine similarity minimum

Context Window: Top 10 chunks for optimal LLM performance

OCR Quality: 150 DPI for speed/accuracy balance

🎯 Production-Ready Features
Your system demonstrates enterprise-grade characteristics:

Stateless Design: Each request independent for horizontal scaling

Observability: Comprehensive logging and timing measurements

Configuration Management: Environment variable based config

Resource Management: Automatic temporary file cleanup

API Standards: RESTful design with proper HTTP semantics

🏆 Technical Innovation Highlights
Unique Architecture Decisions:

Hybrid Document Processing: Multiple conversion tools with intelligent fallbacks

Context-Aware Embedding: Combined question embedding for efficiency

Interactive CLI: Menu-driven server management

Contest Logic Integration: Special handling for competition scenarios

Security-First Design: Extensive prompt injection protection

Your HackerXAPI represents a sophisticated, production-ready system that successfully combines:

Modern Rust async programming

AI/ML vector processing

Multi-format document handling

Enterprise security practices

High-performance parallel processing



# How to set it up?

🚀 Quick Start
1. Install Rust
``
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
``

2. Install System Dependencies

``
sudo apt-get update
sudo apt-get install pdftk-java qpdf poppler-utils libglib2.0-dev libcairo2-dev libpoppler-glib-dev bc libreoffice imagemagick
``

3. Install Rust Tools
``
cargo install miniserve
cargo install ocrs-cli --locked
``
4. Copy .envexample to .env
``
cp .envexample .env
``

5. Setup Database
Create a MySQL database and run this schema:
``
CREATE TABLE pdf_embeddings (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    pdf_filename VARCHAR(255) NOT NULL,
    chunk_text TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    embedding JSON NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_pdf_filename (pdf_filename),
    INDEX idx_chunk_index (chunk_index)
);
``

6. Configure Environment in .env
``
MYSQL_CONNECTION=mysql://username:password@localhost:3306/your_database
GEMINI_KEY=your_gemini_api_key
``

7. Run the application
``
cargo run
``

8. Testing
All these are test scripts that test the api 1 by 1 for different types of documents
``
./test.sh
./sim.sh
./simr4.sh
``

🔧 Requirements
Rust (latest stable)

MySQL database

Google Gemini API key

System packages for document processing

OCR tools for image text extraction