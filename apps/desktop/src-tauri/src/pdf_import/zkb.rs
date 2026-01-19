//! Zürcher Kantonalbank (ZKB) PDF Parser
//!
//! Parses broker statements from Zürcher Kantonalbank (Switzerland).

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct ZkbParser {
    detect_patterns: Vec<&'static str>,
}

impl ZkbParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Zürcher Kantonalbank",
                "ZKBKCHZZ",
                "zkb.ch",
                "8010 Zürich",
            ],
        }
    }

    /// Parse Swiss/German date format (D. Monat YYYY or DD.MM.YYYY)
    fn parse_date(s: &str) -> Option<NaiveDate> {
        // Try "4. Oktober 2021" format
        let months = [
            ("Januar", 1), ("Februar", 2), ("März", 3), ("April", 4),
            ("Mai", 5), ("Juni", 6), ("Juli", 7), ("August", 8),
            ("September", 9), ("Oktober", 10), ("November", 11), ("Dezember", 12),
        ];

        let re = Regex::new(r"(\d{1,2})\.\s*([A-Za-zä]+)\s+(\d{4})").ok()?;
        if let Some(caps) = re.captures(s) {
            let day: u32 = caps[1].parse().ok()?;
            let month_str = &caps[2];
            let year: i32 = caps[3].parse().ok()?;

            for (name, num) in months.iter() {
                if month_str.eq_ignore_ascii_case(name) {
                    return NaiveDate::from_ymd_opt(year, *num, day);
                }
            }
        }

        // Try DD.MM.YYYY
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() == 3 {
            let day: u32 = parts[0].parse().ok()?;
            let month: u32 = parts[1].parse().ok()?;
            let year: i32 = parts[2].parse().ok()?;
            return NaiveDate::from_ymd_opt(year, month, day);
        }

        None
    }

    /// Parse Swiss number format (3'545.00 -> 3545.00)
    fn parse_swiss_number(s: &str) -> Option<f64> {
        let cleaned = s.trim().replace('\'', "").replace(',', ".");
        cleaned.parse::<f64>().ok()
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Ihr Kauf" or "Ihr Verkauf"
        let is_buy = content.contains("Ihr Kauf") && !content.contains("Ihr Verkauf");
        let is_sell = content.contains("Ihr Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract Valor
        let valor_re = Regex::new(r"Valor\s+(\d+)").ok();
        let _valor = valor_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string());

        // Extract security name - line before "Valor"
        let name_re = Regex::new(r"Registered Shs\s+([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares, price and amount - "1'000 zu 3.545 3'545.00"
        let trade_re = Regex::new(r"([\d'.,]+)\s+zu\s+([\d'.,]+)\s+([\d'.,]+)").ok();
        let (shares, price_per_share, gross_amount) = trade_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                (
                    Self::parse_swiss_number(&c[1]),
                    Self::parse_swiss_number(&c[2]),
                    Self::parse_swiss_number(&c[3]).unwrap_or(0.0),
                )
            })
            .unwrap_or((None, None, 0.0));

        // Extract currency from "Stück GBP GBP" line
        let currency_re = Regex::new(r"Stück\s+([A-Z]{3})").ok();
        let currency = currency_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .unwrap_or_else(|| "CHF".to_string());

        // Extract fees
        let commission_re = Regex::new(r"Fremde Kommission\s+([\d'.,]+)").ok();
        let abgaben_re = Regex::new(r"Eidg\. Abgaben\s+([\d'.,]+)").ok();

        let mut fees = 0.0;
        if let Some(re) = &commission_re {
            if let Some(caps) = re.captures(content) {
                fees += Self::parse_swiss_number(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &abgaben_re {
            if let Some(caps) = re.captures(content) {
                fees += Self::parse_swiss_number(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract date from "Abschluss per: DD.MM.YYYY"
        let date_re = Regex::new(r"Abschluss per:\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total in CHF - "Total zu Ihren Lasten Valuta ... CHF X"
        let total_re = Regex::new(r"Total zu Ihren (?:Lasten|Gunsten)[^\d]+([\d'.,]+)").ok();
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
                note: Some("ZKB Import".to_string()),
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

        let shares_re = Regex::new(r"([\d']+)\s+(?:Stück|Shs)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        let gross_re = Regex::new(r"Brutto\s+([A-Z]{3})\s+([\d'.,]+)").ok();
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
                note: Some("ZKB Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for ZkbParser {
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
        "Zürcher Kantonalbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = ZkbParser::new();
        assert!(parser.detect("Zürcher Kantonalbank"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = ZkbParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Zürcher Kantonalbank
Effektenabrechnung
Ihr Kauf
Abschluss per: 04.10.2021
Registered Shs Glencore PLC
Valor 12964057 / ISIN JE00B4T3BW64
Stück GBP GBP
1'000 zu 3.545 3'545.00
Fremde Kommission 0.71
Eidg. Abgaben 5.31
Total zu Ihren Lasten Valuta 06.10.2021 4'469.94
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("JE00B4T3BW64".to_string()));
    }
}
