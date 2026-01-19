//! Oldenburgische Landesbank (OLB) PDF Parser
//!
//! Parses broker statements from Oldenburgische Landesbank AG (Germany).

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct OlbParser {
    detect_patterns: Vec<&'static str>,
}

impl OlbParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Oldenburgische Landesbank",
                "OLB Team",
                "26122 Oldenburg",
                "olb.de",
                "Stau 15/17",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "WERTPAPIERABRECHNUNG" + "Kauf" or "Verkauf"
        let has_abrechnung = content.contains("WERTPAPIERABRECHNUNG");
        let is_buy = has_abrechnung && content.contains("Kauf");
        let is_sell = has_abrechnung && content.contains("Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - format: "DE000A0H0785 (A0H078)"
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name - "Kauf - iS.EO G.B.C.1.5-10.5y.U.ETF DE Inhaber-Anteile"
        let name_re = Regex::new(r"(?:Kauf|Verkauf)\s+-\s+([^\n]+?)(?:\s+Inhaber|\s+Bearer)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück/ Nominale" line then "0,033037"
        let shares_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\([A-Z0-9]+\)\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[2]));

        // Extract price and gross - "0,033037 10,0350 EUR 1,47 EUR"
        let line_re = Regex::new(r"[\d.,]+\s+([\d.,]+)\s+EUR\s+([\d.,]+)\s+EUR").ok();
        let (price_per_share, gross_amount) = line_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]), parse_german_decimal(&c[2]).unwrap_or(0.0)))
            .unwrap_or((None, 0.0));

        // Extract fees - "Orderentgelt: 0,01 EUR"
        let fees_re = Regex::new(r"Orderentgelt:\s*([\d.,]+)\s*EUR").ok();
        let fees = fees_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Ausmachender Betrag: 1,48 EUR"
        let total_re = Regex::new(r"Ausmachender Betrag:\s*([\d.,]+)\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Ausführung 17.05.2023"
        let date_re = Regex::new(r"Ausführung\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                currency: "EUR".to_string(),
                note: Some("OLB Import".to_string()),
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

        // Extract net amount
        let net_re = Regex::new(r"Ausmachender Betrag:?\s*([\d.,]+)\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount);

        // Extract date
        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("OLB Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for OlbParser {
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
        "Oldenburgische Landesbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = OlbParser::new();
        assert!(parser.detect("Oldenburgische Landesbank AG"));
        assert!(parser.detect("OLB Team"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = OlbParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Oldenburgische Landesbank AG
WERTPAPIERABRECHNUNG
Kauf - iS.EO G.B.C.1.5-10.5y.U.ETF DE Inhaber-Anteile
Ausführung 17.05.2023
ISIN (WKN) Stück/ Nominale Kurswert Währung Bruttobetrag
DE000A0H0785 (A0H078) 0,033037 10,0350 EUR 1,47 EUR
Orderentgelt: 0,01 EUR
Ausmachender Betrag: 1,48 EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("DE000A0H0785".to_string()));
    }
}
