use crate::error::Result;

pub async fn download_file(url: &str, file_path: &str) -> Result<()> {
    let allowed_exts = ["jpeg", "pptx", "docx", "xlsx", "png", "pdf"];
    let ignore_exts = ["zip", "bin"];

    let parsed_url = url::Url::parse(url).map_err(|e| crate::error::AppError::Any(e.into()))?;
    let path = parsed_url.path();

    let filename = path.split('/').next_back().unwrap_or("");
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    println!("Extension is: {}", ext);

    if ignore_exts.contains(&ext.as_str()) {
        println!("Ignoring download for file with extension: {}", ext);
        return Ok(());
    }

    if !allowed_exts.contains(&ext.as_str()) {
        return Err(crate::error::AppError::UnsupportedFormat(format!(
            "Download not supported for files with .{} extension",
            ext
        )));
    }

    let bytes = reqwest::get(url).await?.bytes().await?;
    tokio::fs::write(file_path, &bytes).await?;

    Ok(())
}
