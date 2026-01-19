//! Raiffeisen Bank Group PDF Parser
//!
//! Parses broker statements from Raiffeisen Bank Group (Austria).

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct RaiffeisenParser {
    detect_patterns: Vec<&'static str>,
}

impl RaiffeisenParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Raiffeisenbank",
                "Raiffeisen",
                "KESt-Neubestand",
                "ATU16",  // Austrian UID starting pattern
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Geschäftsart: Kauf/Verkauf"
        let type_re = Regex::new(r"Geschäftsart:\s*(Kauf|Verkauf)").ok();
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

        // Extract ISIN - "Titel: DE000BAY0017 Bayer AG"
        let isin_re = Regex::new(r"Titel:\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract security name - "Titel: DE000BAY0017 Bayer AG"
        let name_re = Regex::new(r"Titel:\s*[A-Z]{2}[A-Z0-9]{10}\s+([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Zugang: 2 Stk"
        let shares_re = Regex::new(r"(?:Zugang|Abgang):\s*([\d.,]+)\s*Stk").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs: 53,47 EUR"
        let price_re = Regex::new(r"Kurs:\s*([\d.,]+)\s*([A-Z]{3})").ok();
        let (price_per_share, currency) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]), c[2].to_string()))
            .unwrap_or((None, "EUR".to_string()));

        // Extract gross amount - "Kurswert: -106,94 EUR"
        let kurswert_re = Regex::new(r"Kurswert:\s*-?([\d.,]+)\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Serviceentgelt: -0,32 EUR"
        let fees_re = Regex::new(r"Serviceentgelt:\s*-?([\d.,]+)\s*EUR").ok();
        let fees = fees_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Zu Lasten IBAN ... -107,26 EUR"
        let total_re = Regex::new(r"Zu (?:Lasten|Gunsten)\s+IBAN\s+[A-Z0-9\s]+\s+-?([\d.,]+)\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Schlusstag: 03.05.2021"
        let date_re = Regex::new(r"Schlusstag:\s*(\d{2}\.\d{2}\.\d{4})").ok();
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
                wkn: None,
                shares,
                price_per_share,
                gross_amount,
                fees,
                taxes: 0.0,
                net_amount,
                currency,
                note: Some("Raiffeisen Import".to_string()),
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

        // Extract ISIN
        let isin_re = Regex::new(r"Titel:\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract shares
        let shares_re = Regex::new(r"Bestand:\s*([\d.,]+)\s*Stk").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross amount - "Bruttobetrag: X,XX EUR"
        let gross_re = Regex::new(r"(?:Bruttobetrag|Brutto):\s*([\d.,]+)\s*([A-Z]{3})").ok();
        let (gross_amount, currency) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]).unwrap_or(0.0), c[2].to_string()))
            .unwrap_or((0.0, "EUR".to_string()));

        // Extract KESt (Austrian capital gains tax)
        let kest_re = Regex::new(r"KESt:\s*-?([\d.,]+)\s*EUR").ok();
        let taxes = kest_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract net amount
        let net_re = Regex::new(r"Zu Gunsten\s+IBAN\s+[A-Z0-9\s]+\s+([\d.,]+)\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

        // Extract date
        let date_re = Regex::new(r"(?:Valuta|Zahltag):\s*(\d{2}\.\d{2}\.\d{4})").ok();
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
                wkn: None,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes,
                net_amount,
                currency,
                note: Some("Raiffeisen Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for RaiffeisenParser {
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
        "Raiffeisen Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = RaiffeisenParser::new();
        assert!(parser.detect("Raiffeisenbank"));
        assert!(parser.detect("KESt-Neubestand"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = RaiffeisenParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Raiffeisenbank AAAAAAAAA eGen
Geschäftsart: Kauf Auftrags-Nr.: 11441163 - 03.05.2021
Zugang: 2 Stk
Titel: DE000BAY0017 Bayer AG
Namens-Aktien o.N.
Kurs: 53,47 EUR
Kurswert: -106,94 EUR
Serviceentgelt: -0,32 EUR
Zu Lasten IBAN AT99 9999 9000 0011 1110 -107,26 EUR
Schlusstag: 03.05.2021
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("DE000BAY0017".to_string()));
    }
}
