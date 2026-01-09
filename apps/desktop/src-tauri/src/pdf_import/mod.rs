//! PDF Bank Statement Import Module
//!
//! Parses PDF bank statements from various German banks and brokers.

pub mod dkb;
pub mod ing;
pub mod comdirect;
pub mod trade_republic;
pub mod scalable;

use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Parsed transaction from a PDF statement
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub warnings: Vec<String>,
    pub raw_text: Option<String>,
}

/// Bank parser trait
pub trait BankParser: Send + Sync {
    /// Check if this parser can handle the given PDF content
    fn detect(&self, content: &str) -> bool;

    /// Parse the PDF content into transactions
    fn parse(&self, content: &str) -> Result<Vec<ParsedTransaction>, String>;

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

/// Extract text from a PDF file
pub fn extract_pdf_text(pdf_path: &str) -> Result<String, String> {
    let bytes = std::fs::read(pdf_path)
        .map_err(|e| format!("Failed to read PDF file: {}", e))?;

    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract text from PDF: {}", e))
}

/// Parse a PDF file using auto-detection
pub fn parse_pdf(pdf_path: &str) -> Result<ParseResult, String> {
    let content = extract_pdf_text(pdf_path)?;
    parse_pdf_content(&content)
}

/// Parse PDF content (text already extracted)
pub fn parse_pdf_content(content: &str) -> Result<ParseResult, String> {
    let parsers = get_parsers();

    // Try to detect which bank
    for parser in &parsers {
        if parser.detect(content) {
            let transactions = parser.parse(content)?;
            return Ok(ParseResult {
                bank: parser.bank_name().to_string(),
                transactions,
                warnings: vec![],
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
pub fn extract_wkn(text: &str) -> Option<String> {
    let re = Regex::new(r"\b([A-Z0-9]{6})\b").ok()?;
    // Filter out ISINs
    for cap in re.captures_iter(text) {
        let wkn = &cap[1];
        // WKN should not look like part of an ISIN
        if !text.contains(&format!("{}{}", wkn, "")) {
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
}
