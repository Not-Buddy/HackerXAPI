use tokio::fs as async_fs;
use rayon::prelude::*;
use std::sync::Arc;
use std::process::Command;
use std::fs;
use std::path::Path;


pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}

fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    // Ensure output dir
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    // Create temp directory for PDF chunks
    let temp_dir = pdfs_dir.join("temp_chunks");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Get total number of pages using pdftk or similar tool
    let total_pages = get_pdf_page_count_accurate(file_path)?;
    let pages_per_chunk = (total_pages + 7) / 8; // Ceiling division for 8 chunks
    
    println!("Total pages: {}, pages per chunk: {}", total_pages, pages_per_chunk);

    // Create page ranges for 8 chunks
    let page_ranges: Vec<(usize, usize)> = (0..8)
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

    // Generate output filename based on input PDF filename
    let pdf_filename = Path::new(&**file_path)
    .file_stem()
    .and_then(|name| name.to_str())
    .unwrap_or("document");
    
    let txt_filename = format!("{}.txt", pdf_filename);
    let txt_path = pdfs_dir.join(&txt_filename);
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

pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}
