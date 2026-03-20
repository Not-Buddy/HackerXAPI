
***

# HackerXAPI - Built for HackRx

## System Architecture Overview

The API implements a multi-layered architecture designed to systematically address complex problem statements and satisfy comprehensive test cases.

---

## Architecture Diagram
```text
+===================================================+
|            main.rs (Interactive CLI)              |
+---------------------------------------------------+
|            server.rs (API Gateway)                |
+---------------------------------------------------+
|        final_challenge.rs (Contest Logic)         |
+---------------------------------------------------+
|        ai/embed.rs (Vector Database Layer)        |
|        ai/gemini.rs (LLM Intelligence Layer)      |
+---------------------------------------------------+
|        pdf.rs + ocr.rs (Processing Pipeline)      |
+---------------------------------------------------+
|          MySQL (Persistent Vector Store)          |
+===================================================+
```
```text
## HackerXAPI File Structure:

├── main.rs (Interactive CLI) 
├── server.rs (API Gateway) 
├── final_challenge.rs (Contest Logic) 
├── AI Layer: 
│   ├── embed.rs (Vector Database Layer) 
│   └── gemini.rs (LLM Intelligence Layer) 
├── Processing Layer: 
│   ├── pdf.rs (Document Processing) 
│   └── ocr.rs (OCR Pipeline) 
└── MySQL (Persistent Vector Store) 
```

## Features

* **Intelligent Document Processing**: Handles a wide array of file types (`PDF`, `DOCX`, `XLSX`, `PPTX`, `JPEG`, `PNG`, `TXT`) leveraging a robust tool fallback chain.
* **High-Performance AI**: Utilizes the Gemini API with optimized chunking, parallel processing, and smart context filtering for rapid, relevant responses.
* **Enterprise-Grade Security**: Features multi-layer security, including extensive prompt injection sanitization and parameterized SQL queries.
* **Scalable Architecture**: Built with a stateless design, `tokio` for asynchronous operations, and CPU-aware parallelization for horizontal scaling.
* **Interactive Management**: Includes a menu-driven CLI for streamlined server management, status monitoring, and graceful shutdowns.

---

## Architecture Flowchart

The system is designed as a series of specialized layers, operating from the user-facing API and CLI down to persistent database storage.

```mermaid
flowchart TD
    A[CLI Menu] -->|Start Server| B[Axum Server :8000]
    A -->|Exit| EXIT([Exit])
    
    B -->|POST /api/v1/hackrx/run| C{Auth Valid?}
    C -->|No| E401([401 Unauthorized])
    C -->|Yes| D[Download & Extract Text]
    
    D --> E{File Type?}
    E -->|PDF/DOCX/XLSX| F[Parse Document]
    E -->|Images/PPTX| G[OCR Processing]
    E -->|TXT| H[Direct Read]
    
    F --> I[Text Output]
    G --> I
    H --> I
    
    I --> J{Embeddings<br/>Cached?}
    
    J -->|No| L[Chunk Text &<br/>Generate Embeddings<br/>via Gemini API]
    J -->|Yes| K[Load from<br/>MySQL]
    
    L --> M[Store to MySQL]
    
    K --> N[Cosine Similarity Search]
    M --> N
    
    N --> O[Select Top 10<br/>Relevant Chunks]
    
    O --> P[Gemini 2.0 Flash<br/>Answer Generation]
    
    P --> Q[Parse Structured<br/>JSON Response]
    
    Q --> SUCCESS([200 OK<br/>JSON Response])
    
    D -.->|Uses| TOOLS[pdftk, ocrs<br/>ImageMagick<br/>LibreOffice]
    J -.->|Cache| DB[(MySQL<br/>Database)]
    P -.->|API| GEMINI[Gemini API]
```

---

# Core Components

## `ai` - AI & Embedding Layer

This layer manages all interactions with the AI model and vector embeddings, featuring performance optimizations and context filtering mechanisms.

### Performance Optimizations

* **Chunking Strategy**: Text is split into 33,000-character chunks, calibrated for optimal performance with the Gemini API.
* **Parallel Processing**: Capable of handling up to 50 concurrent requests using `futures::stream` for high throughput.
* **Database Caching**: Caches embedding vectors in MySQL using the native `JSON` data type to eliminate redundant API calls.
* **Batch Operations**: Employs functions such as `batch_store_pdf_embeddings` for highly efficient bulk database insertions.

### Smart Context Filtering

* **Top-K Retrieval**: Retrieves the 10 most relevant document chunks for any submitted query.
* **Similarity Threshold**: Enforces a minimum cosine similarity relevance score of 0.5 to ensure the quality of provided context.
* **Combined Query Embedding**: Generates a consolidated, unified embedding when users submit multiple simultaneous questions.

### Advanced Vector Operations
```rust
// Cosine similarity with proper error handling
fn cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
    let magnitude1: f32 = vec1.iter().map(|v| v * v).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|v| v * v).sum::<f32>().sqrt();
    // ... proper zero-magnitude handling
}
```

---

## `gemini.rs` - LLM Integration Layer

This component establishes enterprise-level security and reliability protocols for integration with the Gemini model.

