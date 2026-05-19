use image::DynamicImage;
use std::path::Path;
use crate::error::{Result, AppError};

// ocrs integration requires these in Cargo.toml:
//   ocrs = "0.12"
//   rten = "0.2"
//   rten-tensor = "0.1"
//
// OcrEngine in ocrs is !Send so wrap with Arc<Mutex<>> or create per-call.
// The pipeline below uses ocrs::OcrEngine, ocrs::OcrEngineParams,
// rten_tensor::NdTensor, and engine.prepare_input() + engine.get_text().
//
// To convert DynamicImage -> NdTensor<f32, 3> (CHW format, values 0-1):
//   - Call image.to_rgb8() for HWC u8 data
//   - Create NdTensor::zeros([3, h, w])
//   - Iterate pixels, normalizing to f32 / 255.0

pub struct PaddleOcrEngine;

impl PaddleOcrEngine {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl super::OcrEngine for PaddleOcrEngine {
    fn extract_text_from_image(&self, _image: &DynamicImage) -> Result<String> {
        Err(AppError::Ocr(
            "PaddleOcrEngine: ocrs crate integration not yet enabled. \
             Add `ocrs`, `rten`, and `rten-tensor` to Cargo.toml and \
             implement NdTensor conversion from DynamicImage.".into()
        ))
    }

    fn extract_text_from_path(&self, path: &Path) -> Result<String> {
        let image = image::open(path)
            .map_err(|e| AppError::Ocr(format!("Failed to open image {:?}: {}", path, e)))?;
        self.extract_text_from_image(&image)
    }
}
