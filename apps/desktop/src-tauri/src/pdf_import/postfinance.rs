//! PostFinance PDF Parser
//!
//! Parses broker statements from PostFinance AG (Switzerland).

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct PostFinanceParser {
    detect_patterns: Vec<&'static str>,
}

impl PostFinanceParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "PostFinance",
                "postfinance.ch",
                "Mingerstrasse 20",
                "3030 Bern",
            ],
        }
    }

    /// Parse Swiss date format (DD.MM.YYYY)
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

    /// Parse Swiss number format (2'837.40 -> 2837.40)
    fn parse_swiss_number(s: &str) -> Option<f64> {
        let cleaned = s.trim().replace('\'', "").replace(',', ".");
        cleaned.parse::<f64>().ok()
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Börsentransaktion: Kauf" or "Börsentransaktion: Verkauf"
        let type_re = Regex::new(r"Börsentransaktion:\s*(Kauf|Verkauf|Achat|Vente)").ok();
        let txn_type = type_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                match c[1].to_lowercase().as_str() {
                    "kauf" | "achat" => ParsedTransactionType::Buy,
                    _ => ParsedTransactionType::Sell,
                }
            });

        if txn_type.is_none() {
            return transactions;
        }

        let txn_type = txn_type.unwrap();

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract security name - text before ISIN
        let name_re = Regex::new(r"Titel[^\n]*\n([^\n]+?)\s+ISIN").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares and price - "60 47.29 EUR 2'837.40"
        let trade_re = Regex::new(r"Anzahl\s+Preis\s+Betrag\n([\d'.,]+)\s+([\d'.,]+)\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let (shares, price_per_share, currency, gross_amount) = trade_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                (
                    Self::parse_swiss_number(&c[1]),
                    Self::parse_swiss_number(&c[2]),
                    c[3].to_string(),
                    Self::parse_swiss_number(&c[4]).unwrap_or(0.0),
                )
            })
            .unwrap_or((None, None, "CHF".to_string(), 0.0));

        // Extract commission
        let commission_re = Regex::new(r"Kommission\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let fees = commission_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[2]))
            .unwrap_or(0.0);

        // Extract stamp duty
        let stamp_re = Regex::new(r"(?:Stempelsteuer|Abgabe)[^\n]*([\d'.,]+)").ok();
        let stamp = stamp_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]))
            .unwrap_or(0.0);

        let total_fees = fees + stamp;

        // Extract date from "vom DD.MM.YYYY"
        let date_re = Regex::new(r"vom\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Zu Ihren Lasten EUR 2'850.24"
        let total_re = Regex::new(r"Zu Ihren (?:Lasten|Gunsten)\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[2]))
            .unwrap_or(gross_amount + total_fees);

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
                fees: total_fees,
                taxes: 0.0,
                net_amount,
                currency,
                note: Some("PostFinance Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Dividend") {
            return transactions;
        }

        let isin = extract_isin(content);

        let shares_re = Regex::new(r"(?:Anzahl|Quantity)\s+([\d']+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        let gross_re = Regex::new(r"(?:Brutto|Gross)\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let (currency, gross_amount) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_swiss_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("CHF".to_string(), 0.0));

        let date_re = Regex::new(r"(?:Valuta|Value date)\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("PostFinance Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for PostFinanceParser {
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
        "PostFinance"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = PostFinanceParser::new();
        assert!(parser.detect("PostFinance AG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = PostFinanceParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
PostFinance AG
Börsentransaktion: Kauf
Titel UNILEVER DUTCH CERT ISIN: NL0000009355 Amsterdam
Anzahl Preis Betrag
60 47.29 EUR 2'837.40
Total EUR 2'837.40
Kommission EUR 8.58
Abgabe (Eidg. Stempelsteuer) EUR 4.26
Zu Ihren Lasten EUR 2'850.24
vom 25.09.2018
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("NL0000009355".to_string()));
    }
}
