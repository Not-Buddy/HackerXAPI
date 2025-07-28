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
    use std::collections::HashSet;

    let doc = Document::load(file_path)?;

    // Ensure output dir
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    // Holds candidate headers and footers (strings)
    let mut candidate_headers = HashSet::new();
    let mut candidate_footers = HashSet::new();

    // Collect headers/footers from first few pages (say first 3 pages)
    let pages = doc.get_pages();
    for (_page_i, page_id) in pages.iter().take(3) {
        let content = doc.get_page_content(*page_id)?;
        let content = Content::decode(&content)?;
        let page_text = content.operations.iter()
            .filter_map(|op| {
    if op.operator == "Tj" || op.operator == "'" || op.operator == "\"" {
        op.operands.get(0).and_then(|operand| match operand {
            Object::String(bytes, _) => Some(clean_text(&String::from_utf8_lossy(bytes))),
            _ => None
        })
    } else if op.operator == "TJ" {
        op.operands.get(0).and_then(|operand| match operand {
            Object::Array(array) => {
                Some(array.iter().filter_map(|item| match item {
                    Object::String(bytes, _) => Some(clean_text(&String::from_utf8_lossy(bytes))),
                    _ => None
                }).collect::<Vec<String>>().join(""))
            }
            _ => None
        })
    } else {
        None
    }
})

            .collect::<Vec<String>>()
            .join("\n");

        let lines: Vec<_> = page_text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        // Add first 3 lines as headers if exist
        for header_line in lines.iter().take(3) {
            candidate_headers.insert((*header_line).to_string());
        }
        // Add last 3 lines as footers if exist
        for footer_line in lines.iter().rev().take(3) {
            candidate_footers.insert((*footer_line).to_string());
        }
    }

    let mut all_text = String::new();
    let mut previous_line_empty = false;

    for (_page_number, page_id) in pages {
        let content = doc.get_page_content(page_id)?;
        let content = Content::decode(&content)?;
        let page_text = content.operations.iter()
            .filter_map(|op| {
                if op.operator == "Tj" || op.operator == "'" || op.operator == "\"" {
                    op.operands.get(0).and_then(|operand| match operand {
                        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
                        _ => None
                    })
                } else if op.operator == "TJ" {
                    op.operands.get(0).and_then(|operand| match operand {
                        Object::Array(array) => {
                            Some(array.iter().filter_map(|item| match item {
                                Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
                                _ => None
                            }).collect::<Vec<String>>().join(""))
                        }
                        _ => None
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        // Split into trimmed lines and filter out header/footer
        for line in page_text.lines() {
            let line_trim = line.trim();
            if line_trim.is_empty() {
                // Avoid multiple consecutive blank lines
                if !previous_line_empty {
                    all_text.push('\n');
                    previous_line_empty = true;
                }
                continue;
            }

            if candidate_headers.contains(line_trim) || candidate_footers.contains(line_trim) {
                // Skip header/footer line
                continue;
            }

            all_text.push_str(line_trim);
            all_text.push('\n');
            previous_line_empty = false;
        }

        // Add a page break marker optionally (or just a blank line)
        all_text.push('\n'); 
        previous_line_empty = true;
    }

    // Write result file
    let txt_path = pdfs_dir.join("policy.txt");
    fs::write(&txt_path, &all_text)?;
    println!("Saved main extracted text (excluding headers/footers) to {:?}", txt_path);

    Ok(all_text)
}


pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}
