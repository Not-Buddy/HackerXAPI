
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


sample input/output
Input :- 
POST /hackrx/run
Content-Type: application/json
Accept: application/json
Authorization: Bearer febc0daceda23ebce03d324301d34ad3768494f0b52a39ffb4adaf083d8f9c5c

{
    "documents": "https://hackrx.blob.core.windows.net/assets/policy.pdf?sv=2023-01-03&st=2025-07-04T09%3A11%3A24Z&se=2027-07-05T09%3A11%3A00Z&sr=b&sp=r&sig=N4a9OU0w0QXO6AOIBiu4bpl7AXvEZogeT%2FjUHNO7HzQ%3D",
    "questions": [
        "What is the grace period for premium payment under the National Parivar Mediclaim Plus Policy?",
        "What is the waiting period for pre-existing diseases (PED) to be covered?",
        "Does this policy cover maternity expenses, and what are the conditions?",
        "What is the waiting period for cataract surgery?",
        "Are the medical expenses for an organ donor covered under this policy?",
        "What is the No Claim Discount (NCD) offered in this policy?",
        "Is there a benefit for preventive health check-ups?",
        "How does the policy define a 'Hospital'?",
        "What is the extent of coverage for AYUSH treatments?",
        "Are there any sub-limits on room rent and ICU charges for Plan A?"
    ]
}

Output :- 
{"answers":["The grace period for payment of the premium is thirty days.\n",
"Yes, this policy covers maternity expenses. (See section 3.1.14)\n",
"According to the policy, \"Accident means a sudden, unforeseen and involuntary event caused by external, visible and violent means.\"\n",
"The waiting period for Pre-Existing Diseases is 36 months of continuous coverage after the date of inception of the first policy.\n",
"The daily room charge limits for Plan A are up to 1% of the Sum Insured (SI) or the actual cost, whichever is lower.  The ICU charges are up to 2% of the SI or the actual cost, whichever is lower.\n",
"The policy covers a post-hospitalisation period of 60 days.\n",
"Yes, the medical expenses of an organ donor are covered, as stated in Section 3.1.7 \"Organ Transplant\".\n",
"No, Air Ambulance is not covered under Plan A.\n","The co-payment percentage is 22.5%.\n",
"The Free Look Period for a new policy is thirty days from the date of receipt of the policy document.\n",
"If any claim is found to be fraudulent, or if any false statement or declaration is made or used in support of a claim, or if any fraudulent means or devices are used by the insured person or anyone acting on their behalf to obtain any benefit under the policy, all benefits under the policy and the premium paid will be forfeited. Any amount already paid against fraudulent claims must be repaid by the recipient(s) / policyholder(s), who are jointly and severally liable for such repayment to the insurer.\n",
"No, treatment for alcoholism or drug abuse is explicitly excluded under exclusion 4.12.\n",
"The coverage limit for cataract surgery under Plan A is up to 15% of the Sum Insured or INR 60,000, whichever is lower.\n",
"A newborn baby is automatically covered from birth under the Sum Insured available to the mother during the corresponding Policy Period, for up to 3 months of age.\n",
"The maximum claim for infertility treatment under Plan B is INR 1,00,000.\n",
"Yes, the policy covers HIV/AIDS treatment. Section 3.1.17 specifically addresses this: \"The Company shall indemnify the Hospital or the Insured the Medical Expenses (including Pre and Post Hospitalisation Expenses) related to following stages of HIV infection...\"\n",
"The Sum Insured is reinstated due to a road traffic accident under the following conditions:\n\n1.  The sum insured has been exhausted because of claims arising out of any injury due to a road traffic accident during a policy year.\n2.  The Insured and/or Insured Person(s) has to subsequently incur any expenses on hospitalization due to any other disease/ injury.\n3.  The Company shall reinstate the sum insured as mentioned in the schedule.\n4.  Reinstatement is allowed only once during the policy year, and the maximum amount payable under a single claim shall not exceed the sum insured as mentioned in the schedule.\n",
"On renewal of policies with a term of one year, a NCD of flat 5% shall be allowed on the base premium, provided claims are not reported in the expiring Policy.\n",
"According to the document, insured persons can claim reimbursement for a health check-up at the end of a block of two continuous policy years, provided the policy has been continuously renewed with the company without a break.\n",
"No, spectacles, contact lenses, and hearing aids are not covered. (See Section 4.27: Spectacles, contact lens, hearing aid, cochlear implants.)\n","The waiting period for treatment for joint replacement, unless it arises from an accident, is three years.\n",
"The policy defines 'Domiciliary Hospitalisation' as:\n\n\"means medical treatment for an illness /injury which in the normal course would require care and treatment at a hospital but is actually taken while confined at home under any of the following circumstances:\n\ni. the condition of the patient is such that he/she is not in a condition to be removed to a hospital, or\nii. the patient takes treatment at home on account of non availability of bed/ room in a hospital.\"\n",
"The time limit to submit documents for a reimbursement claim after hospitalization, pre-hospitalization expenses, and ambulance charges is within **fifteen days from the date of discharge from the hospital.**\n","No, dental treatment is not covered if it is not necessitated due to an injury. (See Section 3.1.1,vii)\n",
"The daily hospital cash allowance for Plan C is INR 2,000, max. of 5 days.\n"]}%