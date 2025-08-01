use tokio::fs as async_fs;
pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}


fn clean_text(text: &str) -> String {
    // Remove control characters and non-printable characters except newlines, tabs, etc.
    text.chars()
        .filter(|c| {
            // Keep:
            // - printable characters (including letters, digits, punctuation)
            // - whitespace chars commonly expected: space, newline, tab
            // Exclude control chars and non-character Unicode points
            (*c == '\n') || (*c == '\t') || (*c == '\r') || !c.is_control()
        })
        .collect()
}


fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    use lopdf::{Document, Object, content::Content};
    use std::fs;
    use std::path::Path;

    let doc = Document::load(file_path)?;
    
    // Ensure output dir
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    let mut all_text = String::new();
    let pages = doc.get_pages();

    for (page_number, page_id) in pages {
        // Add page separator for readability
        if page_number > 1 {
            all_text.push_str("\n\n--- Page ");
            all_text.push_str(&page_number.to_string());
            all_text.push_str(" ---\n\n");
        }

        let content = doc.get_page_content(page_id)?;
        let content = Content::decode(&content)?;
        
        // Extract all text operations from the page
        let page_text = content.operations.iter()
            .filter_map(|op| {
                match op.operator.as_str() {
                    // Handle single text string operations
                    "Tj" | "'" | "\"" => {
                        op.operands.get(0).and_then(|operand| match operand {
                            Object::String(bytes, _) => {
                                String::from_utf8(bytes.clone()).ok()
                                    .map(|s| clean_text(&s))
                                    .filter(|s| !s.trim().is_empty())
                            },
                            _ => None
                        })
                    },
                    // Handle text array operations (for text with spacing adjustments)
                    "TJ" => {
                        op.operands.get(0).and_then(|operand| match operand {
                            Object::Array(array) => {
                                let text_parts: Vec<String> = array.iter()
                                    .filter_map(|item| match item {
                                        Object::String(bytes, _) => {
                                            String::from_utf8(bytes.clone()).ok()
                                                .map(|s| clean_text(&s))
                                                .filter(|s| !s.trim().is_empty())
                                        },
                                        _ => None
                                    })
                                    .collect();
                                
                                if text_parts.is_empty() {
                                    None
                                } else {
                                    Some(text_parts.join(""))
                                }
                            },
                            _ => None
                        })
                    },
                    _ => None
                }
            })
            .collect::<Vec<String>>();

        // Join all text from this page and add to overall text
        let page_content = page_text.join(" ");
        if !page_content.trim().is_empty() {
            // Add the page content with proper line breaks
            all_text.push_str(&page_content);
            all_text.push('\n');
        }
    }

    // Clean up excessive whitespace while preserving paragraph breaks
    let cleaned_text = all_text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");

    // Write result file
    let txt_path = pdfs_dir.join("policy.txt");
    fs::write(&txt_path, &cleaned_text)?;
    println!("Saved extracted text to {:?}", txt_path);

    Ok(cleaned_text)
}

pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}