### Security Features
```rust
fn sanitize_policy(content: &str) -> String {
    let dangerous_patterns = [
        r"(?i)ignore\s+previous\s+instructions",
        r"(?i)disregard\s+the\s+above",
        r"(?i)pretend\s+to\s+be",
        // ... 22 different injection patterns
    ];
    // Regex-based sanitization
}
```

### Advanced API Patterns

* **Structured Output**: Enforces a JSON schema for consistent, predictable LLM responses.
* **Cache Busting**: Utilizes UUIDs to guarantee request uniqueness where necessary.
* **Response Validation**: Implements multi-layer JSON parsing for strict type safety.
* **Prompt Engineering**: Constructs dynamic, context-aware prompts to maximize output accuracy.

---

## Document Processing Pipeline

The system supports the following files for text extraction:

**File Type Support Matrix:**
```rust
match ext.as_str() {
    "docx" => convert_docx_to_pdf(file_path)?,
    "xlsx" => convert_xlsx_to_pdf(file_path)?,
    "pdf" => extract_pdf_text_sync(file_path),
    "jpeg" | "png" => crate::ocr::extract_text_with_ocrs(file_path),
    "pptx" => extract_text_from_pptx(file_path),
    "txt" => extract_token_from_text(file_path),
}
```

### Performance Engineering

* **CPU-Aware Parallelization**: Utilizes `num_cpus::get()` to spawn the optimal number of processing threads based on host hardware.
* **Memory-Safe Concurrency**: Leverages `Arc<String>` for secure, shared data ownership across parallel task executions.
* **Chunk-based PDF Processing**: Intelligently partitions large PDFs into subsets for concurrent processing across CPU cores.
* **Tool Fallback Chain**: Implements a highly resilient processing strategy, prioritizing `pdftk`, failing over to `qpdf`, and relying on estimation techniques as a final resort.

### PDF Processing
```rust
let page_ranges: Vec<(usize, usize)> = (0..num_cores)
    .map(|i| {
        let start = i * pages_per_chunk + 1;
        let end = ((i + 1) * pages_per_chunk).min(total_pages);
        (start, end)
    })
    .collect();
```

### Optical Character Recognition (OCR)

The system deploys an OCR pipeline to parse text from image assets and `.pptx` presentations.

**Multi-Tool Pipeline:**
* **Primary Route**: Direct conversion via `ImageMagick`.
* **Fallback Route**: A `LibreOffice` → PDF → Images sequence.
* **OCR Engine**: Employs `ocrs-cli` for terminal text extraction.
* **Format Chain**: A dedicated PPTX → Images → OCR → Text conversion path.

**Quality Optimization:**
* **DPI Settings**: Calibrated to 150 DPI to balance processing speed with extraction accuracy.
* **Background Processing**: Enforces white backgrounds and alpha channel removal for superior OCR legibility.
* **Slide Preservation**: Strictly maintains original slide order and numbering throughout processing phases.

---

## Server Architecture & API Design

The server implements intelligent request routing combined with edge-level security.

**Security Middleware:**
```rust
let auth = headers.get("authorization")
    .and_then(|value| value.to_str().ok());
if auth.is_none() || !auth.unwrap().starts_with("Bearer ") {
    return Err(StatusCode::UNAUTHORIZED);
}
```

* **URL-to-Filename Generation**: Algorithmically detects and assigns file extensions from raw URLs.
* **Special Endpoint Handling**: Contains dedicated business logic for parsing endpoints directly from documents.
* **File Existence Checking**: Preemptively checks the database for existing vectors to eliminate redundant bandwidth and API usage.

**Advanced Features:**
* **Final Challenge Detection**: Customized logic pathways for contest-specific files.
* **Error Response Standardization**: Returns all errors in a strictly standardized JSON format for predictable client handling.
* **Performance Monitoring**: Integrates request timing and granular logging for full system observability.

---

## Interactive Management Console

Provides a user-friendly, menu-driven interface for direct server administration.

* **Graceful Shutdown**: Intercepts `Ctrl+C` commands to ensure proper memory cleanup and transaction completion before exit.
* **Server Management**: Facilitates straightforward starting and stopping of the server, alongside live status monitoring.
* **Error Recovery**: Robustly captures and handles invalid standard input without initiating process panics.

---

## Advanced Technical Patterns

### Async Programming Mastery

**Tokio Runtime Utilization:**
```rust
tokio::task::spawn_blocking(move || extract_file_text_sync(&file_path)).await?
```

**Concurrency Patterns:**
* **Stream Processing**: Uses `buffer_unordered(PARALLEL_REQS)` for high-throughput, parallelized stream execution.
* **Future Composition**: Employs `tokio::select!` to orchestrate multiple asynchronous operations gracefully, such as coordinating active tasks with shutdown signals.
* **Blocking Task Spawning**: Systematically offloads CPU-bound operations to a dedicated thread pool, protecting the async runtime from blocking.

### Database Architecture

