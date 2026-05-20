# ragx — RAG Document Query Engine

A cross-platform RAG (Retrieval-Augmented Generation) pipeline in Rust. Upload documents via URL, get AI-powered answers backed by chunk embeddings stored in Qdrant.


## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Axum HTTP  │────▶│   Pipeline   │────▶│  Gemini API      │
│  :8000      │     │  Orchestrator│     │  embed + generate │
└─────────────┘     └──────┬───────┘     └─────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
   ┌──────────┐    ┌──────────┐     ┌──────────┐
   │ Storage  │    │Extraction│     │ Qdrant   │
   │ local/R2 │    │ pdf/docx │     │ Vector   │
   │          │    │ xlsx/pptx│     │ Store    │
   └──────────┘    │ image/txt│     └──────────┘
                   └──────────┘
```

## Module Structure

```
src/
├── main.rs              — Startup: model discovery, storage prompt, server bootstrap
├── config.rs            — All env-var configuration
├── error.rs             — thiserror-based AppError + Result<T>
├── server/
│   ├── mod.rs           — Router: POST /api/v1/rag/query
│   ├── handlers.rs      — Request handler with auth + pipeline orchestration
│   └── auth.rs          — Bearer token extraction
├── pipeline/
│   ├── mod.rs           — Pipeline: process → embed → search → answer
│   ├── download.rs      — download_bytes(url) → Vec<u8>
│   └── url.rs           — extract_filename_from_url(url) → String
├── extraction/
│   ├── mod.rs           — TextExtractor trait
│   ├── pdf.rs           — pdf-extract + lopdf (pure Rust)
│   ├── docx.rs          — zip + quick-xml (pure Rust)
│   ├── xlsx.rs          — calamine (pure Rust)
│   ├── pptx.rs          — zip + quick-xml (pure Rust)
│   ├── image.rs         — image crate → OcrEngine
│   ├── text.rs          — fs::read_to_string
│   └── libreoffice.rs   — Optional soffice fallback
├── ocr/
│   ├── mod.rs           — OcrEngine trait
│   └── paddle.rs        — ocrs + rten (RTen-based OCR, auto-downloads models)
├── ai/
│   ├── mod.rs           — Module re-exports
│   ├── traits.rs        — EmbeddingProvider + LlmProvider traits
│   └── gemini/
│       ├── mod.rs       — GeminiProvider: model discovery + constructor
│       ├── client.rs    — reqwest Client builder, backoff/retry helpers
│       ├── embed.rs     — EmbedClient: impl EmbeddingProvider
│       ├── llm.rs       — LlmClient: impl LlmProvider
│       ├── types.rs     — All serde structs + ModelInfo
│       ├── safety.rs    — sanitize_policy() prompt injection defense
│       └── models.rs    — discover_models() API + interactive selection
├── vectordb/
│   ├── mod.rs           — VectorStore trait + ChunkEmbedding/ScoredChunk
│   └── qdrant.rs        — QdrantStore: full gRPC CRUD + cosine search
└── storage/
    ├── mod.rs           — StoredFile struct + StorageBackend trait
    ├── local.rs         — LocalStorage: files on disk
    └── r2.rs            — R2Storage: Cloudflare R2 via aws-sdk-s3
```

## Data Flow

```
URL
  ↓  download_bytes(url)
Vec<u8>
  ↓  StoredFile::new(filename, len)
StoredFile { id: uuid, storage_key, mime_type }
  ↓  storage.put(key, bytes, mime)
  ↓  storage.get_local_path(key) → PathBuf
  ↓  extractor.extract_text(&Path) → String
  ↓  chunk_text(text, 8000 chars)
  ↓  embed_provider.embed(chunk) → Vec<f32>  × N chunks
  ↓  vector_store.store_embeddings(doc_id, chunks)
  ↓  embed_provider.embed(questions) → query vector
  ↓  vector_store.search_similar(query, top_k, threshold)
  ↓  llm_provider.generate(context + questions, schema) → JSON answers
[Qdrant: cosine similarity, 3072-dim vectors]
```

## Quick Start

### Prerequisites
- Rust (latest stable)
- Qdrant (Cloud or Docker)
- Gemini API key

### Setup

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Start Qdrant (skip if using Qdrant Cloud)
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant

# 3. Configure
cp .envexample .env
# Edit .env with your GEMINI_KEY and Qdrant credentials

# 4. Run
cargo run
```

The startup flow is interactive:
1. Discovers available Gemini models via API
2. Auto-selects `text-embedding-004` for embeddings (768 dims)
3. Prompts you to choose an LLM model from available list
4. Prompts for storage backend (local disk or Cloudflare R2)
5. OCR models auto-download on first run (~30MB)

### Testing

```bash
# Start the server, then in another terminal:
./test.sh
```

Sends PDFs from `tests/` through the API, validates JSON responses.

