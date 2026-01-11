//! PDF Bank Statement Import Module
//!
//! Parses PDF bank statements from various German banks and brokers.

pub mod dkb;
pub mod ing;
pub mod comdirect;
pub mod trade_republic;
pub mod scalable;
pub mod ocr;

use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Warning severity level for parsing issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    /// Informational, non-critical
    Info,
    /// Potentially incorrect data
    Warning,
    /// Critical field failed to parse
    Error,
}

/// Warning generated during PDF parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseWarning {
    pub field: String,
    pub raw_value: String,
    pub message: String,
    pub severity: WarningSeverity,
}

/// Context for collecting warnings during parsing
#[derive(Debug, Default)]
pub struct ParseContext {
    pub warnings: Vec<ParseWarning>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self { warnings: vec![] }
    }

    /// Add a warning
    pub fn warn(&mut self, field: &str, raw_value: &str, message: &str) {
        self.warnings.push(ParseWarning {
            field: field.to_string(),
            raw_value: raw_value.to_string(),
            message: message.to_string(),
            severity: WarningSeverity::Warning,
        });
    }

    /// Add an error-level warning
    pub fn error(&mut self, field: &str, raw_value: &str, message: &str) {
        self.warnings.push(ParseWarning {
            field: field.to_string(),
            raw_value: raw_value.to_string(),
            message: message.to_string(),
            severity: WarningSeverity::Error,
        });
    }

    /// Parse a German decimal with warning on failure
    pub fn parse_amount(&mut self, field: &str, raw: &str) -> f64 {
        match parse_german_decimal(raw) {
            Some(v) => v,
            None => {
                self.warn(field, raw, "Betrag konnte nicht geparst werden");
                0.0
            }
        }
    }

    /// Parse a required German decimal - returns None and logs error on failure
    pub fn parse_required_amount(&mut self, field: &str, raw: &str) -> Option<f64> {
        match parse_german_decimal(raw) {
            Some(v) => Some(v),
            None => {
                self.error(field, raw, "Pflichtfeld konnte nicht geparst werden");
                None
            }
        }
    }

    /// Parse German date with warning on failure
    pub fn parse_date(&mut self, field: &str, raw: &str, default: NaiveDate) -> NaiveDate {
        match parse_german_date(raw) {
            Some(d) => d,
            None => {
                self.warn(field, raw, "Datum konnte nicht geparst werden, verwende Standarddatum");
                default
            }
        }
    }

    /// Check if there are any error-level warnings
    pub fn has_errors(&self) -> bool {
        self.warnings.iter().any(|w| w.severity == WarningSeverity::Error)
    }

    /// Convert warnings to simple string list for backward compatibility
    pub fn warnings_as_strings(&self) -> Vec<String> {
        self.warnings.iter().map(|w| {
            format!("[{}] {}: {} (Wert: '{}')",
                match w.severity {
                    WarningSeverity::Info => "Info",
                    WarningSeverity::Warning => "Warnung",
                    WarningSeverity::Error => "Fehler",
                },
                w.field,
                w.message,
                w.raw_value
            )
        }).collect()
    }
}

/// PDF magic bytes
const PDF_MAGIC: &[u8] = b"%PDF";
/// Maximum PDF file size (100 MB)
const MAX_PDF_SIZE: usize = 100 * 1024 * 1024;

/// Parsed transaction from a PDF statement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTransaction {
    pub date: NaiveDate,
    pub txn_type: ParsedTransactionType,
    pub security_name: Option<String>,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub shares: Option<f64>,
    pub price_per_share: Option<f64>,
    pub gross_amount: f64,
    pub fees: f64,
    pub taxes: f64,
    pub net_amount: f64,
    pub currency: String,
    pub note: Option<String>,
    pub exchange_rate: Option<f64>,
    pub forex_currency: Option<String>,
}

/// Transaction type parsed from PDF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParsedTransactionType {
    Buy,
    Sell,
    Dividend,
    Interest,
    Deposit,
    Withdrawal,
    Fee,
    TaxRefund,
    StockSplit,
    TransferIn,
    TransferOut,
    Unknown,
}

