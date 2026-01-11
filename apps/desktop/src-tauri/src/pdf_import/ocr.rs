//! Vision API-based OCR for scanned PDFs
//!
//! Uses AI Vision models (Claude, GPT-4, Gemini) to extract text
//! from scanned PDF documents when regular text extraction fails.
//!
//! **Claude and Gemini support direct PDF upload** (no poppler needed!)
//! OpenAI/Perplexity still require image conversion via poppler.

use crate::ai::{claude, gemini, openai, perplexity, AiError};
use base64::{engine::general_purpose, Engine as _};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// OCR request options
#[derive(Debug, Clone)]
pub struct OcrOptions {
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

/// OCR result for a single page
#[derive(Debug, Clone)]
pub struct OcrPageResult {
    pub page_number: usize,
    pub text: String,
}

/// OCR result for entire document
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub pages: Vec<OcrPageResult>,
    pub full_text: String,
    pub provider: String,
    pub model: String,
}

/// OCR prompt for bank document extraction
const OCR_PROMPT: &str = r#"Extrahiere den vollständigen Text aus diesem Bankdokument.
Behalte die Struktur (Tabellen, Spalten) bei.
Gib nur den extrahierten Text zurück, keine Erklärungen.
Achte besonders auf:
- Datum (TT.MM.JJJJ Format)
- Beträge (mit Komma als Dezimaltrenner, z.B. 1.234,56)
- ISIN Nummern (12-stellig, beginnt mit 2 Buchstaben)
- WKN Nummern (6-stellig alphanumerisch)
- Transaktionstypen (Kauf, Verkauf, Dividende, etc.)
- Wertpapiernamen"#;

/// Check if provider supports direct PDF upload (no image conversion needed)
pub fn supports_direct_pdf(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "claude" | "gemini")
}

