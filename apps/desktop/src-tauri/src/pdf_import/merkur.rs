//! Merkur Privatbank PDF Parser
//!
//! Parses broker statements from Merkur Privatbank AG (Germany).

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct MerkurParser {
    detect_patterns: Vec<&'static str>,
}

impl MerkurParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Merkur Privatbank",
                "GENODEF1M06",
                "97762 Hammelburg",
                "Am Marktplatz 10",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Wertpapier Abrechnung Kauf/Verkauf"
        let is_buy = content.contains("Abrechnung Kauf");
        let is_sell = content.contains("Abrechnung Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - format: "ISIN (WKN)" e.g., "IE00BJ0KDQ92 (A1XB5U)"
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name - line after "Nominale Wertpapierbezeichnung"
        let name_re = Regex::new(r"Stück\s+[\d.,]+\s+([A-Z][^\n]+?)(?:\s+[A-Z]{2}[A-Z0-9]{10})").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück 125,3258"
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Ausführungskurs 79,792 EUR"
        let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*([A-Z]{3})").ok();
        let (price_per_share, currency) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]), c[2].to_string()))
            .unwrap_or((None, "EUR".to_string()));

        // Extract gross amount - "Kurswert 10.000,00- EUR"
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)-?\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Provision 2,50- EUR"
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Ausmachender Betrag 10.002,50- EUR"
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)-?\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Schlusstag/-Zeit 02.05.2023"
        let date_re = Regex::new(r"Schlusstag(?:/-Zeit)?\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

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
                currency,
                note: Some("Merkur Privatbank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Ausschüttung") && !content.contains("Dividende")
            && !content.contains("Ertragsgutschrift") {
            return transactions;
        }

        // Extract ISIN and WKN
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract shares
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross amount - "Ausschüttung X,XX EUR"
        let gross_re = Regex::new(r"(?:Ausschüttung|Brutto)\s+([\d.,]+)\s*([A-Z]{3})").ok();
        let (gross_amount, currency) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "EUR".to_string()));

        // Extract net amount - "Ausmachender Betrag X,XX EUR"
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount);

        // Extract date
        let date_re = Regex::new(r"(?:Zahlbarkeitstag|Valuta)\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        if gross_amount > 0.0 || net_amount > 0.0 {
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
                net_amount,
                currency,
                note: Some("Merkur Privatbank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for MerkurParser {
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
        "Merkur Privatbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = MerkurParser::new();
        assert!(parser.detect("Merkur Privatbank"));
        assert!(parser.detect("GENODEF1M06"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = MerkurParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Am Marktplatz 10 · 97762 Hammelburg
Wertpapier Abrechnung Kauf
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 125,3258 XTR.(IE) - MSCI WORLD IE00BJ0KDQ92 (A1XB5U)
REGISTERED SHARES 1C O.N.
Schlusstag/-Zeit 02.05.2023 09:34:40
Ausführungskurs 79,792 EUR
Kurswert 10.000,00- EUR
Provision 2,50- EUR
Ausmachender Betrag 10.002,50- EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("IE00BJ0KDQ92".to_string()));
    }
}
