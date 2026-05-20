use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::Result;

pub mod local;
pub mod r2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    pub id: uuid::Uuid,
    pub original_name: String,
    pub storage_key: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

impl StoredFile {
    pub fn new(original_name: &str, data_len: u64) -> Self {
        let id = uuid::Uuid::new_v4();
        let ext = Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let mime_type = mime_from_ext(ext);
        let storage_key = format!("files/{}/{}", id, original_name);

        Self {
            id,
            original_name: original_name.to_string(),
            storage_key,
            mime_type,
            size_bytes: data_len,
        }
    }

    pub fn extension(&self) -> &str {
        Path::new(&self.original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
    }

    pub fn doc_id(&self) -> String {
        self.id.to_string()
    }
}

fn mime_from_ext(ext: &str) -> String {
    mime_guess::from_ext(ext)
        .first_or_octet_stream()
        .to_string()
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put(&self, key: &str, data: &[u8], mime: &str) -> Result<()>;
    #[allow(dead_code)]
    async fn get(&self, key: &str) -> Result<Vec<u8>>;
    async fn exists(&self, key: &str) -> Result<bool>;
    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> Result<()>;
    async fn get_local_path(&self, key: &str) -> Result<PathBuf>;
}
