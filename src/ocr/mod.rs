use image::DynamicImage;
use crate::error::Result;

pub trait OcrEngine: Send + Sync {
    fn extract_text_from_image(&self, image: &DynamicImage) -> Result<String>;
}

pub mod paddle;
