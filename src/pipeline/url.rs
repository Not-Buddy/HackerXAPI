use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::error::{AppError, Result};

pub async fn generate_filename_from_url(url: &str) -> Result<String> {
    let parsed_url = Url::parse(url).map_err(|e| AppError::Any(e.into()))?;

    let path_segment = parsed_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("")
        .to_string();

    let clean_path_segment = path_segment
        .split('?')
        .next()
        .unwrap_or(&path_segment)
        .to_string();

    if clean_path_segment == "get-secret-token" {
        return handle_secret_token_endpoint(url).await;
    }

    let unsupported_exts = ["zip", "bin"];
    let allowed_exts = [
        "jpeg", "jpg", "pptx", "docx", "xlsx", "png", "pdf", "txt", "json", "xml",
    ];

    let has_unsupported_ext = unsupported_exts.iter().any(|ext| {
        clean_path_segment
            .to_lowercase()
            .ends_with(&format!(".{}", ext))
    });

    if has_unsupported_ext {
        return Err(AppError::UnsupportedFormat(
            "ZIP and BIN files are not supported.".into(),
        ));
    }

    let has_allowed_ext = allowed_exts.iter().any(|ext| {
        clean_path_segment
            .to_lowercase()
            .ends_with(&format!(".{}", ext))
    });

    let final_filename = if has_allowed_ext && !clean_path_segment.is_empty() {
        clean_path_segment
    } else if !clean_path_segment.is_empty() && clean_path_segment.len() > 3 {
        if is_likely_api_endpoint(&clean_path_segment) {
            format!("{}.json", clean_path_segment)
        } else {
            format!("{}.pdf", clean_path_segment)
        }
    } else {
        format!("document_{}.pdf", hash_url(url))
    };

    let sanitized: String = final_filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    Ok(sanitized)
}

async fn handle_secret_token_endpoint(url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(AppError::Http(format!(
            "HTTP request failed with status: {}",
            response.status()
        )));
    }

    let content = response.text().await?;
    let filename = format!("secret_token_{}.txt", hash_url(url));
    let file_path = format!("pdfs/{}", filename);

    tokio::fs::create_dir_all("pdfs").await?;

    let mut file = File::create(&file_path).await?;
    file.write_all(content.as_bytes()).await?;

    println!("Secret token saved to: {}", file_path);

    Ok(filename)
}

fn is_likely_api_endpoint(segment: &str) -> bool {
    let api_indicators = [
        "api", "get", "post", "fetch", "data", "token", "auth", "secret",
    ];
    let segment_lower = segment.to_lowercase();

    api_indicators
        .iter()
        .any(|indicator| segment_lower.contains(indicator))
        || segment.contains('-')
        || segment.contains('_')
}

fn hash_url(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
