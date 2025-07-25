use tokio::fs as async_fs;
use std::fs;
use pdf_extract;
use std::path::Path;

pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}

fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    let text = pdf_extract::extract_text(file_path)?;
    Ok(text)
}

pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}

/// Utility function to delete file - you can just import std::fs::remove_file where needed,
/// but it's fine to add here if you want.
pub fn delete_file(path: &str) -> std::io::Result<()> {
    if Path::new(path).exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
