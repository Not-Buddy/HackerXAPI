use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use crate::error::{AppError, Result};

const DET_MODEL_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const REC_MODEL_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

static ENGINE: OnceLock<std::result::Result<ocrs::OcrEngine, String>> = OnceLock::new();

fn get_engine() -> Result<&'static ocrs::OcrEngine> {
    let result = ENGINE.get_or_init(|| {
        let model_dir = std::env::var("OCR_MODEL_DIR").unwrap_or_else(|_| "models".to_string());
        let det_path = PathBuf::from(&model_dir).join("text-detection.rten");
        let rec_path = PathBuf::from(&model_dir).join("text-recognition.rten");

        if !det_path.exists() || !rec_path.exists() {
            return Err(format!(
                "OCR models not found in '{}'. Download them:\n  mkdir -p {0}\n  curl -L '{}' -o {0}/text-detection.rten\n  curl -L '{}' -o {0}/text-recognition.rten",
                model_dir, DET_MODEL_URL, REC_MODEL_URL
            ));
        }

        let det_model = rten::Model::load_file(&det_path).map_err(|e| format!("Failed to load detection model: {}", e))?;
        let rec_model = rten::Model::load_file(&rec_path).map_err(|e| format!("Failed to load recognition model: {}", e))?;

        let params = ocrs::OcrEngineParams {
            detection_model: Some(det_model),
            recognition_model: Some(rec_model),
            debug: false,
            decode_method: ocrs::DecodeMethod::default(),
            alphabet: None,
            allowed_chars: None,
        };

        ocrs::OcrEngine::new(params).map_err(|e| format!("Failed to create OCR engine: {}", e))
    });

    match result {
        Ok(engine) => Ok(engine),
        Err(msg) => Err(AppError::Ocr(msg.clone())),
    }
}

/// Downloads OCR models automatically on first run (async, ~30MB total).
pub async fn download_models_if_needed(model_dir: &Path) -> Result<()> {
    let det_path = model_dir.join("text-detection.rten");
    let rec_path = model_dir.join("text-recognition.rten");

    if det_path.exists() && rec_path.exists() {
        return Ok(());
    }

    println!("Downloading OCR models to {:?}...", model_dir);
    std::fs::create_dir_all(model_dir)?;

    if !det_path.exists() {
        println!("Downloading text-detection.rten (~12MB)...");
        let response = reqwest::get(DET_MODEL_URL).await?;
        if !response.status().is_success() {
            return Err(AppError::Ocr(format!(
                "Failed to download detection model: HTTP {}", response.status()
            )));
        }
        let bytes = response.bytes().await?;
        if bytes.len() < 100_000 {
            return Err(AppError::Ocr(
                "Downloaded detection model is too small — likely an error page".into()
            ));
        }
        std::fs::write(&det_path, &bytes)?;
    }

    if !rec_path.exists() {
        println!("Downloading text-recognition.rten (~18MB)...");
        let response = reqwest::get(REC_MODEL_URL).await?;
        if !response.status().is_success() {
            return Err(AppError::Ocr(format!(
                "Failed to download recognition model: HTTP {}", response.status()
            )));
        }
        let bytes = response.bytes().await?;
        if bytes.len() < 100_000 {
            return Err(AppError::Ocr(
                "Downloaded recognition model is too small — likely an error page".into()
            ));
        }
        std::fs::write(&rec_path, &bytes)?;
    }

    println!("OCR models downloaded to {:?}", model_dir);
    Ok(())
}

pub struct PaddleOcrEngine;

impl PaddleOcrEngine {
    pub fn new() -> Result<Self> {
        // Trigger model loading; fails gracefully if models missing
        let _ = get_engine()?;
        Ok(Self)
    }
}

impl super::OcrEngine for PaddleOcrEngine {
    fn extract_text_from_image(&self, image: &DynamicImage) -> Result<String> {
        let engine = get_engine()?;
        let rgb = image.to_rgb8();
        let (w, h) = rgb.dimensions();

        let source = ocrs::ImageSource::from_bytes(
            rgb.as_raw(),
            (w, h),
        ).map_err(|e| AppError::Ocr(format!("Failed to create image source: {}", e)))?;

        let input = engine
            .prepare_input(source)
            .map_err(|e| AppError::Ocr(format!("Failed to prepare OCR input: {}", e)))?;

        let text = engine
            .get_text(&input)
            .map_err(|e| AppError::Ocr(format!("OCR text extraction failed: {}", e)))?;

        Ok(text)
    }

    fn extract_text_from_path(&self, path: &Path) -> Result<String> {
        let image = image::open(path)
            .map_err(|e| AppError::Ocr(format!("Failed to open image {:?}: {}", path, e)))?;
        self.extract_text_from_image(&image)
    }
}
