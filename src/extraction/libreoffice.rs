use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

static SOFFICE_AVAILABLE: OnceLock<bool> = OnceLock::new();

fn soffice_available() -> bool {
    *SOFFICE_AVAILABLE.get_or_init(|| {
        Command::new("which")
            .arg("soffice")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

pub struct LibreOfficeExtractor;

impl TextExtractor for LibreOfficeExtractor {
    fn supported_extensions(&self) -> &[&str] {
        if soffice_available() {
            &["docx", "pptx", "xlsx"]
        } else {
            &[]
        }
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        if !soffice_available() {
            return Err(AppError::Extraction(
                "LibreOffice not available on PATH".into(),
            ));
        }

        let temp_dir = std::env::temp_dir().join(format!("ragx_soffice_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        let status = Command::new("soffice")
            .arg("--headless")
            .arg("--convert-to")
            .arg("txt:Text")
            .arg("--outdir")
            .arg(&temp_dir)
            .arg(path)
            .status()
            .map_err(|e| AppError::Extraction(format!("Failed to run soffice: {}", e)))?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AppError::Extraction(
                "LibreOffice conversion failed".into(),
            ));
        }

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let txt_path = temp_dir.join(format!("{}.txt", file_stem));

        let text = std::fs::read_to_string(&txt_path)
            .map_err(|e| AppError::Extraction(format!("Failed to read converted text: {}", e)))?;

        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(text)
    }
}
