use std::path::Path;
use std::sync::OnceLock;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;
use crate::ocr::OcrEngine;

static OCR_ENGINE: OnceLock<Box<dyn OcrEngine>> = OnceLock::new();

pub fn set_ocr_engine(engine: Box<dyn OcrEngine>) {
    let _ = OCR_ENGINE.set(engine);
}

pub struct ImageExtractor;

impl TextExtractor for ImageExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg", "bmp", "tiff"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let img = image::open(path)
            .map_err(|e| AppError::Extraction(format!("Failed to open image: {}", e)))?;

        let engine = OCR_ENGINE
            .get()
            .ok_or_else(|| AppError::Ocr("OCR engine not configured. Call set_ocr_engine() first.".into()))?;

        engine.extract_text_from_image(&img)
    }
}
