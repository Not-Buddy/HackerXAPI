use tokio::fs as async_fs;
use rayon::prelude::*;
use std::sync::Arc;
use std::process::Command;
use std::fs;
use std::path::Path;
use num_cpus;
use docx_rs::*;
use calamine::{open_workbook_auto, Reader, DataType, Range};
use printpdf::*;
use std::io::BufWriter;

use crate::ocr::extract_text_from_pptx;



pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_file(url: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Allowed extensions to download
    let allowed_exts = ["jpeg", "pptx", "docx", "xlsx", "png", "pdf"];
    // Extensions to ignore
    let ignore_exts = ["zip", "bin"];

    // Parse URL to extract the path component (without query parameters)
    let parsed_url = url::Url::parse(url)?;
    let path = parsed_url.path();
    
    // Extract filename from path
    let filename = path.split('/').last().unwrap_or("");

    
    
    // Extract the file extension from filename (not the full URL)
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    println!("Extension is: {}", ext);

    // Check if extension is to be ignored
    if ignore_exts.contains(&ext.as_str()) {
        // Skip downloading the file
        println!("Ignoring download for file with extension: {}", ext);
        return Ok(());
    }

    // Check if extension is allowed
    if !allowed_exts.contains(&ext.as_str()) {
        // Return error for unsupported file extension
        return Err(format!("Download not supported for files with .{} extension", ext).into());
    }

    // If allowed, proceed to download
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;

    Ok(())
}

fn extract_file_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    // Determine file extension
    let ext = Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Handle different file types
    match ext.as_str() {
        "docx" => {
            // Convert DOCX to PDF first, then extract text
            let pdf_path = convert_docx_to_pdf(file_path)?;
            extract_pdf_text_sync(&pdf_path)
        }
        "xlsx" => {
            // Convert XLSX to PDF first, then extract text
            let pdf_path = convert_xlsx_to_pdf(file_path)?;
            extract_pdf_text_sync(&pdf_path)
        }
        "pdf" => {
            // Extract directly from PDF
            extract_pdf_text_sync(file_path)
        }
        "jpeg" | "png" => {
            // Extract text directly using OCR from images
            crate::ocr::extract_text_with_ocrs(file_path)

        }
        "pptx" => {
            // Extract PPTX pages as images first, then apply OCR
            extract_text_from_pptx(file_path)
        }

        "txt" => {
            extract_token_from_text(file_path)
        }
        _ => {
            Err(format!("Unsupported file type: .{}", ext).into())
        }
    }
}


// Rename your existing function to avoid conflicts
fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    // Ensure output dir
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    // Generate output filename based on input PDF filename
    let pdf_filename = Path::new(file_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    
    let txt_filename = format!("{}.txt", pdf_filename);
    let txt_path = pdfs_dir.join(&txt_filename);

    // Check if text file already exists
    if txt_path.exists() {
        println!("Text file already exists at {:?}, reading existing content", txt_path);
        let existing_text = fs::read_to_string(&txt_path)?;
        if !existing_text.trim().is_empty() {
            println!("Using existing extracted text ({} characters)", existing_text.len());
            return Ok(existing_text);
        } else {
            println!("Existing text file is empty, re-extracting...");
        }
    }

    println!("Text file not found, extracting from PDF...");

    // Create temp directory for PDF chunks
    let temp_dir = pdfs_dir.join("temp_chunks");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Get total number of pages using pdftk or similar tool
    let total_pages = get_pdf_page_count_accurate(file_path)?;

    // Get number of available CPU cores
    let num_cores = num_cpus::get();
    let pages_per_chunk = (total_pages + num_cores - 1) / num_cores; // Ceiling division
    println!("Total pages: {}, CPU cores: {}, pages per chunk: {}", total_pages, num_cores, pages_per_chunk);

    // Create page ranges for all available cores
    let page_ranges: Vec<(usize, usize)> = (0..num_cores)
        .map(|i| {
            let start = i * pages_per_chunk + 1; // PDF pages are 1-indexed
            let end = ((i + 1) * pages_per_chunk).min(total_pages);
            (start, end)
        })
        .filter(|(start, end)| start <= end)
        .collect();

    println!("Processing {} pages in {} chunks", total_pages, page_ranges.len());

    // Split PDF into chunks and process in parallel
    let file_path = Arc::new(file_path.to_string());
    let temp_dir = Arc::new(temp_dir);
    
    let chunk_results: Result<Vec<String>, Box<StdError>> = page_ranges
        .into_par_iter()
        .enumerate()
        .map(|(chunk_idx, (start_page, end_page))| {
            let file_path = Arc::clone(&file_path);
            let temp_dir = Arc::clone(&temp_dir);
            process_pdf_chunk(&file_path, &temp_dir, start_page, end_page, chunk_idx)
        })
        .collect();

    let chunk_texts = chunk_results?;

    // Clean up temp directory
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&*temp_dir);
    }

    // Combine all chunk results in order
    let combined_text = chunk_texts.join("\n");

    // Clean up the extracted text
    let cleaned_text = combined_text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");

    fs::write(&txt_path, &cleaned_text)?;
    println!("Saved extracted text to {:?}", txt_path);
    Ok(cleaned_text)
}

