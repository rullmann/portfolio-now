//! Vision API-based OCR for scanned PDFs
//!
//! Uses AI Vision models (Claude, GPT-4, Gemini) to extract text
//! from scanned PDF documents when regular text extraction fails.
//!
//! **Claude and Gemini support direct PDF upload** (no conversion needed!)
//! OpenAI/Perplexity require poppler (pdftoppm) for PDF-to-image conversion.

use crate::ai::{claude, gemini, openai, perplexity, AiError};
use base64::{engine::general_purpose, Engine as _};
use std::process::Command;

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

/// Check if pdftoppm (poppler) is available for PDF-to-image conversion
pub fn is_pdftoppm_available() -> bool {
    Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convert PDF to PNG images using pdftoppm (poppler)
fn pdf_to_images(pdf_path: &str) -> Result<Vec<Vec<u8>>, String> {
    let temp_dir = std::env::temp_dir().join(format!("ocr_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Temporäres Verzeichnis konnte nicht erstellt werden: {}", e))?;

    let output_prefix = temp_dir.join("page");

    // Run pdftoppm to convert PDF to PNG images
    let output = Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            "150", // 150 DPI - good balance between quality and size
            pdf_path,
            output_prefix.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("pdftoppm konnte nicht ausgeführt werden: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftoppm fehlgeschlagen: {}", stderr));
    }

    // Collect all generated PNG files
    let mut images = Vec::new();
    let mut page_num = 1;

    loop {
        // pdftoppm generates files like page-1.png, page-2.png, etc.
        let png_path = temp_dir.join(format!("page-{}.png", page_num));
        if !png_path.exists() {
            // Also try page-01.png format for documents with >9 pages
            let png_path_padded = temp_dir.join(format!("page-{:02}.png", page_num));
            if !png_path_padded.exists() {
                break;
            }
            let bytes = std::fs::read(&png_path_padded)
                .map_err(|e| format!("PNG konnte nicht gelesen werden: {}", e))?;
            images.push(bytes);
        } else {
            let bytes = std::fs::read(&png_path)
                .map_err(|e| format!("PNG konnte nicht gelesen werden: {}", e))?;
            images.push(bytes);
        }
        page_num += 1;
    }

    // Cleanup temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    if images.is_empty() {
        return Err("Keine Seiten im PDF gefunden".to_string());
    }

    Ok(images)
}

/// Convert image bytes to base64
fn image_to_base64(image_bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(image_bytes)
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

/// Perform OCR using pdftoppm image conversion (OpenAI/Perplexity)
async fn ocr_pdf_via_images(
    pdf_path: &str,
    options: OcrOptions,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send>>,
) -> Result<OcrResult, String> {
    // Check if pdftoppm is available
    if !is_pdftoppm_available() {
        return Err(
            "pdftoppm (poppler) nicht gefunden.\n\
             Bitte installiere poppler:\n\n\
             macOS: brew install poppler\n\
             Ubuntu/Debian: sudo apt install poppler-utils\n\
             Windows: choco install poppler\n\n\
             Alternativ: Wechsle zu Claude oder Gemini (unterstützt PDF direkt)"
                .to_string(),
        );
    }

    // Convert PDF to images using pdftoppm
    log::info!("OCR: Converting PDF to images using pdftoppm...");
    let image_bytes_list = pdf_to_images(pdf_path)?;
    let total_pages = image_bytes_list.len();
    log::info!("OCR: Converted {} pages", total_pages);

    // Process each page
    let mut pages = Vec::new();
    let mut all_text = String::new();

    for (i, image_bytes) in image_bytes_list.iter().enumerate() {
        let page_num = i + 1;
        log::info!("OCR: Processing page {}/{}", page_num, total_pages);

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(page_num, total_pages);
        }

        // Convert to base64
        let image_base64 = image_to_base64(image_bytes);

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
/// - Claude and Gemini: Send PDF directly (no conversion needed!)
/// - OpenAI and Perplexity: Convert to images using pdftoppm (requires poppler)
pub async fn ocr_pdf(
    pdf_path: &str,
    options: OcrOptions,
    progress_callback: Option<Box<dyn Fn(usize, usize) + Send>>,
) -> Result<OcrResult, String> {
    // Check if provider supports direct PDF upload
    if supports_direct_pdf(&options.provider) {
        // Direct PDF upload - no conversion needed!
        ocr_pdf_direct(pdf_path, &options).await
    } else {
        // Need to convert PDF to images first using pdftoppm
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
