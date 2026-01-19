//! Flatex (FinTech Group Bank) PDF Parser
//!
//! Parses broker statements from flatex / FinTech Group Bank AG.

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct FlatexParser {
    detect_patterns: Vec<&'static str>,
}

impl FlatexParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "FinTech Group Bank",
                "flatex",
                "biw AG",
                "biw Bank",
                "47877 Willich",
                "Rotfeder-Ring 7",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Wertpapierabrechnung Kauf" or "Wertpapierabrechnung Verkauf"
        let type_re = Regex::new(r"Wertpapierabrechnung\s+(Kauf|Verkauf)").ok();
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

        // Extract ISIN and WKN - "ISHSIII-S+P SM.CAP600 DLD (IE00B2QWCY14/A0Q1YY)"
        let isin_wkn_re = Regex::new(r"\(([A-Z]{2}[A-Z0-9]{10})/([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name
        let name_re = Regex::new(r"(?:Kauf|Verkauf)\s+([^\(]+?)\s*\([A-Z]{2}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Ausgeführt 19,334524 St."
        let shares_re = Regex::new(r"Ausgeführt\s+([\d.,]+)\s*St").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs 54,307000 EUR"
        let price_re = Regex::new(r"Kurs\s+([\d.,]+)\s*EUR").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert - "Kurswert EUR 1.050,00"
        let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees
        let courtage_re = Regex::new(r"Courtage\s+EUR\s+([\d.,]+)").ok();
        let trading_re = Regex::new(r"Tradinggebühr\s+EUR\s+([\d.,]+)").ok();
        let provision_re = Regex::new(r"Provision\s+EUR\s+([\d.,]+)").ok();

        let mut fees = 0.0;
        for re_opt in [&courtage_re, &trading_re, &provision_re] {
            if let Some(re) = re_opt {
                if let Some(caps) = re.captures(content) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
        }

        // Extract date - "Schlusstag 15.12.2016"
        let date_re = Regex::new(r"Schlusstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Endbetrag EUR -1.050,00"
        let total_re = Regex::new(r"Endbetrag\s+EUR\s+-?([\d.,]+)").ok();
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
                note: Some("flatex Import".to_string()),
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

        let isin_wkn_re = Regex::new(r"\(([A-Z]{2}[A-Z0-9]{10})/([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        let shares_re = Regex::new(r"(?:STK|Stück)\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        let gross_re = Regex::new(r"Bruttobetrag\s+EUR\s+([\d.,]+)").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"Zahltag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract taxes
        let kapst_re = Regex::new(r"Kapitalertragsteuer\s+EUR\s+([\d.,]+)").ok();
        let soli_re = Regex::new(r"Solidaritätszuschlag\s+EUR\s+([\d.,]+)").ok();
        let kist_re = Regex::new(r"Kirchensteuer\s+EUR\s+([\d.,]+)").ok();

        let mut taxes = 0.0;
        for re_opt in [&kapst_re, &soli_re, &kist_re] {
            if let Some(re) = re_opt {
                if let Some(caps) = re.captures(content) {
                    taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
        }

        let net_re = Regex::new(r"Nettobetrag\s+EUR\s+([\d.,]+)").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

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
                note: Some("flatex Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for FlatexParser {
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
        "flatex"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = FlatexParser::new();
        assert!(parser.detect("FinTech Group Bank AG"));
        assert!(parser.detect("flatex"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = FlatexParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
FinTech Group Bank AG
Wertpapierabrechnung Kauf Fonds/Zertifikate
Nr.92212123/1  Kauf            ISHSIII-S+P SM.CAP600 DLD (IE00B2QWCY14/A0Q1YY)
Ausgeführt     19,334524 St.           Kurswert       EUR             1.050,00
Kurs           54,307000 EUR
Schlusstag        15.12.2016
Endbetrag      EUR            -1.050,00
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("IE00B2QWCY14".to_string()));
    }
}
