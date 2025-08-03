# HackerXAPI
Install Rust First

On linux its
``
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
``

Then run 
``
cargo run
``
Temp README will make it more brief later


System dependcies on java


sudo apt-get install pdftk-java qpdf poppler-utils libglib2.0-dev libcairo2-dev libpoppler-glib-dev




Database schema for mysql


CREATE TABLE pdf_embeddings (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    pdf_filename VARCHAR(255) NOT NULL,
    chunk_text TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    embedding JSON NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_pdf_filename (pdf_filename),
    INDEX idx_chunk_index (chunk_index)
);



To run the test.sh script u will need to install miniserve using the command


"cargo install miniserve"