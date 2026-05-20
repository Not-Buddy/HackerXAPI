use std::path::{Path, PathBuf};
use async_trait::async_trait;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region, BehaviorVersion};
use crate::error::Result;
use super::StorageBackend;

pub struct R2Storage {
    client: Client,
    bucket: String,
    temp_dir: PathBuf,
}

impl R2Storage {
    pub async fn new(
        account_id: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
    ) -> Self {
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        let creds = Credentials::new(access_key, secret_key, None, None, "r2");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(&endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();
        let client = Client::from_conf(config);

        let temp_dir = std::env::temp_dir().join("ragx_r2_cache");
        tokio::fs::create_dir_all(&temp_dir).await.ok();

        Self {
            client,
            bucket: bucket.to_string(),
            temp_dir,
        }
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| crate::error::AppError::VectorStore(format!("R2 get failed: {}", e)))?;

        let data = resp.body.collect().await
            .map_err(|e| crate::error::AppError::VectorStore(format!("R2 read body: {}", e)))?;
        Ok(data.to_vec())
    }
}

#[async_trait]
impl StorageBackend for R2Storage {
    async fn put(&self, key: &str, data: &[u8], mime: &str) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.to_vec().into())
            .content_type(mime)
            .send()
            .await
            .map_err(|e| crate::error::AppError::VectorStore(format!("R2 put failed: {}", e)))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        self.get_object(key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| crate::error::AppError::VectorStore(format!("R2 delete failed: {}", e)))?;
        Ok(())
    }

    async fn get_local_path(&self, key: &str) -> Result<PathBuf> {
        let data = self.get_object(key).await?;
        let filename = Path::new(key)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let cache_path = self.temp_dir.join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&cache_path).await?;
        let file_path = cache_path.join(filename);
        tokio::fs::write(&file_path, &data).await?;
        Ok(file_path)
    }
}