impl ParsedTransactionType {
    pub fn to_account_type(&self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::Dividend => "DIVIDENDS",
            Self::Interest => "INTEREST",
            Self::Deposit => "DEPOSIT",
            Self::Withdrawal => "REMOVAL",
            Self::Fee => "FEES",
            Self::TaxRefund => "TAX_REFUND",
            Self::TransferIn => "TRANSFER_IN",
            Self::TransferOut => "TRANSFER_OUT",
            _ => "DEPOSIT",
        }
    }

    pub fn to_portfolio_type(&self) -> Option<&'static str> {
        match self {
            Self::Buy => Some("BUY"),
            Self::Sell => Some("SELL"),
            Self::TransferIn => Some("TRANSFER_IN"),
            Self::TransferOut => Some("TRANSFER_OUT"),
            _ => None,
        }
    }
}

/// Result of parsing a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub bank: String,
    pub transactions: Vec<ParsedTransaction>,
    pub warnings: Vec<ParseWarning>,
    pub raw_text: Option<String>,
}

/// Bank parser trait
pub trait BankParser: Send + Sync {
    /// Check if this parser can handle the given PDF content
    fn detect(&self, content: &str) -> bool;

    /// Parse the PDF content into transactions
    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String>;

    /// Get the bank name
    fn bank_name(&self) -> &'static str;
}

/// All available bank parsers
pub fn get_parsers() -> Vec<Box<dyn BankParser>> {
    vec![
        Box::new(dkb::DkbParser::new()),
        Box::new(ing::IngParser::new()),
        Box::new(comdirect::ComdirectParser::new()),
        Box::new(trade_republic::TradeRepublicParser::new()),
        Box::new(scalable::ScalableParser::new()),
    ]
}

/// Validate PDF file before extraction
fn validate_pdf(bytes: &[u8]) -> Result<(), String> {
    // Check minimum size for a valid PDF
    if bytes.len() < 8 {
        return Err("Datei zu klein für eine gültige PDF".to_string());
    }

    // Check file size limit
    if bytes.len() > MAX_PDF_SIZE {
        return Err(format!(
            "PDF-Datei zu groß ({} MB). Maximum: {} MB",
            bytes.len() / (1024 * 1024),
            MAX_PDF_SIZE / (1024 * 1024)
        ));
    }

    // Check PDF magic number
    if !bytes.starts_with(PDF_MAGIC) {
        return Err("Ungültige PDF-Datei: PDF-Header fehlt".to_string());
    }

    Ok(())
}

/// Find the pdf_extractor binary path
fn find_pdf_extractor() -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Konnte ausführbare Datei nicht finden: {}", e))?;

    let exe_dir = exe_path.parent()
        .ok_or("Konnte Verzeichnis der ausführbaren Datei nicht ermitteln")?;

    // Try various locations for the pdf_extractor binary
    let candidates = [
        exe_dir.join("pdf_extractor"),
        exe_dir.join("pdf_extractor.exe"),
        exe_dir.join("../pdf_extractor"),
        exe_dir.join("../pdf_extractor.exe"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            log::info!("PDF Extract: Found extractor at {:?}", candidate);
            return Ok(candidate.clone());
        }
    }

    Err("pdf_extractor Binary nicht gefunden. Bitte stellen Sie sicher, dass es kompiliert wurde.".to_string())
}

/// Extract text from a PDF file using subprocess isolation for crash safety
pub fn extract_pdf_text(pdf_path: &str) -> Result<String, String> {
    log::info!("PDF Extract: Processing file {}", pdf_path);

    let bytes = std::fs::read(pdf_path)
        .map_err(|e| format!("PDF konnte nicht gelesen werden: {}", e))?;

    log::info!("PDF Extract: File read, {} bytes", bytes.len());
    validate_pdf(&bytes)?;
    drop(bytes);

    log::info!("PDF Extract: Validation passed, starting subprocess extraction");

    let extractor_path = find_pdf_extractor()?;

    let output = Command::new(&extractor_path)
        .arg(pdf_path)
        .output()
        .map_err(|e| format!("PDF-Extractor konnte nicht gestartet werden: {}", e))?;

    match output.status.code() {
        Some(0) => {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            log::info!("PDF Extract: Success, extracted {} chars", text.len());
            Ok(text)
        }
        Some(code) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("PDF Extract: Exit code {}, stderr: {}", code, stderr);
            Err(format!("PDF-Extraktion fehlgeschlagen (Code {}): {}", code, stderr.trim()))
        }
        None => {
            log::error!("PDF Extract: Process was killed");
            Err("PDF-Verarbeitung abgebrochen".to_string())
        }
    }
}