### API

```
POST /api/v1/rag/query
Authorization: Bearer <token>
Content-Type: application/json

{
    "documents": "https://example.com/document.pdf",
    "questions": [
        "What is the grace period?",
        "What does section 4.1 cover?"
    ]
}

→ {
    "answers": [
        "The grace period is 30 days...",
        "Section 4.1 covers..."
    ]
}
```

## Trait Abstractions

### VectorStore
```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_embeddings(&self, doc_id: &str, chunks: &[ChunkEmbedding]) -> Result<()>;
    async fn get_embeddings(&self, doc_id: &str) -> Result<Vec<ChunkEmbedding>>;
    async fn embeddings_exist(&self, doc_id: &str) -> Result<bool>;
    async fn search_similar(&self, embedding: &[f32], top_k: usize, threshold: f32) -> Result<Vec<ScoredChunk>>;
}
```
**Impl**: `QdrantStore` — gRPC client, cosine distance, payload indexes, auto-creates collection.

### EmbeddingProvider + LlmProvider
```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str, schema: Option<Value>) -> Result<String>;
}
```
**Impl**: `GeminiProvider` — delegates to `EmbedClient` (embedding-001, 3072 dims) and `LlmClient` (user-selected flash model). Exponential backoff with jitter, Retry-After header parsing, 15s connect / 120s request timeouts.

### TextExtractor
```rust
pub trait TextExtractor: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn extract_text(&self, path: &Path) -> Result<String>;
}
```
**Impls**: `PdfExtractor`, `DocxExtractor`, `XlsxExtractor`, `PptxExtractor`, `PlainTextExtractor`, `ImageExtractor`, `LibreOfficeExtractor` (optional).

### OcrEngine
```rust
pub trait OcrEngine: Send + Sync {
    fn extract_text_from_image(&self, image: &DynamicImage) -> Result<String>;
}
```
**Impl**: `PaddleOcrEngine` — `ocrs` crate (RTen inference), auto-downloads detection + recognition models.

### StorageBackend
```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put(&self, key: &str, data: &[u8], mime: &str) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Vec<u8>>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn get_local_path(&self, key: &str) -> Result<PathBuf>;
}
```
**Impls**: `LocalStorage` (filesystem under `./data/files/`), `R2Storage` (Cloudflare R2 via `aws-sdk-s3`).

## Configuration

| Env Var | Default | Description |
|---|---|---|
| `GEMINI_KEY` | *required* | Google Gemini API key |
| `QDRANT_URL` | `http://localhost:6334` | Qdrant gRPC endpoint |
| `QDRANT_API_KEY` | — | Qdrant Cloud API key |
| `QDRANT_COLLECTION` | `rag_embeddings` | Qdrant collection name |
| `SERVER_PORT` | `8000` | HTTP server port |
| `CHUNK_SIZE` | `8000` | Characters per text chunk |
| `TOP_K` | `10` | Chunks to retrieve for context |
| `SIMILARITY_THRESHOLD` | `0.3` | Minimum cosine similarity |
| `EMBED_MODEL` | `auto` | Embedding model (auto-discover or pin) |
| `LLM_MODEL` | `prompt` | LLM model (interactive pick or pin) |
| `AUTO_DISCOVER_MODELS` | `true` | Query Gemini for available models |
| `STORAGE_BACKEND` | `prompt` | Storage: `local`, `r2`, or `prompt` |
| `STORAGE_LOCAL_DIR` | `./data/files` | Local storage directory |
| `R2_ACCOUNT_ID` | — | Cloudflare R2 account ID |
| `R2_ACCESS_KEY_ID` | — | R2 access key |
| `R2_SECRET_ACCESS_KEY` | — | R2 secret key |
| `R2_BUCKET` | — | R2 bucket name |

## Key Features

- **Pure-Rust document extraction** — PDF (`pdf-extract` + `lopdf`), DOCX (`quick-xml`), XLSX (`calamine`), PPTX (`quick-xml`), images (`ocrs` + `rten`). LibreOffice retained as optional fallback.
- **Persistent embeddings** — Qdrant stores chunk vectors with cosine similarity search. Embeddings survive server restarts.
- **Rate-limit resilience** — Exponential backoff with jitter, `Retry-After` header parsing, 200ms inter-chunk throttle.
- **Structured logging** — Every API call logged with timing: `[embed] 200 OK (742ms) 8000B chunk`, `[llm] 200 OK (3240ms) 12840B prompt`. Per-request summary with call counts.
- **UUID-based doc identity** — Files get unique UUIDs stored alongside their content. No filename collisions.
- **Prompt injection defense** — 22-pattern regex sanitization applied to all LLM inputs.
- **Configurable** — Everything tunable via environment variables. No recompile needed.
- **Cross-platform** — Linux, macOS, Windows. No system packages required.