**Connection Pool Management:**
```rust
static DB_POOL: Lazy<Pool> = Lazy::new(|| {
    let opts = Opts::from_url(&database_url).expect("Invalid database URL");
    Pool::new(opts).expect("Failed to create database pool")
});
```
**Performance Optimizations:**
* **Batch Insertions**: Commits multiple embedding records within single transactions to minimize overhead.
* **Index Strategy**: Deploys targeted indexes such as `idx_pdf_filename` and `idx_chunk_index` to guarantee rapid data retrieval.
* **JSON Storage**: Native utilization of MySQL's `JSON` data type for streamlined embedding storage and extraction.

### Memory Management & Safety
**Rust Best Practices:**
* **RAII Pattern**: Guarantees deterministic, automatic cleanup of temporary files and system resources upon scope exit.
* **`Arc<T>`**: Employs Atomic Reference Counting (`Arc`) for thread-safe data access across parallel execution environments.
* **`Result<T, E>`**: Implements exhaustive error propagation throughout the stack for reliable failure handling.
* **`Option<T>`**: Ensures rigorous null safety and state verification across the entire codebase.

---

## Security & Reliability Features

### Multi-Layer Security
* **Input Sanitization**: Actively defends against sophisticated prompt injection attack vectors.
* **File Type Validation**: Enforces a strict whitelist-based approach for allowable processing formats.
* **Payload Limits**: Restricts request sizes (e.g., 35KB on embeddings) to comply with API constraints. These thresholds can be adjusted based on host infrastructure capacity to scale throughput.
* **SQL Injection Prevention**: Exclusively utilizes parameterized database queries to secure the data layer.

### Error Handling Strategy
**Graceful Degradation:**
* **Tool Fallbacks**: Implements a cascading chain of OCR and conversion tools to maximize processing success rates.
* **File Recovery**: Systematically reuses valid intermediate files to recover from partial pipeline failures.
* **API Resilience**: Guarantees standard HTTP status codes accompanied by clear, actionable error messaging.

---

## Performance Characteristics

### Scalability Metrics
* **Concurrent Embeddings**: Processes up to 50 parallel requests. Overall throughput is currently bound by API rate limits; elevating these limits will yield linear performance scaling.
* **Chunk Processing**: Fully utilizes CPU-core optimized parallelization for rapid processing of high-volume PDFs.
* **Database & Caching**: Leverages persistent connection pooling and aggressive file caching to maximize token efficiency and minimize latency.

### Quality Thresholds
* **Relevance Filter**: Mandates a 0.5 minimum cosine similarity score to qualify context for retrieval.
* **Context Window**: Aggregates the top 10 most relevant chunks to supply optimal context to the LLM. Expanding this window further increases granular accuracy.
* **OCR Quality**: Operates at 150 DPI to establish an optimal baseline between processing duration and text accuracy.

---

## Production-Ready Features

* **Stateless Design**: Ensures each request is entirely independent, facilitating seamless multithreading and horizontal scalability.
* **Observability**: Incorporates comprehensive logging pipelines and precise timing measurements for analytical review.
* **Configuration**: Centralizes all runtime configurations via environment variables to simplify deployment pipelines.
* **Resource Management**: Automates the purging of temporary files via strict adherence to the RAII pattern.
* **API Standards**: Strictly adheres to RESTful design principles and semantic HTTP operations.

---

## Key Differentiators

* **Built in Rust**: Engineered in Rust to guarantee optimal processing speeds, strict memory safety, and minimal system latency.
* **Persistent Vector Store**: Utilizes a MySQL backend, providing a robust architecture for enterprise-level document querying by broad user bases.
* **Comprehensive Document Handling**: A sophisticated chain of tools with automated fallbacks guarantees support for an exceptionally wide spectrum of document formats.
* **Context-Aware Embedding**: Consolidates multiple concurrent queries into unified embeddings to drastically improve API token efficiency. 
* **Prompt Injection Protection**: Integrates rigorous algorithmic sanitization protocols to defend the LLM against malicious inputs.

---

## Installation and Setup Guide

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Install System Dependencies

Execute the following on Debian/Ubuntu-based distributions to prepare the host environment:

```bash
sudo apt-get update
sudo apt-get install pdftk-java qpdf poppler-utils libglib2.0-dev libcairo2-dev libpoppler-glib-dev bc libreoffice imagemagick
```

### 3. Install Rust Tools

```bash
cargo install miniserve
cargo install ocrs-cli --locked
```

### 4. Configure Environment

Initialize the environment variable file from the provided template:

```bash
cp .envexample .env
```

### 5. Setup Database

Deploy a MySQL database instance and execute the following schema initialization:

```sql
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
```

Next, update your `.env` file with the appropriate database connection string and your Gemini API credentials:

```ini
MYSQL_CONNECTION=mysql://username:password@localhost:3306/your_database
GEMINI_KEY=your_gemini_api_key
```

### 6. Run the Application

```bash
cargo run
```

### 7. Testing

The repository includes three automated shell scripts designed to test the API endpoint against various payload types and document formats:

```bash
./test.sh
./sim.sh
./simr4.sh
```

---

## Requirements

* Rust (latest stable release)
* MySQL database instance
* Google Gemini API key
* Host system packages for document processing (detailed in Step 2)
* OCR CLI tools for image text extraction (detailed in Step 3)

***
