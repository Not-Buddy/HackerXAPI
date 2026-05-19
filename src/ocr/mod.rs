use image::DynamicImage;
use std::path::Path;
use crate::error::Result;

pub trait OcrEngine: Send + Sync {
    fn extract_text_from_image(&self, image: &DynamicImage) -> Result<String>;
    fn extract_text_from_path(&self, path: &Path) -> Result<String>;
}

pub mod paddle;
