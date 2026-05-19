use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{AppError, Result};
use crate::extraction::TextExtractor;

pub struct DocxExtractor;

impl TextExtractor for DocxExtractor {
    fn supported_extensions(&self) -> &[&str] {
        &["docx"]
    }

    fn extract_text(&self, path: &Path) -> Result<String> {
        let file = std::fs::File::open(path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| AppError::Extraction(format!("Failed to read DOCX as ZIP: {}", e)))?;

        let mut doc_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| AppError::Extraction(format!("Failed to find document.xml: {}", e)))?;

        let mut xml_str = String::new();
        doc_xml.read_to_string(&mut xml_str)?;

        extract_text_from_docx_xml(&xml_str)
    }
}

fn extract_text_from_docx_xml(xml_str: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml_str);
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut in_paragraph = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"w:p" => in_paragraph = true,
                b"w:t" => in_text = true,
                _ => {}
            },
            Ok(Event::Text(ref e)) if in_text => {
                output.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"w:p" => {
                    if in_paragraph {
                        output.push('\n');
                    }
                    in_paragraph = false;
                }
                b"w:t" => in_text = false,
                _ => {}
            },
            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"w:br" | b"w:cr" => output.push('\n'),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Extraction(format!("XML parse error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    if output.trim().is_empty() {
        return Err(AppError::Extraction("No text found in DOCX file".into()));
    }

    Ok(output)
}