/// Check if pdftoppm (poppler-utils) is available
pub fn is_pdftoppm_available() -> bool {
    Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convert PDF to PNG images using pdftoppm
fn pdf_to_images(pdf_path: &str, output_dir: &Path) -> Result<Vec<String>, String> {
    let pdf_path = Path::new(pdf_path);
    let stem = pdf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");

    let output_prefix = output_dir.join(stem);

    // Run pdftoppm to convert PDF to PNG images
    let output = Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            "150", // 150 DPI is sufficient for OCR
            pdf_path.to_str().ok_or("Invalid PDF path")?,
            output_prefix.to_str().ok_or("Invalid output path")?,
        ])
        .output()
        .map_err(|e| format!("pdftoppm konnte nicht ausgeführt werden: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PDF-Konvertierung fehlgeschlagen: {}", stderr));
    }

    // Find all generated PNG files
    let mut image_paths: Vec<String> = std::fs::read_dir(output_dir)
        .map_err(|e| format!("Ausgabeverzeichnis konnte nicht gelesen werden: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "png")
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect();

    // Sort by filename to maintain page order
    image_paths.sort();

    if image_paths.is_empty() {
        return Err("Keine Bilder aus PDF extrahiert".to_string());
    }

    Ok(image_paths)
}

/// Read image file and convert to base64
fn image_to_base64(image_path: &str) -> Result<String, String> {
    let bytes = std::fs::read(image_path)
        .map_err(|e| format!("Bild konnte nicht gelesen werden: {}", e))?;

    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Read PDF file and convert to base64
fn pdf_to_base64(pdf_path: &str) -> Result<String, String> {
    let bytes = std::fs::read(pdf_path)
        .map_err(|e| format!("PDF konnte nicht gelesen werden: {}", e))?;

    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Perform OCR on a single image using Vision API
async fn ocr_image(
    image_base64: &str,
    options: &OcrOptions,
) -> Result<String, AiError> {
    // Use the analyze_with_custom_prompt function with OCR prompt
    let result = match options.provider.as_str() {
        "claude" => {
            claude::analyze_with_custom_prompt(
                image_base64,
                &options.model,
                &options.api_key,
                OCR_PROMPT,
            )
            .await
        }
        "openai" => {
            openai::analyze_with_custom_prompt(
                image_base64,
                &options.model,
                &options.api_key,
                OCR_PROMPT,
            )
            .await
        }
        "gemini" => {
            gemini::analyze_with_custom_prompt(
                image_base64,
                &options.model,
                &options.api_key,
                OCR_PROMPT,
            )
            .await
        }
        "perplexity" => {
            perplexity::analyze_with_custom_prompt(
                image_base64,
                &options.model,
                &options.api_key,
                OCR_PROMPT,
            )
            .await
        }
        _ => Err(AiError::other(
            &options.provider,
            &options.model,
            &format!("Unbekannter Anbieter: {}", options.provider),
        )),
    }?;

    Ok(result.analysis)
}

/// Perform OCR using direct PDF upload (Claude/Gemini only)
async fn ocr_pdf_direct(
    pdf_path: &str,
    options: &OcrOptions,
) -> Result<OcrResult, String> {
    log::info!("OCR: Using direct PDF upload for {}", options.provider);

    let pdf_base64 = pdf_to_base64(pdf_path)?;

    let result = match options.provider.as_str() {
        "claude" => {
            claude::ocr_pdf(&pdf_base64, &options.model, &options.api_key, OCR_PROMPT)
                .await
                .map_err(|e| e.message)?
        }
        "gemini" => {
            gemini::ocr_pdf(&pdf_base64, &options.model, &options.api_key, OCR_PROMPT)
                .await
                .map_err(|e| e.message)?
        }
        _ => return Err(format!(
            "Provider {} unterstützt keinen direkten PDF-Upload",
            options.provider
        )),
    };

    Ok(OcrResult {
        pages: vec![OcrPageResult {
            page_number: 1,
            text: result.analysis.clone(),
        }],
        full_text: result.analysis,
        provider: options.provider.clone(),
        model: options.model.clone(),
    })
}

/// Perform OCR using image conversion (requires poppler)
async fn ocr_pdf_via_images(
    pdf_path: &str,
    options: OcrOptions,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send>>,
) -> Result<OcrResult, String> {
    // Check if pdftoppm is available
    if !is_pdftoppm_available() {
        return Err(
            "OCR mit OpenAI/Perplexity benötigt poppler-utils. \
             Alternativen:\n\
             1. Wechsle zu Claude oder Gemini (unterstützt PDF direkt)\n\
             2. Installiere poppler:\n   \
                macOS: brew install poppler\n   \
                Ubuntu: sudo apt install poppler-utils\n   \
                Windows: choco install poppler"
                .to_string(),
        );
    }

    // Create temporary directory for images
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Temporäres Verzeichnis konnte nicht erstellt werden: {}", e))?;

    // Convert PDF to images
    log::info!("OCR: Converting PDF to images (using poppler)...");
    let image_paths = pdf_to_images(pdf_path, temp_dir.path())?;
    let total_pages = image_paths.len();
    log::info!("OCR: Converted {} pages", total_pages);

    // Process each page
    let mut pages = Vec::new();
    let mut all_text = String::new();

    for (i, image_path) in image_paths.iter().enumerate() {
        let page_num = i + 1;
        log::info!("OCR: Processing page {}/{}", page_num, total_pages);

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(page_num, total_pages);
        }

        // Read and encode image
        let image_base64 = image_to_base64(image_path)?;

        // Perform OCR
        let text = ocr_image(&image_base64, &options)
            .await
            .map_err(|e| format!("OCR für Seite {} fehlgeschlagen: {}", page_num, e.message))?;

        pages.push(OcrPageResult {
            page_number: page_num,
            text: text.clone(),
        });

        if !all_text.is_empty() {
            all_text.push_str("\n\n--- Seite ");
            all_text.push_str(&page_num.to_string());
            all_text.push_str(" ---\n\n");
        }
        all_text.push_str(&text);
    }

    Ok(OcrResult {
        pages,
        full_text: all_text,
        provider: options.provider,
        model: options.model,
    })
}

/// Perform OCR on a PDF file using Vision API
///
/// - Claude and Gemini: Send PDF directly (no poppler needed!)
/// - OpenAI and Perplexity: Convert to images first (requires poppler)
pub async fn ocr_pdf(
    pdf_path: &str,
    options: OcrOptions,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send>>,
) -> Result<OcrResult, String> {
    // Check if provider supports direct PDF upload
    if supports_direct_pdf(&options.provider) {
        // Direct PDF upload - no poppler needed!
        ocr_pdf_direct(pdf_path, &options).await
    } else {
        // Need to convert PDF to images first
        ocr_pdf_via_images(pdf_path, options, progress_callback).await
    }
}

/// Check if text extraction yielded too little content
/// Returns true if OCR fallback should be used
pub fn should_use_ocr_fallback(extracted_text: &str, min_chars: usize) -> bool {
    // Clean text: remove whitespace and common PDF artifacts
    let cleaned: String = extracted_text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{0}')
        .collect();

    cleaned.len() < min_chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_ocr_fallback() {
        // Very short text - should use OCR
        assert!(should_use_ocr_fallback("", 100));
        assert!(should_use_ocr_fallback("   \n\n\t  ", 100));
        assert!(should_use_ocr_fallback("ABC", 100));

        // Enough text - should not use OCR
        assert!(!should_use_ocr_fallback("A".repeat(200).as_str(), 100));

        // Just at threshold
        assert!(!should_use_ocr_fallback("A".repeat(100).as_str(), 100));
        assert!(should_use_ocr_fallback("A".repeat(99).as_str(), 100));
    }

    #[test]
    fn test_supports_direct_pdf() {
        assert!(supports_direct_pdf("claude"));
        assert!(supports_direct_pdf("Claude"));
        assert!(supports_direct_pdf("CLAUDE"));
        assert!(supports_direct_pdf("gemini"));
        assert!(supports_direct_pdf("Gemini"));
        assert!(!supports_direct_pdf("openai"));
        assert!(!supports_direct_pdf("perplexity"));
        assert!(!supports_direct_pdf("unknown"));
    }
}
