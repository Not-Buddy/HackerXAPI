use tokio::fs as async_fs;

pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}

fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    use std::fs;
    use std::path::Path;

    // Ensure output dir
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    // Extract text directly from the PDF file
    let text = pdf_extract::extract_text(file_path)?;
    
    // Clean up the extracted text - remove excessive whitespace while preserving structure
    let cleaned_text = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");

    // Write result file
    let txt_path = pdfs_dir.join("policy.txt");
    fs::write(&txt_path, &cleaned_text)?;
    println!("Saved extracted text to {:?}", txt_path);

    Ok(cleaned_text)
}

pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}
