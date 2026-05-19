pub fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    text.chars()
        .collect::<Vec<char>>()
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect::<String>())
        .filter(|chunk| !chunk.trim().is_empty())
        .collect()
}
