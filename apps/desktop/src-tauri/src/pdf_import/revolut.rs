//! Revolut Trading PDF Parser
//!
//! Parses broker statements from Revolut Trading Ltd.

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct RevolutParser {
    detect_patterns: Vec<&'static str>,
}

impl RevolutParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Revolut Trading",
                "Revolut Ltd",
                "DriveWealth",
                "7 Westferry Circus",
                "Canary Wharf",
            ],
        }
    }

    /// Parse English date format (DD MMM YYYY or YYYY-MM-DD)
    fn parse_date(s: &str) -> Option<NaiveDate> {
        // Try DD MMM YYYY format (01 Nov 2021)
        let months = [
            ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
            ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
            ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
        ];

        let re = Regex::new(r"(\d{2})\s+([A-Za-z]{3})\s+(\d{4})").ok()?;
        if let Some(caps) = re.captures(s) {
            let day: u32 = caps[1].parse().ok()?;
            let month_str = &caps[2];
            let year: i32 = caps[3].parse().ok()?;

            for (name, num) in months.iter() {
                if month_str.eq_ignore_ascii_case(name) {
                    return NaiveDate::from_ymd_opt(year, *num, day);
                }
            }
        }

        // Try YYYY-MM-DD format
        NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
    }

    /// Parse USD amount ($1,166.12 -> 1166.12)
    fn parse_usd_amount(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace('$', "")
            .replace(',', "");
        cleaned.parse::<f64>().ok()
    }

    fn parse_trade_confirmation(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Trade Confirmation") {
            return transactions;
        }

        // Extract trade details from table
        // Symbol Company ISIN Type Quantity Price Settlement date
        let trade_re = Regex::new(
            r"([A-Z]+)\s+([^\n]+?)\s+([A-Z]{2}[A-Z0-9]{10})\s+(Buy|Sell)\s+([\d.]+)\s+\$([\d.,]+)\s+(\d{2}\s+\w{3}\s+\d{4})"
        ).ok();

        if let Some(re) = trade_re {
            for caps in re.captures_iter(content) {
                let symbol = caps[1].trim();
                let company = caps[2].trim().to_string();
                let isin = caps[3].to_string();
                let txn_type = if &caps[4] == "Buy" {
                    ParsedTransactionType::Buy
                } else {
                    ParsedTransactionType::Sell
                };
                let shares = caps[5].parse::<f64>().ok();
                let price = Self::parse_usd_amount(&caps[6]);
                let date = Self::parse_date(&caps[7])
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

                let gross_amount = match (shares, price) {
                    (Some(s), Some(p)) => s * p,
                    _ => 0.0,
                };

                // Extract fees from "Total Fee charged"
                let fee_re = Regex::new(r"Total Fee charged\s+\$([\d.,]+)").ok();
                let fees = fee_re
                    .as_ref()
                    .and_then(|re| re.captures(content))
                    .and_then(|c| Self::parse_usd_amount(&c[1]))
                    .unwrap_or(0.0);

                transactions.push(ParsedTransaction {
                    date,
                    time: None,
                    txn_type,
                    security_name: Some(format!("{} ({})", company, symbol)),
                    isin: Some(isin),
                    wkn: None,
                    shares,
                    price_per_share: price,
                    gross_amount,
                    fees,
                    taxes: 0.0,
                    net_amount: gross_amount + fees,
                    currency: "USD".to_string(),
                    note: Some("Revolut Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                });
            }
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividend") {
            return transactions;
        }

        let isin = extract_isin(content);

        // Look for dividend information
        let amount_re = Regex::new(r"(?:Net dividend|Dividend amount)[:\s]+\$([\d.,]+)").ok();
        let gross_amount = amount_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_usd_amount(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"(?:Pay date|Payment date)[:\s]+(\d{2}\s+\w{3}\s+\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        if gross_amount > 0.0 {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type: ParsedTransactionType::Dividend,
                security_name: None,
                isin,
                wkn: None,
                shares: None,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency: "USD".to_string(),
                note: Some("Revolut Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for RevolutParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns.iter().any(|p| content.contains(p))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();
        transactions.extend(self.parse_trade_confirmation(content, ctx));
        transactions.extend(self.parse_dividends(content, ctx));
        transactions.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Revolut"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = RevolutParser::new();
        assert!(parser.detect("Revolut Trading Ltd"));
        assert!(parser.detect("DriveWealth"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(
            RevolutParser::parse_date("01 Nov 2021"),
            Some(NaiveDate::from_ymd_opt(2021, 11, 1).unwrap())
        );
    }

    #[test]
    fn test_parse_sell() {
        let parser = RevolutParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Trade Confirmation
Revolut Trading Ltd
Symbol Company ISIN Type Quantity Price Settlement date
TSLA Tesla US88160R1014 Sell 2.1451261 $1,166.12 03 Nov 2021
Total Fee charged $0.02
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Sell);
        assert_eq!(txns[0].isin, Some("US88160R1014".to_string()));
    }
}
