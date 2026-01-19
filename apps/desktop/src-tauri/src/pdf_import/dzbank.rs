//! DZ Bank Group PDF Parser
//!
//! Parses broker statements from DZ Bank AG and affiliated banks
//! (Volks- und Raiffeisenbanken, GLS Bank, etc.)

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct DzBankParser {
    detect_patterns: Vec<&'static str>,
}

impl DzBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "DZ BANK",
                "DZ Bank",
                "GLS Bank",
                "Volksbank",
                "Raiffeisenbank",
                "VR Bank",
                "GENODEM",
                "44774 Bochum",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        let is_buy = content.contains("Wertpapierabrechnung") && content.contains("Kauf") && !content.contains("Verkauf");
        let is_sell = content.contains("Wertpapierabrechnung") && content.contains("Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - "ISIN (WKN)" pattern
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

        // Extract price
        let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*EUR").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract date
        let date_re = Regex::new(r"Schlusstag(?:/Zeit)?\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)[+-]?\s*EUR").ok();
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
                security_name: None,
                isin,
                wkn,
                shares,
                price_per_share,
                gross_amount,
                fees,
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("DZ Bank Import".to_string()),
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

        // Extract gross dividend in EUR - "Dividendengutschrift 85,00 PLN 18,87+ EUR"
        let gross_re = Regex::new(r"Dividendengutschrift[^\d]*([\d.,]+)\s*(?:PLN|USD|EUR)[^\d]*([\d.,]+)\+?\s*EUR").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[2]))
            .unwrap_or(0.0);

        // Extract withholding tax
        let quellensteuer_re = Regex::new(r"Einbehaltene Quellensteuer[^\d]*([\d.,]+)-?\s*EUR").ok();
        let withholding_tax = quellensteuer_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract German taxes
        let kapst_re = Regex::new(r"Kapitalertragsteuer[^\d]*([\d.,]+)-?\s*EUR").ok();
        let kapst = kapst_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let soli_re = Regex::new(r"Solidaritätszuschlag[^\d]*([\d.,]+)-?\s*EUR").ok();
        let soli = soli_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let taxes = withholding_tax + kapst + soli;

        // Extract net amount
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

        // Extract date
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
                taxes,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("DZ Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for DzBankParser {
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
        "DZ Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = DzBankParser::new();
        assert!(parser.detect("GLS Bank"));
        assert!(parser.detect("DZ BANK AG"));
        assert!(parser.detect("Volksbank"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_dividend() {
        let parser = DzBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
GLS Bank · 44774 Bochum
Dividendengutschrift
Stück 17 CD PROJEKT S.A. PLOPTTC00011 (534356)
Zahlbarkeitstag 08.06.2021
Dividendengutschrift 85,00 PLN 18,87+ EUR
Einbehaltene Quellensteuer 19 % auf 85,00 PLN 3,59- EUR
Kapitalertragsteuer 25 % auf 7,55 EUR 1,89- EUR
Solidaritätszuschlag 5,5 % auf 1,89 EUR 0,11- EUR
Ausmachender Betrag 13,28+ EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txns[0].isin, Some("PLOPTTC00011".to_string()));
    }
}