/// Parse a PDF file using auto-detection
pub fn parse_pdf(pdf_path: &str) -> Result<ParseResult, String> {
    let content = extract_pdf_text(pdf_path)?;
    parse_pdf_content(&content)
}

/// Parse PDF content (text already extracted)
pub fn parse_pdf_content(content: &str) -> Result<ParseResult, String> {
    let parsers = get_parsers();
    let mut ctx = ParseContext::new();

    // Try to detect which bank
    for parser in &parsers {
        if parser.detect(content) {
            let transactions = parser.parse(content, &mut ctx)?;
            return Ok(ParseResult {
                bank: parser.bank_name().to_string(),
                transactions,
                warnings: ctx.warnings,
                raw_text: Some(content.to_string()),
            });
        }
    }

    Err("Could not detect bank from PDF content. Supported banks: DKB, ING, Comdirect, Trade Republic, Scalable Capital".to_string())
}

/// Parse a German decimal number (1.234,56 -> 1234.56)
pub fn parse_german_decimal(s: &str) -> Option<f64> {
    let cleaned = s
        .trim()
        .replace('.', "")  // Remove thousand separators
        .replace(',', "."); // Convert decimal separator

    cleaned.parse::<f64>().ok()
}

/// Parse a German date (DD.MM.YYYY -> NaiveDate)
pub fn parse_german_date(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let day: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let year: i32 = parts[2].parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

/// Extract ISIN from text (12 chars, starts with 2 letters)
pub fn extract_isin(text: &str) -> Option<String> {
    let re = Regex::new(r"\b([A-Z]{2}[A-Z0-9]{10})\b").ok()?;
    re.captures(text).map(|c| c[1].to_string())
}

/// Extract WKN from text (6 alphanumeric chars)
/// Excludes matches that are substrings of ISINs
pub fn extract_wkn(text: &str) -> Option<String> {
    let wkn_re = Regex::new(r"\b([A-Z0-9]{6})\b").ok()?;
    let isin_re = Regex::new(r"\b[A-Z]{2}[A-Z0-9]{10}\b").ok()?;

    // Collect all ISINs in the text
    let isins: Vec<&str> = isin_re.find_iter(text).map(|m| m.as_str()).collect();

    // Find WKN that is NOT a substring of any ISIN
    for cap in wkn_re.captures_iter(text) {
        let wkn = &cap[1];
        let is_part_of_isin = isins.iter().any(|isin| isin.contains(wkn));
        if !is_part_of_isin {
            return Some(wkn.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_german_decimal() {
        assert_eq!(parse_german_decimal("1.234,56"), Some(1234.56));
        assert_eq!(parse_german_decimal("1234,56"), Some(1234.56));
        assert_eq!(parse_german_decimal("-123,45"), Some(-123.45));
        assert_eq!(parse_german_decimal("0,01"), Some(0.01));
    }

    #[test]
    fn test_parse_german_date() {
        assert_eq!(
            parse_german_date("15.03.2024"),
            Some(NaiveDate::from_ymd_opt(2024, 3, 15).unwrap())
        );
        assert_eq!(
            parse_german_date("01.01.2020"),
            Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        );
    }

    #[test]
    fn test_extract_isin() {
        assert_eq!(extract_isin("ISIN: DE0005140008"), Some("DE0005140008".to_string()));
        assert_eq!(extract_isin("US0378331005 Apple Inc"), Some("US0378331005".to_string()));
    }

    #[test]
    fn test_extract_wkn() {
        // WKN standalone
        assert_eq!(extract_wkn("WKN: A1JX52"), Some("A1JX52".to_string()));
        assert_eq!(extract_wkn("WKN 514000 Siemens AG"), Some("514000".to_string()));

        // WKN should NOT match when it's part of an ISIN
        // DE000A1JX520 contains A1JX52 at positions 5-10
        assert_eq!(extract_wkn("ISIN: DE000A1JX520"), None);

        // WKN alongside ISIN - but note: German ISINs contain the WKN!
        // DE0005140008 contains 514000, so this WKN won't be extracted
        // Use a case where WKN is NOT in the ISIN
        assert_eq!(
            extract_wkn("ISIN: US0378331005 WKN: A0M4W9"),
            Some("A0M4W9".to_string())
        );

        // No WKN present (only lowercase or wrong length)
        assert_eq!(extract_wkn("no wkn here"), None);
        assert_eq!(extract_wkn("ABC12"), None); // Too short
    }
}
