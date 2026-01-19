//! DEGIRO PDF Parser
//!
//! Parses broker statements from DEGIRO (flatexDEGIRO Bank).

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct DegiroParser {
    detect_patterns: Vec<&'static str>,
}

impl DegiroParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "DEGIRO",
                "degiro.de",
                "degiro.nl",
                "flatexDEGIRO",
                "Amstelplein 1",
            ],
        }
    }

    /// Parse DEGIRO date format (DD-MM-YYYY)
    fn parse_degiro_date(s: &str) -> Option<NaiveDate> {
        let parts: Vec<&str> = s.trim().split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let day: u32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let year: i32 = parts[2].parse().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)
    }

    /// Parse number with comma as decimal separator
    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace('.', "")  // Remove thousand separators
            .replace(',', "."); // Convert decimal separator
        cleaned.parse::<f64>().ok()
    }

    fn parse_kontoauszug(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Match buy/sell lines: "Kauf X zu je Y" or "Verkauf X zu je Y"
        let txn_re = Regex::new(
            r"(\d{2}-\d{2}-\d{4})\s+\d{2}:\d{2}\s+([^\n]+?)\s+([A-Z]{2}[A-Z0-9]{10})\s+(Kauf|Verkauf)\s+([\d.,]+)\s+zu je\s+([\d.,]+)\s+([A-Z]{3})\s+-?([\d.,]+)"
        ).ok();

        if let Some(re) = txn_re {
            for caps in re.captures_iter(content) {
                let date = Self::parse_degiro_date(&caps[1])
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
                let security_name = caps[2].trim().to_string();
                let isin = caps[3].to_string();
                let txn_type = if &caps[4] == "Kauf" {
                    ParsedTransactionType::Buy
                } else {
                    ParsedTransactionType::Sell
                };
                let shares = Self::parse_number(&caps[5]);
                let price = Self::parse_number(&caps[6]);
                let currency = caps[7].to_string();
                let amount = Self::parse_number(&caps[8]).unwrap_or(0.0);

                transactions.push(ParsedTransaction {
                    date,
                    time: None,
                    txn_type,
                    security_name: Some(security_name),
                    isin: Some(isin),
                    wkn: None,
                    shares,
                    price_per_share: price,
                    gross_amount: amount,
                    fees: 0.0, // Fees are separate lines in DEGIRO
                    taxes: 0.0,
                    net_amount: amount,
                    currency,
                    note: Some("DEGIRO Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                });
            }
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividend") && !content.contains("dividend") {
            return transactions;
        }

        let isin = extract_isin(content);

        // Extract security name
        let name_re = Regex::new(r"Security name:\s*([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract date
        let date_re = Regex::new(r"Dividend date \(Pay date\):\s*(\d{4}-\d{2}-\d{2})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| NaiveDate::parse_from_str(&c[1], "%Y-%m-%d").ok())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // DEGIRO dividend format: shares, div_per_share, gross, tax, net, currency
        // Example line: "20 0,142 2,84 -0,43 2,41 EUR"
        let data_re = Regex::new(
            r"(?m)^([\d.,]+)\s+([\d.,]+)\s+([\d.,]+)\s+-?([\d.,]+)\s+([\d.,]+)\s+([A-Z]{3})\s*$"
        ).ok();

        let (shares, gross_amount, taxes, net_amount, currency) = data_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                (
                    Self::parse_number(&c[1]),
                    Self::parse_number(&c[3]).unwrap_or(0.0),
                    Self::parse_number(&c[4]).unwrap_or(0.0),
                    Self::parse_number(&c[5]).unwrap_or(0.0),
                    c[6].to_string(),
                )
            })
            .unwrap_or((None, 0.0, 0.0, 0.0, "EUR".to_string()));

        if gross_amount > 0.0 || net_amount > 0.0 {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type: ParsedTransactionType::Dividend,
                security_name,
                isin,
                wkn: None,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes,
                net_amount,
                currency,
                note: Some("DEGIRO Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for DegiroParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns.iter().any(|p| content.contains(p))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();
        transactions.extend(self.parse_kontoauszug(content, ctx));
        transactions.extend(self.parse_dividends(content, ctx));
        transactions.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "DEGIRO"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = DegiroParser::new();
        assert!(parser.detect("DEGIRO B.V."));
        assert!(parser.detect("flatexDEGIRO Bank"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_degiro_date() {
        assert_eq!(
            DegiroParser::parse_degiro_date("03-08-2017"),
            Some(NaiveDate::from_ymd_opt(2017, 8, 3).unwrap())
        );
    }

    #[test]
    fn test_parse_dividend() {
        let parser = DegiroParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
DEGIRO B.V.
Dividend note
Security name: ROYAL DUTCH SHELLA
Security ISIN: GB00B03MLX29
Dividend date (Pay date): 2020-06-22
Number of shares Amount of dividend Gross amount of Amount of tax Net amount of
per share dividend withheld dividend Wäh.
20 0,142 2,84 -0,43 2,41 EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txns[0].isin, Some("GB00B03MLX29".to_string()));
    }
}