fn get_pdf_page_count_accurate(file_path: &str) -> Result<usize, Box<StdError>> {
    // Try using pdftk first (most accurate)
    if let Ok(output) = Command::new("pdftk")
        .arg(file_path)
        .arg("dump_data")
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("NumberOfPages:") {
                if let Ok(pages) = line.split(':').nth(1).unwrap_or("0").trim().parse::<usize>() {
                    return Ok(pages);
                }
            }
        }
    }

    // Fallback: try using pdfinfo
    if let Ok(output) = Command::new("pdfinfo")
        .arg(file_path)
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("Pages:") {
                if let Ok(pages) = line.split_whitespace().nth(1).unwrap_or("0").parse::<usize>() {
                    return Ok(pages);
                }
            }
        }
    }

    // Final fallback: estimate from file size
    let metadata = fs::metadata(file_path)?;
    let estimated_pages = (metadata.len() / 10000).max(1) as usize; // Rough estimate
    println!("Warning: Could not determine exact page count, estimating {} pages", estimated_pages);
    Ok(estimated_pages)
}

fn process_pdf_chunk(
    file_path: &Arc<String>,
    temp_dir: &Arc<std::path::PathBuf>,
    start_page: usize,
    end_page: usize,
    chunk_idx: usize,
) -> Result<String, Box<StdError>> {
    println!("Processing chunk {} (pages {}-{})", chunk_idx, start_page, end_page);

    // Create chunk file path
    let chunk_file = temp_dir.join(format!("chunk_{}.pdf", chunk_idx));

    // Split PDF using pdftk (most reliable) or qpdf as fallback
    let success = split_pdf_chunk(file_path, &chunk_file, start_page, end_page)?;
    
    if !success {
        return Ok(String::new());
    }

    // Extract text from the chunk
    let chunk_text = pdf_extract::extract_text(&chunk_file)?;
    
    // Clean up the chunk file
    let _ = fs::remove_file(&chunk_file);
    
    println!("Completed chunk {} ({} characters)", chunk_idx, chunk_text.len());
    Ok(chunk_text)
}

fn split_pdf_chunk(
    source_file: &str,
    chunk_file: &std::path::Path,
    start_page: usize,
    end_page: usize,
) -> Result<bool, Box<StdError>> {
    // Try pdftk first
    let pdftk_result = Command::new("pdftk")
        .arg(source_file)
        .arg("cat")
        .arg(format!("{}-{}", start_page, end_page))
        .arg("output")
        .arg(chunk_file)
        .status();

    if pdftk_result.is_ok() && pdftk_result.unwrap().success() {
        return Ok(true);
    }

    // Fallback to qpdf
    let qpdf_result = Command::new("qpdf")
        .arg("--pages")
        .arg(source_file)
        .arg(format!("{}-{}", start_page, end_page))
        .arg("--")
        .arg(chunk_file)
        .status();

    if qpdf_result.is_ok() && qpdf_result.unwrap().success() {
        return Ok(true);
    }

    println!("Warning: Could not split PDF chunk {}-{}, skipping", start_page, end_page);
    Ok(false)
}

