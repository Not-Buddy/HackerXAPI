use url::Url;
use crate::error::{AppError, Result};

pub fn extract_filename_from_url(url: &str) -> Result<String> {
    let parsed_url = Url::parse(url).map_err(|e| AppError::Any(e.into()))?;

    let path_segment = parsed_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("")
        .to_string();

    let clean = path_segment
        .split('?')
        .next()
        .unwrap_or(&path_segment)
        .to_string();

    if clean.is_empty() {
        return Err(AppError::UnsupportedFormat("Empty filename in URL".into()));
    }

    let sanitized: String = clean
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
