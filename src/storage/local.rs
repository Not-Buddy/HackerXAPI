use std::path::PathBuf;
use async_trait::async_trait;
use crate::error::Result;
use super::StorageBackend;

pub struct LocalStorage {
    base_dir: PathBuf,
}

impl LocalStorage {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn abs_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put(&self, key: &str, data: &[u8], _mime: &str) -> Result<()> {
        let path = self.abs_path(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.abs_path(key);
        let data = tokio::fs::read(&path).await?;
        Ok(data)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.abs_path(key).exists())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.abs_path(key);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    async fn get_local_path(&self, key: &str) -> Result<PathBuf> {
        let path = self.abs_path(key);
        if !path.exists() {
            return Err(crate::error::AppError::NotFound(format!(
                "File not found: {}",
                key
            )));
        }
        Ok(path)
    }
}
