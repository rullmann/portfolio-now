//! 1822direkt PDF Parser
//!
//! Parses broker statements from 1822direkt (Frankfurter Sparkasse).

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct Direkt1822Parser {
    detect_patterns: Vec<&'static str>,
}

impl Direkt1822Parser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "1822direkt",
                "60255 Frankfurt",
                "HELADEF1822",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        let is_buy = content.contains("Abrechnung Kauf") && !content.contains("Verkauf");
        let is_sell = content.contains("Abrechnung Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - "LU0635178014 (ETF127)"
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), None));

        // Extract security name
        let name_re = Regex::new(r"Stück\s+\d+\s+([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price
        let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*EUR").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)-?\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
        let spesen_re = Regex::new(r"Eigene Spesen\s+([\d.,]+)-?\s*EUR").ok();

        let mut fees = 0.0;
        if let Some(re) = &provision_re {
            if let Some(caps) = re.captures(content) {
                fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &spesen_re {
            if let Some(caps) = re.captures(content) {
                fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract date
        let date_re = Regex::new(r"Schlusstag(?:/\-Zeit)?\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)-?\s*EUR").ok();
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
                note: Some("1822direkt Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividendengutschrift") && !content.contains("Ertragsgutschrift") {
            return transactions;
        }

        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), None));

        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        let gross_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"Zahlbarkeitstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                wkn,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency: "EUR".to_string(),
                note: Some("1822direkt Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for Direkt1822Parser {
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
        "1822direkt"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = Direkt1822Parser::new();
        assert!(parser.detect("1822direkt"));
        assert!(parser.detect("BIC HELADEF1822"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = Direkt1822Parser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
1822direkt
Wertpapier Abrechnung Kauf
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 13 COMSTA.-MSCI EM.MKTS.TRN U.ETF LU0635178014 (ETF127)
Schlusstag/-Zeit 01.12.2017 10:30:52
Ausführungskurs 40,968 EUR
Kurswert 532,58- EUR
Provision 4,95- EUR
Eigene Spesen 1,95- EUR
Ausmachender Betrag 539,48- EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("LU0635178014".to_string()));
        assert!((txns[0].shares.unwrap() - 13.0).abs() < 0.001);
    }
}
