use std::path::Path;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

pub struct PdfExtractor;

impl TextExtractor for PdfExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let text = pdf_extract::extract_text(path).map_err(|e| {
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
