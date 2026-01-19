//! Swissquote Bank PDF Parser
//!
//! Parses broker statements from Swissquote Bank AG (Switzerland).

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct SwissquoteParser {
    detect_patterns: Vec<&'static str>,
}

impl SwissquoteParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Swissquote",
                "swissquote.ch",
                "CH-1196 Gland",
            ],
        }
    }

    /// Parse Swiss date format (05.08.2019)
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

    /// Parse Swiss number format (2'895.00 -> 2895.00)
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

        // Extract security name - first line after "Titel"
        let name_re = Regex::new(r"(?:Titel|Title)\s+[^\n]*\n([^\n]+?)\s+ISIN").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string())
            .or_else(|| {
                let alt_re = Regex::new(r"([A-Z][A-Z\s]+)\s+ISIN").ok();
                alt_re.as_ref().and_then(|re| re.captures(content)).map(|c| c[1].trim().to_string())
            });

        // Extract shares - "Anzahl ... 15"
        let shares_re = Regex::new(r"(?:Anzahl|Quantity)\s+[\w\s]*?([\d']+)\s").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        // Extract price - "Preis ... 193"
        let price_re = Regex::new(r"(?:Preis|Price)\s+[\w\s]*?([\d'.,]+)\s").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[1]));

        // Extract total amount in original currency
        let total_re = Regex::new(r"Total\s+([A-Z]{3})\s+([\d',.-]+)").ok();
        let (currency, gross_amount) = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_swiss_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("USD".to_string(), 0.0));

        // Extract commission
        let commission_re = Regex::new(r"(?:Kommission|Commission)[^\n]*([A-Z]{3})\s+([\d'.,]+)").ok();
        let fees = commission_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[2]))
            .unwrap_or(0.0);

        // Extract date - "vom 05.08.2019"
        let date_re = Regex::new(r"vom\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract final amount - "Zu Ihren Lasten USD 2'900.60"
        let final_re = Regex::new(r"Zu Ihren (?:Lasten|Gunsten)\s+([A-Z]{3})\s+([\d',.-]+)").ok();
        let net_amount = final_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_swiss_number(&c[2]))
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
                note: Some("Swissquote Import".to_string()),
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

        let gross_re = Regex::new(r"(?:Brutto|Gross)\s+([A-Z]{3})\s+([\d',.-]+)").ok();
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
                note: Some("Swissquote Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for SwissquoteParser {
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
        "Swissquote"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = SwissquoteParser::new();
        assert!(parser.detect("Swissquote Bank AG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_swiss_number() {
        assert_eq!(SwissquoteParser::parse_swiss_number("2'895.00"), Some(2895.0));
        assert_eq!(SwissquoteParser::parse_swiss_number("1'234'567.89"), Some(1234567.89));
    }

    #[test]
    fn test_parse_buy() {
        let parser = SwissquoteParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Swissquote Bank AG
Börsentransaktion: Kauf
APPLE ORD ISIN: US0378331005
Anzahl Preis Betrag
15 193 USD 2'895.00
Total USD 2'895.00
Kommission Swissquote Bank AG USD 0.85
Zu Ihren Lasten USD 2'900.60
vom 05.08.2019
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("US0378331005".to_string()));
        assert_eq!(txns[0].currency, "USD");
    }
}
