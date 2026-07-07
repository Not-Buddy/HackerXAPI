use std::path::Path;

use crate::error::Result;
use crate::extraction::TextExtractor;

pub struct PlainTextExtractor;

impl TextExtractor for PlainTextExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["txt", "md"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(Into::into)
    }
}
