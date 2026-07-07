use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

static MARKITDOWN_AVAILABLE: OnceLock<bool> = OnceLock::new();

fn markitdown_available() -> bool {
    *MARKITDOWN_AVAILABLE.get_or_init(|| {
        Command::new("markitdown")
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

pub struct MarkitdownExtractor;

impl TextExtractor for MarkitdownExtractor {
    fn supported_extensions(&self) -> &[&str] {
        if markitdown_available() {
            &[
                "pdf", "docx", "pptx", "xlsx", "xls",
                "html", "csv", "json", "xml", "epub",
            ]
        } else {
            &[]
        }
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        if !markitdown_available() {
            return Err(AppError::Extraction(
                "markitdown is not available on PATH. Install with: pip install 'markitdown[all]'"
                    .into(),
            ));
        }

        let output = Command::new("markitdown")
            .arg(path)
            .output()
            .map_err(|e| AppError::Extraction(format!("Failed to run markitdown: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Extraction(format!(
                "markitdown conversion failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();

        // For PDFs: detect scanned documents that yield little/no text.
        // markitdown (via pdfminer) cannot OCR image-based PDFs.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "pdf" {
            let trimmed_len = text.trim().len();
            if trimmed_len < 50 {
                return Err(AppError::Ocr(format!(
                    "PDF appears to be scanned (only {} chars extracted by markitdown). OCR engine required.",
                    trimmed_len
                )));
            }
        }

        if text.trim().is_empty() {
            return Err(AppError::Extraction(format!(
                "markitdown produced no output for: {}",
                path.display()
            )));
        }

        Ok(text)
    }
}
