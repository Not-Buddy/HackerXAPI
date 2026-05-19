use std::path::Path;

use calamine::{open_workbook_auto, DataType, Reader};

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

pub struct XlsxExtractor;

impl TextExtractor for XlsxExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["xlsx"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| AppError::Extraction(format!("Failed to open XLSX: {}", e)))?;

        let mut output = String::new();

        for sheet_name in workbook.sheet_names().to_owned() {
            if let Some(Ok(range)) = workbook.worksheet_range(&sheet_name) {
                output.push_str(&format!("=== Sheet: {} ===\n", sheet_name));

                for row in range.rows() {
                    let mut row_text = String::new();
                    for (col_idx, cell) in row.iter().enumerate() {
                        if col_idx > 0 {
                            row_text.push_str(" | ");
                        }
                        let cell_str = match cell {
                            DataType::String(s) => s.to_string(),
                            DataType::Float(f) => f.to_string(),
                            DataType::Int(i) => i.to_string(),
                            DataType::Bool(b) => b.to_string(),
                            DataType::DateTime(dt) => dt.to_string(),
                            DataType::DateTimeIso(dt) => dt.to_string(),
                            DataType::Duration(d) => d.to_string(),
                            DataType::DurationIso(d) => d.to_string(),
                            DataType::Error(e) => format!("ERROR: {:?}", e),
                            DataType::Empty => String::new(),
                        };
                        row_text.push_str(&cell_str);
                    }
                    let trimmed = row_text.trim().to_string();
                    if !trimmed.is_empty() {
                        output.push_str(&trimmed);
                        output.push('\n');
                    }
                }
                output.push('\n');
            }
        }

        if output.trim().is_empty() {
            return Err(AppError::Extraction("No data found in XLSX file".into()));
        }

        Ok(output)
    }
}
