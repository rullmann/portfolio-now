//! Baader Bank PDF Parser
//!
//! Parses broker statements from Baader Bank AG.

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct BaaderBankParser {
    detect_patterns: Vec<&'static str>,
}

impl BaaderBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Baader Bank",
                "baaderbank.de",
                "85716 Unterschleißheim",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Wertpapierabrechnung: Kauf" or "Wertpapierabrechnung: Verkauf"
        let type_re = Regex::new(r"Wertpapierabrechnung:\s*(Kauf|Verkauf)").ok();
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

        // Extract ISIN and WKN - "ISIN: IE0032895942 WKN: 911950"
        let isin_re = Regex::new(r"ISIN:\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let wkn_re = Regex::new(r"WKN:\s*([A-Z0-9]{6})").ok();

        let isin = isin_re.as_ref().and_then(|re| re.captures(content)).map(|c| c[1].to_string());
        let wkn = wkn_re.as_ref().and_then(|re| re.captures(content)).map(|c| c[1].to_string());

        // Extract security name - after "STK X" line
        let name_re = Regex::new(r"STK\s+\d+\s+([^\n]+?)\s+EUR").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "STK 2"
        let shares_re = Regex::new(r"STK\s+([\d.,]+)\s").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs EUR 104,37" or "EUR 104,37" after shares
        let price_re = Regex::new(r"(?:Kurs\s+)?EUR\s+([\d.,]+)\s*\n").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert
        let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Provision EUR 0,21"
        let provision_re = Regex::new(r"Provision\s+EUR\s+([\d.,]+)").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract date - "Handelsdatum ... 20.03.2017"
        let date_re = Regex::new(r"(?:Handelsdatum|Auftragsdatum)[:\s]+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Zu Lasten Konto ... EUR 208,95"
        let total_re = Regex::new(r"Zu (?:Lasten|Gunsten)[^\n]*EUR\s+([\d.,]+)").ok();
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
                note: Some("Baader Bank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Ertrag") && !content.contains("Ausschüttung") {
            return transactions;
        }

        let isin = extract_isin(content);
        let wkn = extract_wkn(content);

        let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        let gross_re = Regex::new(r"Zu Gunsten[^\n]*EUR\s+([\d.,]+)").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"Valuta[:\s]+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("Baader Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for BaaderBankParser {
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
        "Baader Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = BaaderBankParser::new();
        assert!(parser.detect("Baader Bank Aktiengesellschaft"));
        assert!(parser.detect("service@baaderbank.de"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = BaaderBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Baader Bank Aktiengesellschaft
Wertpapierabrechnung: Kauf
Auftragsdatum: 20.03.2017
Nominale ISIN: IE0032895942 WKN: 911950 Kurs
STK 2 iShs DL Corp Bond UCITS ETF EUR 104,37
Kurswert EUR 208,74
Provision EUR 0,21
Zu Lasten Konto 12345004 Valuta: 22.03.2017 EUR 208,95
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("IE0032895942".to_string()));
        assert!((txns[0].shares.unwrap() - 2.0).abs() < 0.001);
    }
}