fn convert_docx_to_pdf(docx_path: &str) -> Result<String, Box<StdError>> {
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    let base_stem = Path::new(docx_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    
    let pdf_filename = format!("{}.pdf", base_stem);
    let pdf_path = pdfs_dir.join(&pdf_filename);

    if pdf_path.exists() {
        println!("Converted PDF already exists at {:?}, using existing file", pdf_path);
        return Ok(pdf_path.to_string_lossy().to_string());
    }

    println!("Converting DOCX to PDF: {}", docx_path);

    // Read the DOCX file using the correct docx-rs API
    let file_bytes = fs::read(docx_path)?;
    let _docx = Docx::new();
    
    // Parse the DOCX content - this is a simplified approach
    // For a more complete implementation, you might need to use a different approach
    // or consider using a different crate that better supports reading existing DOCX files
    
    // As a workaround, let's try to extract text directly without full parsing
    let content = extract_text_from_docx_bytes(&file_bytes)?;

    create_pdf_from_text_content(&pdf_path, &content)?;
    
    println!("Successfully converted DOCX to PDF: {:?}", pdf_path);
    Ok(pdf_path.to_string_lossy().to_string())
}

// Helper function to extract text from DOCX bytes
fn extract_text_from_docx_bytes(file_bytes: &[u8]) -> Result<Vec<String>, Box<StdError>> {
    // This is a simplified text extraction approach
    // For production use, you might want to use a more robust DOCX parsing library
    
    // Convert bytes to string and try to extract readable text
    // This is a basic approach - you might want to use zip crate to properly parse DOCX
    use std::io::Read;
    use std::io::Cursor;
    
    let cursor = Cursor::new(file_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to read DOCX as ZIP: {}", e))?;
    
    // Try to read the main document XML
    let mut document = archive.by_name("word/document.xml")
        .map_err(|e| format!("Failed to find document.xml: {}", e))?;
    
    let mut document_content = String::new();
    document.read_to_string(&mut document_content)
        .map_err(|e| format!("Failed to read document content: {}", e))?;
    
    // Basic XML text extraction (this is very simplified)
    let text = extract_text_from_xml(&document_content);
    
    Ok(vec![text])
}

fn extract_text_from_xml(xml_content: &str) -> String {
    // Very basic XML text extraction
    // This removes XML tags and extracts text content
    let mut result = String::new();
    let mut in_tag = false;
    
    for ch in xml_content.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    
    // Clean up the text
    result.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}



fn convert_xlsx_to_pdf(xlsx_path: &str) -> Result<String, Box<StdError>> {
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    let base_stem = Path::new(xlsx_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    
    let pdf_filename = format!("{}.pdf", base_stem);
    let pdf_path = pdfs_dir.join(&pdf_filename);

    if pdf_path.exists() {
        println!("Converted PDF already exists at {:?}, using existing file", pdf_path);
        return Ok(pdf_path.to_string_lossy().to_string());
    }

    println!("Converting XLSX to PDF: {}", xlsx_path);

    let mut workbook = open_workbook_auto(xlsx_path)
        .map_err(|e| format!("Failed to open XLSX: {}", e))?;
    
    let mut content = Vec::new();
    
    for sheet_name in workbook.sheet_names().to_owned() {
        if let Some(Ok(range)) = workbook.worksheet_range(&sheet_name) {
            content.push(format!("=== Sheet: {} ===", sheet_name));
            
            let sheet_text = convert_range_to_text(&range);
            content.extend(sheet_text);
            content.push("".to_string());
        }
    }

    create_pdf_from_text_content(&pdf_path, &content)?;
    
    println!("Successfully converted XLSX to PDF: {:?}", pdf_path);
    Ok(pdf_path.to_string_lossy().to_string())
}

fn convert_range_to_text(range: &Range<DataType>) -> Vec<String> {
    let mut lines = Vec::new();
    
    for row in range.rows() {
        let mut line = String::new();
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx > 0 {
                line.push_str(" | ");
            }
            
            let cell_str = match cell {
                DataType::String(s) => s.to_string(),
                DataType::Float(f) => f.to_string(),
                DataType::Int(i) => i.to_string(),
                DataType::Bool(b) => b.to_string(),
                DataType::DateTime(dt) => format!("{}", dt),
                DataType::DateTimeIso(dt) => dt.to_string(),
                DataType::Duration(d) => format!("{}", d),
                DataType::DurationIso(d) => d.to_string(),
                DataType::Error(e) => format!("ERROR: {:?}", e),
                DataType::Empty => "".to_string(),
            };
            line.push_str(&cell_str);
        }
        
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }
    
    lines
}


fn create_pdf_from_text_content(pdf_path: &Path, content: &[String]) -> Result<(), Box<StdError>> {
    let (doc, page1, layer1) = PdfDocument::new("Converted Document", Mm(210.0), Mm(297.0), "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to add font: {}", e))?;

    let font_size = 12.0;
    let line_height = 14.0;
    let margin = Mm(20.0);
    let page_width = Mm(210.0);
    let page_height = Mm(297.0);
    let text_width = page_width - (margin * 2.0);
    
    let mut current_y = page_height - margin;

    for line in content {
        if current_y < margin {
            let (new_page, new_layer) = doc.add_page(page_width, page_height, "Layer 1");
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            current_y = page_height - margin;
        }

        // Convert Mm to f32 by accessing the inner value
        let wrapped_lines = wrap_text(line, text_width.0, font_size);
        
        for wrapped_line in wrapped_lines {
            if current_y < margin {
                let (new_page, new_layer) = doc.add_page(page_width, page_height, "Layer 1");
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                current_y = page_height - margin;
            }

            current_layer.use_text(wrapped_line, font_size, margin, current_y, &font);
            current_y -= Mm(line_height);
        }
    }

    let mut file = BufWriter::new(std::fs::File::create(pdf_path)?);
    doc.save(&mut file)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(())
}


fn wrap_text(text: &str, max_width: f32, _font_size: f32) -> Vec<String> {
    let chars_per_line = (max_width / 6.0) as usize;
    
    if text.len() <= chars_per_line {
        vec![text.to_string()]
    } else {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > chars_per_line {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
}

pub async fn extract_file_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_file_text_sync(&file_path)).await?
}


fn extract_token_from_text(filepath: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let content = fs::read_to_string(filepath)?;
    
    // Look for hexadecimal strings that are 32+ characters (typical for tokens)
    let re = regex::Regex::new(r"[a-fA-F0-9]{32,}")?;
    
    if let Some(token) = re.find(&content) {
        Ok(token.as_str().to_string())
    } else {
        Err("No token found in the HTML content".into())
    }
}

