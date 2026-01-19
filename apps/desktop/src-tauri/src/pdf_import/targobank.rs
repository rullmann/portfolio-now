//! Targobank PDF Parser
//!
//! Parses broker statements from TARGOBANK AG.

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct TargobankParser {
    detect_patterns: Vec<&'static str>,
}

impl TargobankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "TARGOBANK",
                "targobank.de",
                "47002 Duisburg",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect transaction type
        let type_re = Regex::new(r"Transaktionstyp\s+(Kauf|Verkauf)").ok();
        let txn_type = type_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                if &c[1] == "Kauf" {
                    ParsedTransactionType::Buy
                } else {
                    ParsedTransactionType::Sell
                }
            });

        if txn_type.is_none() {
            return transactions;
        }

        let txn_type = txn_type.unwrap();

        // Extract ISIN and WKN - "WKN / ISIN ABC123 / DE0000ABC123"
        let isin_wkn_re = Regex::new(r"WKN\s*/\s*ISIN\s+([A-Z0-9]{6})\s*/\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let (wkn, isin) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_wkn(content), extract_isin(content)));

        // Extract security name - "Wertpapier FanCy shaRe. nAmE X0-X0"
        let name_re = Regex::new(r"Wertpapier\s+([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück 987,654"
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs 12,34 EUR"
        let price_re = Regex::new(r"Kurs\s+([\d.,]+)\s*EUR").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Provision 8,90 EUR"
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)\s*EUR").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract date - "Schlusstag / Handelszeit 02.01.2020"
        let date_re = Regex::new(r"Schlusstag\s*/?\s*(?:Handelszeit)?\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Konto-Nr. 0101753165 1.008,91 EUR"
        let total_re = Regex::new(r"Konto-Nr\.\s+\d+\s+([\d.,]+)\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        if gross_amount > 0.0 || shares.is_some() {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type,
                security_name,
                isin,
                wkn,
                shares,
                price_per_share,
                gross_amount,
                fees,
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Targobank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Ertrag") {
            return transactions;
        }

        // Similar pattern extraction for dividends
        let isin = extract_isin(content);

        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        let gross_re = Regex::new(r"(?:Brutto|Gutschrift)\s+([\d.,]+)\s*EUR").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"(?:Zahltag|Valuta)\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

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
                currency: "EUR".to_string(),
                note: Some("Targobank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for TargobankParser {
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
        "Targobank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = TargobankParser::new();
        assert!(parser.detect("TARGOBANK AG"));
        assert!(parser.detect("www.targobank.de"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = TargobankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
TARGOBANK AG
Transaktionstyp Kauf
Stück 987,654
Wertpapier FanCy shaRe. nAmE X0-X0
WKN / ISIN ABC123 / DE0000ABC123
Schlusstag / Handelszeit 02.01.2020 / 13:01:00
Kurs 12,34 EUR
Kurswert 1.000,01 EUR
Provision 8,90 EUR
Konto-Nr. 0101753165 1.008,91 EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("DE0000ABC123".to_string()));
        assert!((txns[0].shares.unwrap() - 987.654).abs() < 0.001);
    }
}
