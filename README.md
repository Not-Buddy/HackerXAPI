# HackerXAPI - Built for HackRx 🚀

This is a production-ready system built in Rust that combines high-performance asynchronous processing, AI/ML integration with the Gemini API, multi-format document handling, and security practices.

## 🏗️ System Architecture Overview

Our API implements a multi-layered architecture to tackle the problem statement and all test cases.

---

## Architecture
```text
+===================================================+
|           main.rs (Interactive CLI)               |
+---------------------------------------------------+
|            server.rs (API Gateway)                |
+---------------------------------------------------+
|        final_challenge.rs (Contest Logic)         |
+---------------------------------------------------+
|      ai/embed.rs (Vector Database Layer)          |
|      ai/gemini.rs (LLM Intelligence Layer)        |
+---------------------------------------------------+
|      pdf.rs + ocr.rs (Processing Pipeline)        |
+---------------------------------------------------+
|          MySQL (Persistent Vector Store)          |
+===================================================+
```
```
## HackerXAPI Architecture:

├── main.rs (Interactive CLI) <br />
├── server.rs (API Gateway) <br />
├── final_challenge.rs (Contest Logic) <br />
├── AI Layer: <br />
│   ├── embed.rs (Vector Database Layer) <br />
│   └── gemini.rs (LLM Intelligence Layer) <br />
├── Processing Layer: <br />
│   ├── pdf.rs (Document Processing) <br />
│   └── ocr.rs (OCR Pipeline) <br />
└── MySQL (Persistent Vector Store) <br />
```

## ✨ Features

* **Intelligent Document Processing**: Handles a wide array of file types (`PDF`, `DOCX`, `XLSX`, `PPTX`, `JPEG`, `PNG`, `TXT`) with a robust fallback chain.
* **High-Performance AI**: Leverages the Gemini API with optimized chunking, parallel processing, and smart context filtering for fast, relevant responses.
* **Enterprise-Grade Security**: Features multi-layer security, including extensive prompt injection sanitization, and parameterized SQL queries.
* **Scalable Architecture**: Built with a stateless design, `tokio` for async operations, and CPU-aware parallelization for horizontal scaling.
* **Interactive Management**: Includes a menu-driven CLI for easy server management, status monitoring, and graceful shutdowns.

---

## 🏛️ Architecture Overview

The system is designed as a series of specialized layers, from the user-facing API and CLI down to the persistent database storage.
```mermaid
graph TD
    subgraph "User Interfaces"
        A["main.rs<br/>(Interactive CLI)"]
        B["server.rs<br/>(RESTful API Gateway)"]
    end

    subgraph "Core Logic & Processing"
        C["final_challenge.rs<br/>(Contest Logic)"]
        D["pdf.rs & ocr.rs<br/>(Document Processing Pipeline)"]
    end

    subgraph "Intelligence & Data Layer"
        E["ai/gemini.rs<br/>(LLM Integration w/ Sanitization)"]
        F["ai/embed.rs<br/>(Vector Embedding & Caching)"]
        G[("MySQL<br/>(Persistent Vector Store)")]
    end

    A --> C
    B --> C
    C --> D
    C --> E
    D --> F
    E -- "Uses Embeddings" --> F
    F -- "Stores/Retrieves Vectors" --> G
```

-----

# 🔧 Core Components

## 🧠 `ai` - AI & Embedding Layer

This layer handles all interactions with the AI model and vector embeddings, featuring performance optimizations and smart context filtering.

### Performance Optimizations

* **Chunking Strategy**: Text is split into `33,000` character chunks, which is optimal for the Gemini API.
* **Parallel Processing**: Handles up to `50` concurrent requests using `futures::stream` for high throughput.
* **Database Caching**: Caches embedding vectors in MySQL to avoid redundant and costly API calls.
* **Batch Operations**: Uses functions like `batch_store_pdf_embeddings` for efficient bulk database insertions.

### Smart Context Filtering

* **Top-K Retrieval**: Fetches the top `10` most relevant document chunks for any given query.
* **Similarity Threshold**: Enforces a minimum relevance score of `0.5` (cosine similarity) to ensure context quality.
* **Combined Query Embedding**: Creates a single, more effective embedding when multiple user questions are asked at once.

### Enterprise-Level Security (`gemini.rs`)

* **Prompt Injection Defense**: Proactively sanitizes all user input against a list of over 22 known prompt injection patterns to protect the LLM.

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

### Performance Optimizations

