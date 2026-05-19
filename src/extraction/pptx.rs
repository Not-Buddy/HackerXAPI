use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

pub struct PptxExtractor;

impl TextExtractor for PptxExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["pptx"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let file = std::fs::File::open(path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| AppError::Extraction(format!("Failed to read PPTX as ZIP: {}", e)))?;

        let mut slide_names: Vec<String> = archive
            .file_names()
            .filter(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
            .map(|s| s.to_string())
            .collect();

        if slide_names.is_empty() {
            return Err(AppError::Extraction("No slides found in PPTX file".into()));
        }

        slide_names.sort_by_key(|name| {
            name.trim_start_matches("ppt/slides/slide")
                .trim_end_matches(".xml")
                .parse::<u32>()
                .unwrap_or(0)
        });

        let mut output = String::new();

        for (idx, slide_name) in slide_names.iter().enumerate() {
            let mut slide_xml = archive.by_name(slide_name).map_err(|e| {
                AppError::Extraction(format!("Failed to read {}: {}", slide_name, e))
            })?;
            let mut xml_str = String::new();
            slide_xml.read_to_string(&mut xml_str)?;

            let slide_text = extract_text_from_slide_xml(&xml_str)?;
            let trimmed = slide_text.trim();
            if !trimmed.is_empty() {
                output.push_str(&format!("=== Slide {} ===\n{}\n", idx + 1, trimmed));
            }
        }

        if output.trim().is_empty() {
            return Err(AppError::Extraction("No text found in PPTX file".into()));
        }

        Ok(output)
    }
}

fn extract_text_from_slide_xml(xml_str: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml_str);
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut in_paragraph = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"a:p" => in_paragraph = true,
                b"a:t" => in_text = true,
                _ => {}
            },
            Ok(Event::Text(ref e)) if in_text => {
                output.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"a:p" => {
                    if in_paragraph {
                        output.push('\n');
                    }
                    in_paragraph = false;
                }
                b"a:t" => in_text = false,
                _ => {}
            },
            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"a:br" => output.push('\n'),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AppError::Extraction(format!(
                    "XML parse error in slide: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(output)
}
