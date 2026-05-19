use std::path::Path;
use crate::error::Result;

pub trait TextExtractor: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn extract_text(&self, path: &Path) -> Result<String>;
}

pub mod pdf;
pub mod docx;
pub mod xlsx;
pub mod pptx;
pub mod image;
pub mod text;
pub mod libreoffice;
