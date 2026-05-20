use crate::error::Result;

pub async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let parsed_url = url::Url::parse(url).map_err(|e| crate::error::AppError::Any(e.into()))?;
    let path = parsed_url.path();
    let filename = path.split('/').next_back().unwrap_or("");
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    let allowed_exts = ["jpeg", "jpg", "pptx", "docx", "xlsx", "png", "pdf", "txt"];
    let ignore_exts = ["zip", "bin"];

    if ignore_exts.contains(&ext.as_str()) {
        return Err(crate::error::AppError::UnsupportedFormat(format!(
            ".{} files are not supported", ext
        )));
    }

    if !allowed_exts.contains(&ext.as_str()) {
        return Err(crate::error::AppError::UnsupportedFormat(format!(
            "Download not supported for .{} files", ext
        )));
    }

    println!("Downloading: {} (ext={})", url, ext);
    let bytes = reqwest::get(url).await?.bytes().await?;
    Ok(bytes.to_vec())
}