* **Chunking Strategy**: Text is split into `33,000` character chunks, which is optimal for the Gemini API.
* **Parallel Processing**: Handles up to `50` concurrent requests using `futures::stream`.
* **Database Caching**: Caches embedding vectors in MySQL using the native `JSON` data type.
* **Batch Operations**: Uses functions like `batch_store_pdf_embeddings` for high-performance bulk database insertions.

### Smart Context Filtering

* **Top-K Retrieval**: Fetches the `10` most relevant document chunks for any given query.
* **Similarity Threshold**: Enforces a minimum relevance score of `0.5` (cosine similarity) to ensure context quality.
* **Combined Query Embedding**: Creates a single, more effective embedding when multiple user questions are asked at once.

---

## `gemini.rs` - LLM Integration Layer

This component showcases enterprise-level security and reliability in its integration with the Gemini model.

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

* **Structured Output**: Enforces a JSON schema for consistent and predictable LLM responses.
* **Cache Busting**: Uses UUIDs to ensure request uniqueness where needed.
* **Response Validation**: Implements multi-layer JSON parsing.
* **Prompt Engineering**: Constructs context-aware prompts for more accurate results.

---

## 📄 Document Processing Pipeline

The system will support the following files for text extraction:
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

* **CPU-Aware Parallelization**: Uses `num_cpus::get()` to spawn an optimal number of threads for processing.
* **Memory-Safe Concurrency**: Leverages `Arc<String>` for safe, shared ownership of data across parallel tasks.
* **Chunk-based PDF Processing**: Intelligently splits large PDFs into chunks to be processed in parallel across CPU cores.
* **Tool Fallback Chain**: Implements a resilient processing strategy, trying `pdftk`, then `qpdf`, and finally falling back to estimation if needed.

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
### Optical Character Recognition

The system also uses OCR to parse text from images or `pptx` files

**Multi-Tool Pipeline:**
* **Primary**: `ImageMagick` direct conversion.
* **Fallback**: A `LibreOffice` → PDF → Images chain.
* **OCR Engine**: Uses `ocrs-cli` for the final text extraction.
* **Format Chain**: A dedicated PPTX → Images → OCR → Text chain.

**Quality Optimization:**
* **DPI Settings**: Balances quality vs. speed with a `150 DPI` setting.
* **Background Processing**: Enforces a white background and alpha removal for better accuracy.
* **Slide Preservation**: Maintains original slide order and numbering throughout the process.

---

## 🌐 Server Architecture & API Design

The server implements intelligent request routing and security.

**Security Middleware:**
```rust
let auth = headers.get("authorization")
    .and_then(|value| value.to_str().ok());
if auth.is_none() || !auth.unwrap().starts_with("Bearer ") {
    return Err(StatusCode::UNAUTHORIZED);
}
```

* **URL-to-Filename Generation**: Intelligently detects file types from URLs.
* **Special Endpoint Handling**: Dedicated logic for handling endpoints in documents.
* **File Existence Checking**: Avoids redundant downloads by checking for existing vectors in the database first.

**Advanced Features:**
* **Final Challenge Detection**: Special handling for contest-specific files.
* **Error Response Standardization**: Returns errors in a consistent JSON format.
* **Performance Monitoring**: Includes request timing and logging for observability.

---


This module provides a user-friendly, menu-driven interface for managing the server.

**Menu-Driven Architecture:**
* **Graceful Shutdown**: Handles `Ctrl+C` for proper cleanup before exiting.
* **Server Management**: Allows starting and stopping the server with status monitoring.
* **Error Recovery**: Robustly handles invalid user input without crashing.

---

## 🚀 Advanced Technical Patterns

### Async Programming Mastery

**Tokio Runtime Utilization:**
```rust
tokio::task::spawn_blocking(move || extract_file_text_sync(&file_path)).await?
```

**Concurrency Patterns:**
* **Stream Processing**: Uses `buffer_unordered(PARALLEL_REQS)` for high-throughput, parallel stream processing.
* **Future Composition**: Employs `tokio::select!` for gracefully handling multiple asynchronous operations, such as a task and a shutdown signal.
* **Blocking Task Spawning**: Correctly offloads CPU-bound work to a dedicated thread pool to avoid blocking the async runtime.

### Database Architecture

