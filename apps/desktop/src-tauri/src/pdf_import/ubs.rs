//! UBS AG PDF Parser
//!
//! Parses broker statements from UBS Switzerland AG.

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct UbsParser {
    detect_patterns: Vec<&'static str>,
}

impl UbsParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "UBS Switzerland",
                "UBS AG",
                "ubs.com",
                "CH-8098 Zürich",
            ],
        }
    }

    /// Parse Swiss number format (2'895.00 or 4'890.60)
    fn parse_swiss_number(s: &str) -> Option<f64> {
        let cleaned = s.trim().replace('\'', "").replace(',', ".");
        cleaned.parse::<f64>().ok()
    }

    /// Parse date format DD.MM.YYYY
    fn parse_date(s: &str) -> Option<NaiveDate> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let day: u32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let year: i32 = parts[2].parse().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type - "Börse Kauf" or "Börse Verkauf"
        let type_re = Regex::new(r"Börse\s+(Kauf|Verkauf)").ok();
        let txn_type = type_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                if &c[1] == "Kauf" { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell }
            });

        if txn_type.is_none() {
            return transactions;
        }

        let txn_type = txn_type.unwrap();

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract security name - line with fund name
        let name_re = Regex::new(r"(?:USD|CHF|EUR)\s+[\d',]+\s+([^\n]+?)\s+\d{8}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Komptant 450"
        let shares_re = Regex::new(r"Komptant\s+([\d',]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        // Extract price - "USD 10.868" or similar
        let price_re = Regex::new(r"Trans\.-Preis[^\n]*\n[^\n]*\n[^\n]*([\d.,]+)").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        // Extract transaction value - "Transaktionswert USD 4'890.60"
        let value_re = Regex::new(r"Transaktionswert\s+([A-Z]{3})\s+([\d',.-]+)").ok();
        let (currency, gross_amount) = value_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_swiss_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("CHF".to_string(), 0.0));

        // Extract fees - "Courtage USD -22.01"
        let courtage_re = Regex::new(r"Courtage\s+[A-Z]{3}\s+-?([\d',.-]+)").ok();
        let fees = courtage_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]))
            .unwrap_or(0.0);

        // Extract date - "Abschluss 08.03.2022"
        let date_re = Regex::new(r"Abschluss\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Abrechnungsbetrag USD -4'919.95"
        let total_re = Regex::new(r"Abrechnungsbetrag\s+[A-Z]{3}\s+-?([\d',.-]+)").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]))
            .unwrap_or(gross_amount + fees);

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
                note: Some("UBS Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Ausschüttung") {
            return transactions;
        }

        let isin = extract_isin(content);

        let shares_re = Regex::new(r"Anzahl[/\s]Betrag\s+([\d',]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        let gross_re = Regex::new(r"Brutto\s+([A-Z]{3})\s+([\d',.-]+)").ok();
        let (currency, gross_amount) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_swiss_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("CHF".to_string(), 0.0));

        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency,
                note: Some("UBS Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for UbsParser {
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
        "UBS"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = UbsParser::new();
        assert!(parser.detect("UBS Switzerland AG"));
        assert!(parser.detect("www.ubs.com"));
        assert!(!parser.detect("Credit Suisse AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = UbsParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
UBS Switzerland AG
Börse Kauf Komptant 450 USD 10.868
LU0950674175
Abschluss 08.03.2022
Transaktionswert USD 4'890.60
Courtage USD -22.01
Abrechnungsbetrag USD -4'919.95
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("LU0950674175".to_string()));
        assert_eq!(txns[0].currency, "USD");
    }
}
