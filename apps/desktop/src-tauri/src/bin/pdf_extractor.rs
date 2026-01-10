//! Standalone PDF text extractor binary.
//!
//! This is run as a subprocess to isolate pdf-extract crashes from the main application.
//! If pdf-extract panics (which can happen with malformed PDFs), only this process crashes,
//! not the main Tauri application.
//!
//! Usage: pdf_extractor <path_to_pdf>
//! Output: Extracted text on stdout, errors on stderr
//! Exit codes:
//!   0 - Success
//!   1 - Invalid arguments
//!   2 - PDF read error
//!   3 - PDF extraction error
//!   4 - PDF validation failed

use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

/// PDF magic bytes
const PDF_MAGIC: &[u8] = b"%PDF";
/// Maximum PDF file size (100 MB)
const MAX_PDF_SIZE: usize = 100 * 1024 * 1024;

fn validate_pdf(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 8 {
        return Err("Datei zu klein für eine gültige PDF".to_string());
    }

    if bytes.len() > MAX_PDF_SIZE {
        return Err(format!(
            "PDF-Datei zu groß ({} MB). Maximum: {} MB",
            bytes.len() / (1024 * 1024),
            MAX_PDF_SIZE / (1024 * 1024)
        ));
    }

    if !bytes.starts_with(PDF_MAGIC) {
        return Err("Ungültige PDF-Datei: PDF-Header fehlt".to_string());
    }

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: pdf_extractor <path_to_pdf>");
        return ExitCode::from(1);
    }

    let pdf_path = &args[1];

    // Read the PDF file
    let bytes = match fs::read(pdf_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("READ_ERROR:{}", e);
            return ExitCode::from(2);
        }
    };

    // Validate PDF structure
    if let Err(e) = validate_pdf(&bytes) {
        eprintln!("VALIDATE_ERROR:{}", e);
        return ExitCode::from(4);
    }

    // Extract text from PDF
    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => {
            // Write text to stdout
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = handle.write_all(text.as_bytes()) {
                eprintln!("WRITE_ERROR:{}", e);
                return ExitCode::from(3);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("EXTRACT_ERROR:{}", e);
            ExitCode::from(3)
        }
    }
}