**Connection Pool Management:**
```rust
static DB_POOL: Lazy<Pool> = Lazy::new(|| {
    let opts = Opts::from_url(&database_url).expect("Invalid database URL");
    Pool::new(opts).expect("Failed to create database pool")
});
```
**Performance Optimizations:**
* **Batch Insertions**: Commits multiple embeddings in a single transaction for efficiency.
* **Index Strategy**: Uses dedicated indexes like `idx_pdf_filename` and `idx_chunk_index` for fast lookups.
* **JSON Storage**: Uses MySQL's native `JSON` data type for optimal embedding storage and retrieval.

### Memory Management & Safety
**Rust Best Practices:**
* **RAII Pattern**: Guarantees automatic cleanup of temporary files and other resources when they go out of scope.
* **`Arc<T>`**: Employs `Arc` for safe, shared ownership of data in parallel processing contexts.
* **`Result<T, E>`**: Uses comprehensive error propagation throughout the application for robust failure handling.
* **`Option<T>`**: Ensures null safety across the entire codebase.

---

## 🛡️ Security & Reliability Features

### Multi-Layer Security
* **Input Sanitization**: Defends against prompt injection attacks.
* **File Type Validation**: Uses a whitelist-based approach for processing file types.
* **Payload Limits**: Enforces request limits (e.g., 35KB on embeddings) for staying within API limits. This can be removed for a big performance gain.
* **SQL Injection Prevention**: Exclusively uses parameterized queries to protect the database.

### Error Handling Strategy
**Graceful Degradation:**
* **Tool Fallbacks**: Implements a chain of multiple OCR and file conversion tools to maximize success rates.
* **File Recovery**: Reuses existing files to recover from partial processing failures.
* **API Resilience**: Provides proper HTTP status codes and clear, standardized error messages.

---

## 📊 Performance Characteristics

### Scalability Metrics
* **Concurrent Embeddings**: Processes up to **50 parallel requests** This is of course, limited by the API rate limits. Removing it will improve performance greatly.
* **Chunk Processing**: Utilizes CPU-core optimized parallel processing for large PDFs.
* **Database & Caching**: Leverages connection pooling and file caching to maximize token use and be as efficient as possible.

### Quality Thresholds
* **Relevance Filter**: A `0.5` cosine similarity score is the minimum for context retrieval.
* **Context Window**: Uses the **top 10** chunks to provide optimal context to the LLM. A higher context window increases accuracy even further.
* **OCR Quality**: Balances speed and accuracy with a `150 DPI` setting.

---

## 🎯 Production-Ready Features

* **Stateless Design**: Each request is independent, making it easy to scale and multithread.
* **Observability**: Includes comprehensive logging and timing measurements for every case.
* **Configuration**: All configuration is managed via environment variables for easy deployment.
* **Resource Management**: Temporary files are cleaned up automatically via the RAII pattern.
* **API Standards**: Adheres to RESTful design principles with proper HTTP semantics.

---

## What is Unique here then?

* **Built in Rust**: We chose rust to make the API as fast as possible.
* **Persistent Vector Store**: The MySQL Database is perfect for company level usage of the system, where a document is queried constantly by both employees and clients.
* **Handles all Documents**: A chain of tools with fallbacks ensures that the system handles as many document types as possible.
* **Context-Aware Embedding**: Combines multiple questions into a single embedding for token efficiency. 
* **Prompt Injection Protecton**: Features prompt injection protection.
-----
## Get It Running

### 1\. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
```

### 2\. Install System Dependencies

This command is for Debian/Ubuntu-based systems.

```bash
sudo apt-get update
sudo apt-get install pdftk-java qpdf poppler-utils libglib2.0-dev libcairo2-dev libpoppler-glib-dev bc libreoffice imagemagick
```

### 3\. Install Rust Tools

```bash
cargo install miniserve
cargo install ocrs-cli --locked
```

### 4\. Configure Environment

Create a `.env` file from the example:

```bash
cp .envexample .env
```

### 5\. Setup Database

Create a MySQL database and run the following schema:

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

Then, populate your `.env` file with the database connection string and your Gemini API key:

```ini
MYSQL_CONNECTION=mysql://username:password@localhost:3306/your_database
GEMINI_KEY=your_gemini_api_key
```

### 6\. Run the Application

```bash
cargo run
```

### 7\. Testing

The repository includes three scripts with  various payloads to test the API with different document types:

```bash
./test.sh
./sim.sh
./simr4.sh
```

-----

## 🔧 Requirements

  * Rust (latest stable)
  * MySQL database
  * Google Gemini API key
  * System packages for document processing (listed in step 2)
  * OCR tools for image text extraction (listed in step 3)