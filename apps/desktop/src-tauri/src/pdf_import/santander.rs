//! Santander Consumer Bank PDF Parser
//!
//! Parses broker statements from Santander Consumer Bank AG (Germany).

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct SantanderParser {
    detect_patterns: Vec<&'static str>,
}

impl SantanderParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Santander Consumer Bank",
                "SCFBDE33",
                "41061 Mönchengladbach",
                "Santander-Platz 1",
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

        // Extract ISIN and WKN - format: "US88579Y1010 (851745)"
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name - "Stück 2 3M CO." format
        let name_re = Regex::new(r"Stück\s+[\d.,]+\s+([A-Z0-9][^\n]+?)(?:\s+[A-Z]{2}[A-Z0-9]{10})").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück 2"
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Ausführungskurs 158,98 EUR"
        let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*([A-Z]{3})").ok();
        let (price_per_share, currency) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]), c[2].to_string()))
            .unwrap_or((None, "EUR".to_string()));

        // Extract gross amount - "Kurswert 317,96- EUR"
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)-?\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Provision 7,90- EUR"
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Ausmachender Betrag 325,86- EUR"
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)-?\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Schlusstag/-Zeit 17.03.2021"
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
                note: Some("Santander Import".to_string()),
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

        // Extract gross amount
        let gross_re = Regex::new(r"(?:Ausschüttung|Brutto)\s+([\d.,]+)\s*([A-Z]{3})").ok();
        let (gross_amount, currency) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "EUR".to_string()));

        // Extract taxes
        let tax_re = Regex::new(r"Kapitalertragsteuer\s+([\d.,]+)\s*EUR").ok();
        let taxes = tax_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract net amount
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

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
                taxes,
                net_amount,
                currency,
                note: Some("Santander Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for SantanderParser {
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
        "Santander Consumer Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = SantanderParser::new();
        assert!(parser.detect("Santander Consumer Bank AG"));
        assert!(parser.detect("SCFBDE33XXX"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = SantanderParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Santander Consumer Bank AG · Santander-Platz 1 · 41061 Mönchengladbach
Wertpapier Abrechnung Kauf
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 2 3M CO. US88579Y1010 (851745)
REGISTERED SHARES DL -,01
Schlusstag/-Zeit 17.03.2021 16:53:45
Ausführungskurs 158,98 EUR
Kurswert 317,96- EUR
Provision 7,90- EUR
Ausmachender Betrag 325,86- EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("US88579Y1010".to_string()));
    }
}
