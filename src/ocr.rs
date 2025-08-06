use std::path::Path;
use std::fs;
use crate::pdf::StdError;

pub fn extract_text_from_pptx(pptx_path: &str) -> Result<String, Box<StdError>> {
    println!("Processing PPTX file: {}", pptx_path);
    
    // Create directory for extracted images
    let images_dir = Path::new("temp_pptx_images");
    if images_dir.exists() {
        fs::remove_dir_all(images_dir)?;
    }
    fs::create_dir_all(images_dir)?;
    
    // Extract all pages from PPTX as images
    let image_paths = extract_pptx_pages_as_images(pptx_path, images_dir)?;
    
    // Apply OCR to each extracted image using ocrs CLI
    let mut all_text = Vec::new();
    
    for (page_num, image_path) in image_paths.iter().enumerate() {
        println!("Processing PPTX page {}: {}", page_num + 1, image_path);
        
        // Extract text from this page image using ocrs CLI tool
        match extract_text_with_ocrs(image_path) {
            Ok(page_text) => {
                if !page_text.trim().is_empty() {
                    all_text.push(format!("=== Slide {} ===\n{}", page_num + 1, page_text));
                }
            }
            Err(e) => {
                println!("Warning: Failed to extract text from slide {}: {}", page_num + 1, e);
                // Continue processing other slides even if one fails
            }
        }
    }
    
    // Clean up temporary images
    let _ = fs::remove_dir_all(images_dir);
    
    // Combine all slide text
    let combined_text = all_text.join("\n\n");
    
    if combined_text.trim().is_empty() {
        return Err("No text could be extracted from the PPTX file".into());
    }
    
    // Ensure pdfs directory exists
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)
            .map_err(|e| format!("Failed to create pdfs directory: {}", e))?;
    }
    
    // Extract filename from pptx_path without extension
    let pptx_filename = Path::new(pptx_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pptx_output");
    
    let txt_filename = format!("{}.txt", pptx_filename);
    let txt_path = pdfs_dir.join(&txt_filename);
    
    // Write combined text to the file
    fs::write(&txt_path, &combined_text)
        .map_err(|e| format!("Failed to write PPTX text to file {:?}: {}", txt_path, e))?;
    
    println!("PPTX extracted text saved to: {:?}", txt_path);
    println!("Successfully extracted text from {} slides", all_text.len());
    
    Ok(combined_text)
}


// New function to use ocrs CLI tool
pub fn extract_text_with_ocrs(image_path: &str) -> Result<String, Box<StdError>> {
    println!("Running OCR on image: {}", image_path);
    
    let output = std::process::Command::new("ocrs")
        .arg(image_path)
        .output()
        .map_err(|e| format!("Failed to execute ocrs command: {}. Make sure ocrs is installed.", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ocrs command failed: {}", stderr).into());
    }
    
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Ensure pdfs directory exists
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)
            .map_err(|e| format!("Failed to create pdfs directory: {}", e))?;
    }
    
    // Extract filename from image path and create .txt filename
    let image_filename = Path::new(image_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ocr_output");
    
    let txt_filename = format!("{}.txt", image_filename);
    let txt_path = pdfs_dir.join(&txt_filename);
    
    // Write extracted text to file
    fs::write(&txt_path, &text)
        .map_err(|e| format!("Failed to write OCR text to file {:?}: {}", txt_path, e))?;
    
    println!("OCR text saved to: {:?}", txt_path);
    
    Ok(text)
}



fn extract_pptx_pages_as_images(pptx_path: &str, output_dir: &Path) -> Result<Vec<String>, Box<StdError>> {
    println!("Extracting PPTX pages as images using ImageMagick...");
    
    // Use 'convert' command for ImageMagick v6
    let status = std::process::Command::new("convert")
        .arg("-density")
        .arg("150")  // 150 DPI for good OCR quality while being faster
        .arg("-background")
        .arg("white")
        .arg("-alpha")
        .arg("remove")
        .arg("-quality")
        .arg("85")   // Good quality, faster processing
        .arg(pptx_path)
        .arg(output_dir.join("slide-%02d.png").to_str().unwrap())
        .status()
        .map_err(|e| format!("Failed to execute convert command: {}", e))?;

    if !status.success() {
        println!("ImageMagick direct conversion failed, falling back to LibreOffice method");
        // Fallback to your original method
        return fallback_pptx_conversion(pptx_path, output_dir);
    }

    println!("ImageMagick conversion successful");
    collect_image_files(output_dir)
}

// Add these helper functions
fn fallback_pptx_conversion(pptx_path: &str, output_dir: &Path) -> Result<Vec<String>, Box<StdError>> {
    println!("Using fallback: LibreOffice -> PDF -> Images");
    let pdf_path = convert_pptx_to_pdf_for_images(pptx_path)?;
    convert_pdf_pages_to_images(&pdf_path, output_dir)
}

fn collect_image_files(output_dir: &Path) -> Result<Vec<String>, Box<StdError>> {
    let mut image_paths = Vec::new();
    let entries = fs::read_dir(output_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            image_paths.push(path.to_string_lossy().to_string());
        }
    }
    
    image_paths.sort();
    Ok(image_paths)
}




pub fn convert_pptx_to_pdf_for_images(pptx_path: &str) -> Result<String, Box<StdError>> {
    let temp_dir = Path::new("temp_pptx_pdf");
    if !temp_dir.exists() {
        fs::create_dir_all(temp_dir)?;
    }
    
    let base_name = Path::new(pptx_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    
    let pdf_path = temp_dir.join(format!("{}.pdf", base_name));
    
    // Use LibreOffice to convert PPTX to PDF
    let status = std::process::Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(temp_dir)
        .arg(pptx_path)
        .status()
        .map_err(|e| format!("Failed to execute LibreOffice: {}. Make sure LibreOffice is installed.", e))?;
    
    if !status.success() {
        return Err("LibreOffice PPTX to PDF conversion failed".into());
    }
    
    Ok(pdf_path.to_string_lossy().to_string())
}

pub fn convert_pdf_pages_to_images(pdf_path: &str, output_dir: &Path) -> Result<Vec<String>, Box<StdError>> {
    // Use pdftoppm to convert PDF pages to images
    let status = std::process::Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("300") // 300 DPI for good OCR quality
        .arg(pdf_path)
        .arg(output_dir.join("slide").to_str().unwrap())
        .status()
        .map_err(|e| format!("Failed to execute pdftoppm: {}. Make sure poppler-utils is installed.", e))?;
    
    if !status.success() {
        return Err("PDF to images conversion failed".into());
    }
    
    // Collect all generated image files
    let mut image_paths = Vec::new();
    let entries = fs::read_dir(output_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            image_paths.push(path.to_string_lossy().to_string());
        }
    }
    
    // Sort by filename to maintain slide order
    image_paths.sort();
    
    Ok(image_paths)
}
