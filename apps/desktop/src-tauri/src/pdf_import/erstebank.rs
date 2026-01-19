//! Erste Bank PDF Parser
//!
//! Parses broker statements from Erste Bank der oesterreichischen Sparkassen AG.

use super::{
    extract_isin, parse_german_date, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct ErsteBankParser {
    detect_patterns: Vec<&'static str>,
}

impl ErsteBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Erste Bank",
                "ECTRATWW",
                "Brokerjet",
                "brokerjet.at",
                "oesterreichischen Sparkassen",
            ],
        }
    }

    /// Parse number format
    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace(',', ".")
            .replace(' ', "");
        cleaned.parse::<f64>().ok()
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        let is_buy = content.contains("KAUF") || content.contains("Kauf");
        let is_sell = content.contains("VERKAUF") || content.contains("Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN - use specific pattern for Erste Bank format "ISIN : CA0679011084"
        let isin_re = Regex::new(r"ISIN\s*:\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract security name
        let name_re = Regex::new(r"Wertpapierbezeichnung\s*:\s*([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares
        let shares_re = Regex::new(r"(?:Stück|Anzahl)\s*:\s*([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        // Extract price
        let price_re = Regex::new(r"Kurs\s*:\s*([A-Z]{3})?\s*([\d.,]+)").ok();
        let (currency, price_per_share) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                let curr = c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "EUR".to_string());
                (curr, Self::parse_number(&c[2]))
            })
            .unwrap_or(("EUR".to_string(), None));

        // Extract total
        let total_re = Regex::new(r"Gesamtbetrag\s*(?:\(in[^)]*\))?\s*:\s*([A-Z]{3})?\s*([\d.,]+)").ok();
        let gross_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[2]))
            .unwrap_or(0.0);

        // Extract date
        let date_re = Regex::new(r"(?:Handelstag|Schlusstag)\s*:\s*(\d{2}\.\d{2}\.\d{4})").ok();
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
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency,
                note: Some("Erste Bank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("DIVIDENDE") && !content.contains("BARDIVIDENDE")
            && !content.contains("Dividende") {
            return transactions;
        }

        // Extract ISIN - use specific pattern for Erste Bank format "ISIN : CA0679011084"
        let isin_re = Regex::new(r"ISIN\s*:\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract security name
        let name_re = Regex::new(r"Wertpapierbezeichnung\s*:\s*([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Anspruchsberechtigter : 35"
        let shares_re = Regex::new(r"(?:Anspruchsberechtigter|Bestand)\s*:\s*([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        // Extract gross amount
        let gross_re = Regex::new(r"Brutto-Betrag\s*:\s*([A-Z]{3})\s*([\d.,]+)").ok();
        let (currency, gross_amount) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("EUR".to_string(), 0.0));

        // Extract taxes (Quellensteuer + KESt)
        let tax_re = Regex::new(r"Steuern\s*:\s*[A-Z]{3}\s*([\d.,]+)").ok();
        let mut taxes = 0.0;
        if let Some(re) = &tax_re {
            for caps in re.captures_iter(content) {
                taxes += Self::parse_number(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract net amount in account currency
        let net_re = Regex::new(r"Gesamtbetrag \(in[^)]*\)\s*:\s*([A-Z]{3})\s*([\d.,]+)").ok();
        let (final_currency, net_amount) = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or((currency.clone(), gross_amount - taxes));

        // Extract date
        let date_re = Regex::new(r"(?:Valutatag|Zahltag)\s*:\s*(\d{2}\.\d{2}\.\d{4})").ok();
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
                security_name,
                isin,
                wkn: None,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes,
                net_amount,
                currency: final_currency,
                note: Some("Erste Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: Some(currency),
            });
        }

        transactions
    }
}

impl BankParser for ErsteBankParser {
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
        "Erste Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = ErsteBankParser::new();
        assert!(parser.detect("Erste Bank der oesterreichischen Sparkassen AG"));
        assert!(parser.detect("Brokerjet"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_dividend() {
        let parser = ErsteBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Erste Bank
WERTPAPIERBESTÄTIGUNG
BARDIVIDENDE
ISIN : CA0679011084
Wertpapierbezeichnung : BARRICK GOLD CORP.
Anspruchsberechtigter : 35
Brutto-Betrag : USD 0.7
Steuern : USD 0.18
Steuern : USD 0.07
Gesamtbetrag (in Kontowährung) : EUR 0.4
Valutatag : 15.09.2015
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txns[0].isin, Some("CA0679011084".to_string()));
    }
}
