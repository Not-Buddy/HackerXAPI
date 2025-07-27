use tokio::fs as async_fs;
use std::fs;
use std::path::Path;

pub type StdError = dyn std::error::Error + Send + Sync + 'static;

pub async fn download_pdf(url: &str, file_path: &str) -> Result<(), Box<StdError>> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    async_fs::write(file_path, &bytes).await?;
    Ok(())
}

fn extract_pdf_text_sync(file_path: &str) -> Result<String, Box<StdError>> {
    use lopdf::{Document, Object, content::Content};
    use std::fs;
    use std::path::Path;

    let doc = Document::load(file_path)?;

    // Ensure the output folder exists
    let pdfs_dir = Path::new("pdfs");
    if !pdfs_dir.exists() {
        fs::create_dir_all(pdfs_dir)?;
    }

    let mut all_text = String::new();

    // get_pages() returns BTreeMap<u32, ObjectId>
    for (page_number, page_id) in doc.get_pages() {
        // Get Content for the page
        let content = doc.get_page_content(page_id)?;
        // Decode PDF instructions/content
        let content = Content::decode(&content)?;
        // Extract text from content operations
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

        // Separate pages by newlines
        all_text.push_str(&format!("==== Page {} ====\n", page_number));
        all_text.push_str(&page_text);
        all_text.push_str("\n\n");
    }

    // Write all extracted text to pdfs/policy.txt
    let txt_path = pdfs_dir.join("policy.txt");
    fs::write(&txt_path, &all_text)?;
    println!("Saved all extracted text to {:?}", txt_path);

    Ok(all_text)
}


pub async fn extract_pdf_text(file_path: &str) -> Result<String, Box<StdError>> {
    let file_path = file_path.to_owned();
    tokio::task::spawn_blocking(move || extract_pdf_text_sync(&file_path)).await?
}

/// Utility function to delete file - you can just import std::fs::remove_file where needed,
/// but it's fine to add here if you want.
pub fn delete_file(path: &str) -> std::io::Result<()> {
    if Path::new(path).exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
