use std::path::Path;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

pub struct PdfExtractor;

impl TextExtractor for PdfExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let text = extract_text_quiet(path).map_err(|e| {
            AppError::Extraction(format!("PDF extraction failed: {}", e))
        })?;

        let trimmed_len = text.trim().len();
        if trimmed_len < 50 {
            let page_count = lopdf::Document::load(path)
                .map_err(|e| AppError::Extraction(format!("Failed to open PDF: {}", e)))?
                .get_pages()
                .len();

            if page_count > 0 && trimmed_len < 50 {
                return Err(AppError::Ocr(format!(
                    "PDF appears to be scanned ({} pages, only {} chars extracted). OCR engine required.",
                    page_count, trimmed_len
                )));
            }
        }

        Ok(text)
    }
}

#[cfg(unix)]
fn extract_text_quiet(path: &Path) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use std::os::unix::io::AsRawFd;

    let devnull = std::fs::File::open("/dev/null")?;
    let saved_out;
    let saved_err;
    unsafe {
        saved_out = libc::dup(1);
        saved_err = libc::dup(2);
        libc::dup2(devnull.as_raw_fd(), 1);
        libc::dup2(devnull.as_raw_fd(), 2);
    }
    let result = pdf_extract::extract_text(path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
    unsafe {
        libc::dup2(saved_out, 1);
        libc::dup2(saved_err, 2);
        libc::close(saved_out);
        libc::close(saved_err);
    }
    result
}

#[cfg(not(unix))]
fn extract_text_quiet(path: &Path) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
    pdf_extract::extract_text(path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}
