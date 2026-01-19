//! ebase (European Bank for Financial Services) PDF Parser
//!
//! Parses broker statements from ebase GmbH.

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct EbaseParser {
    detect_patterns: Vec<&'static str>,
}

impl EbaseParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "European Bank for Financial Services",
                "ebase",
                "80002 München",
                "Postfach 200252",
            ],
        }
    }

    /// Clean OCR artifacts (} -> ü, { -> ä, etc.)
    fn clean_ocr(s: &str) -> String {
        s.replace('}', "ü")
            .replace('{', "ä")
            .replace('|', "ö")
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();
        let content = Self::clean_ocr(content);

        let is_buy = content.contains("Kauf") && !content.contains("Verkauf");
        let is_sell = content.contains("Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - "ISIN (WKN)" pattern
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(&content), extract_wkn(&content)));

        // Extract shares
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract date
        let date_re = Regex::new(r"(?:Handelstag|Schlusstag)\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)[+-]?\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount);

        if gross_amount > 0.0 || shares.is_some() {
            transactions.push(ParsedTransaction {
                date,
                time: None,
                txn_type,
                security_name: None,
                isin,
                wkn,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("ebase Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();
        let content = Self::clean_ocr(content);

        if !content.contains("Dividendengutschrift") && !content.contains("Ertragsgutschrift") {
            return transactions;
        }

        // Extract ISIN and WKN
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(&content), extract_wkn(&content)));

        // Extract shares
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross dividend in EUR
        let gross_re = Regex::new(r"Dividendengutschrift[^\n]*\n?[^\d]*([\d.,]+)\s*(?:USD|EUR)[^\d]*([\d.,]+)\+?\s*EUR").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[2]))
            .unwrap_or(0.0);

        // Extract withholding tax
        let quellensteuer_re = Regex::new(r"Einbehaltene Quellensteuer[^\d]*([\d.,]+)-?\s*EUR").ok();
        let withholding_tax = quellensteuer_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract German taxes
        let kapst_re = Regex::new(r"Kapitalertragsteuer[^\d]*([\d.,]+)-?\s*EUR").ok();
        let kapst = kapst_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let soli_re = Regex::new(r"Solidaritätszuschlag[^\d]*([\d.,]+)-?\s*EUR").ok();
        let soli = soli_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let taxes = withholding_tax + kapst + soli;

        // Extract fees
        let fees_re = Regex::new(r"Fremde Spesen\s+([\d.,]+)-?\s*EUR").ok();
        let fees = fees_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract net amount
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(&content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes - fees);

        // Extract date
        let date_re = Regex::new(r"Zahlbarkeitstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(&content))
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
                fees,
                taxes,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("ebase Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for EbaseParser {
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
        "ebase"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = EbaseParser::new();
        assert!(parser.detect("European Bank for Financial Services GmbH"));
        assert!(parser.detect("ebase"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_clean_ocr() {
        assert_eq!(EbaseParser::clean_ocr("St}ck"), "Stück");
        assert_eq!(EbaseParser::clean_ocr("Gesch{ftsjahr"), "Geschäftsjahr");
    }

    #[test]
    fn test_parse_dividend() {
        let parser = EbaseParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
European Bank for Financial Services GmbH
Dividendengutschrift
Stück 180 GAZPROM NEFT PJSC US36829G1076 (A0J4TC)
Zahlbarkeitstag 22.07.2021
Dividendengutschrift 120,92 USD 102,47+ EUR
Einbehaltene Quellensteuer 15 % auf 120,92 USD 15,37- EUR
Fremde Spesen 3,05- EUR
Ausmachender Betrag 84,05+ EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txns[0].isin, Some("US36829G1076".to_string()));
    }
}
