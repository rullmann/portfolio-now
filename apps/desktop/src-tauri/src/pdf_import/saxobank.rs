//! Saxo Bank PDF Parser
//!
//! Parses broker statements from Saxo Bank (Switzerland/Denmark).

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct SaxoBankParser {
    detect_patterns: Vec<&'static str>,
}

impl SaxoBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Saxo Bank",
                "saxobank",
                "8058 Zürich-Flughafen",
            ],
        }
    }

    /// Parse Saxo date format (05-Dez-2024 or 05-Dec-2024)
    fn parse_saxo_date(s: &str) -> Option<NaiveDate> {
        // Try DD-Mon-YYYY format
        let re = Regex::new(r"(\d{2})-([A-Za-z]{3})-(\d{4})").ok()?;
        let caps = re.captures(s)?;

        let day: u32 = caps[1].parse().ok()?;
        let month_str = caps[2].to_lowercase();
        let year: i32 = caps[3].parse().ok()?;

        let month = match month_str.as_str() {
            "jan" => 1, "feb" => 2, "mar" | "mär" => 3, "apr" => 4,
            "may" | "mai" => 5, "jun" => 6, "jul" => 7, "aug" => 8,
            "sep" => 9, "oct" | "okt" => 10, "nov" => 11, "dec" | "dez" => 12,
            _ => return None,
        };

        NaiveDate::from_ymd_opt(year, month, day)
    }

    /// Parse Swiss/Saxo number format (5'480.43 or 5.480,43)
    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace('\'', "")
            .replace(',', ".");
        // Handle negative with dash
        let cleaned = if cleaned.starts_with('-') {
            cleaned
        } else {
            cleaned.replace('-', "")
        };
        cleaned.parse::<f64>().ok().map(|v| v.abs())
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type - "K/V Kauf" or "K/V Verkauf"
        let type_re = Regex::new(r"K/V\s+(Kauf|Verkauf|Buy|Sell)").ok();
        let txn_type = type_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                match c[1].to_lowercase().as_str() {
                    "kauf" | "buy" => ParsedTransactionType::Buy,
                    _ => ParsedTransactionType::Sell,
                }
            });

        if txn_type.is_none() {
            return transactions;
        }

        let txn_type = txn_type.unwrap();

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract security name - "Instrument iShares Core MSCI World UCITS ETF"
        let name_re = Regex::new(r"Instrument\s+([^\n]+?)(?:\s+Handelszeit|\n)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Menge 49,00" or "Menge 49.00"
        let shares_re = Regex::new(r"Menge\s+([\d'.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        // Extract price - "Preis 111,8455 USD" or "Preis 111.8455"
        let price_re = Regex::new(r"Preis\s+([\d'.,]+)\s*([A-Z]{3})?").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        // Extract currency and amount - "Gehandelter Wert -5.480,43 USD"
        let value_re = Regex::new(r"Gehandelter Wert\s+-?([\d'.,]+)\s+([A-Z]{3})").ok();
        let (gross_amount, trade_currency) = value_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Self::parse_number(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "CHF".to_string()));

        // Extract fees - "Gesamte Trading-Kosten -19,41 CHF"
        let fees_re = Regex::new(r"(?:Gesamte\s+)?Trading-Kosten\s+-?([\d'.,]+)\s+([A-Z]{3})").ok();
        let (fees, fee_currency) = fees_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Self::parse_number(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "CHF".to_string()));

        // Extract date - "Handelszeit 05-Dez-2024"
        let date_re = Regex::new(r"(?:Handelszeit|Trade-Datum)\s+(\d{2}-[A-Za-z]{3}-\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_saxo_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Use the settlement currency (CHF typically) as final currency
        let currency = fee_currency;
        let net_amount = gross_amount + fees;

        if gross_amount > 0.0 || shares.is_some() {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type,
                security_name,
                isin,
                wkn: None,
                shares,
                price_per_share,
                gross_amount,
                fees,
                taxes: 0.0,
                net_amount,
                currency,
                note: Some("Saxo Bank Import".to_string()),
                exchange_rate: None,
                forex_currency: Some(trade_currency),
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Dividend") {
            return transactions;
        }

        let isin = extract_isin(content);

        let shares_re = Regex::new(r"(?:Menge|Anzahl)\s+([\d'.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        let gross_re = Regex::new(r"(?:Brutto|Gross)\s+([\d'.,]+)\s+([A-Z]{3})").ok();
        let (gross_amount, currency) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Self::parse_number(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "CHF".to_string()));

        let date_re = Regex::new(r"(?:Valuta|Value Date)\s+(\d{2}-[A-Za-z]{3}-\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_saxo_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        if gross_amount > 0.0 {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type: ParsedTransactionType::Dividend,
                security_name: None,
                isin,
                wkn: None,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency,
                note: Some("Saxo Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for SaxoBankParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns.iter().any(|p| content.contains(p))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();
        transactions.extend(self.parse_buy_sell(content, ctx));
        transactions.extend(self.parse_dividends(content, ctx));
        transactions.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Saxo Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = SaxoBankParser::new();
        assert!(parser.detect("Saxo Bank CH"));
        assert!(!parser.detect("UBS AG"));
    }

    #[test]
    fn test_parse_saxo_date() {
        assert_eq!(
            SaxoBankParser::parse_saxo_date("05-Dez-2024"),
            Some(NaiveDate::from_ymd_opt(2024, 12, 5).unwrap())
        );
        assert_eq!(
            SaxoBankParser::parse_saxo_date("08-Mar-2022"),
            Some(NaiveDate::from_ymd_opt(2022, 3, 8).unwrap())
        );
    }

    #[test]
    fn test_parse_buy() {
        let parser = SaxoBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Saxo Bank CH
K/V Kauf
Instrument iShares Core MSCI World UCITS ETF Handelszeit 05-Dez-2024 11:21:27
ISIN IE00B4L5Y983
Preis 111,8455 USD
Menge 49,00
Gehandelter Wert -5.480,43 USD
Gesamte Trading-Kosten -19,41 CHF
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("IE00B4L5Y983".to_string()));
    }
}
