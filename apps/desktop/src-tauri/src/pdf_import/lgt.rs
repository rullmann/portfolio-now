//! LGT Bank PDF Parser
//!
//! Parses broker statements from LGT Bank AG (Liechtenstein).

use super::{
    extract_isin, extract_wkn, parse_german_date, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct LgtParser {
    detect_patterns: Vec<&'static str>,
}

impl LgtParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "LGT Bank",
                "BLFLLI2X",
                "FL-9490 Vaduz",
                "Herrengasse 12",
            ],
        }
    }

    /// Parse Swiss/LI number format (6'732.00 -> 6732.00)
    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace('\'', "")
            .replace(',', ".");
        cleaned.parse::<f64>().ok()
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Abrechnung Kauf" or "Abrechnung Verkauf"
        let type_re = Regex::new(r"Abrechnung\s+(Kauf|Verkauf)").ok();
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

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract WKN - "Wertpapierkennnummer 861837"
        let wkn_re = Regex::new(r"Wertpapierkennnummer\s+(\d+)").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_wkn(content));

        // Extract security name - "Titel X"
        let name_re = Regex::new(r"Titel\s+([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Anzahl 12 Stück"
        let shares_re = Regex::new(r"Anzahl\s+([\d'.,]+)\s*(?:Stück|St)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        // Extract price and currency - "Kurs DKK 6'732.00"
        let price_re = Regex::new(r"Kurs\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let (currency, price_per_share) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2])))
            .unwrap_or(("EUR".to_string(), None));

        // Extract Kurswert - "Kurswert DKK 80'784.00"
        let kurswert_re = Regex::new(r"Kurswert\s+[A-Z]{3}\s+([\d'.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]))
            .unwrap_or(0.0);

        // Extract fees
        let umsatz_re = Regex::new(r"Eidg\. Umsatzabgabe\s+[A-Z]{3}\s+([\d'.,]+)").ok();
        let courtage_re = Regex::new(r"Courtage\s+[A-Z]{3}\s+([\d'.,]+)").ok();
        let broker_re = Regex::new(r"Broker Kommission\s+[A-Z]{3}\s+([\d'.,]+)").ok();

        let mut fees = 0.0;
        for re_opt in [&umsatz_re, &courtage_re, &broker_re] {
            if let Some(re) = re_opt {
                if let Some(caps) = re.captures(content) {
                    fees += Self::parse_number(&caps[1]).unwrap_or(0.0);
                }
            }
        }

        // Extract date - "Abschlussdatum 14.04.2020"
        let date_re = Regex::new(r"Abschlussdatum\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Belastung DKK Konto ... DKK 82'452.21"
        let total_re = Regex::new(r"Belastung[^\d]+([\d'.,]+)\s*$").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]))
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
                currency,
                note: Some("LGT Bank Import".to_string()),
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
        let wkn = extract_wkn(content);

        let shares_re = Regex::new(r"Anzahl\s+([\d'.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        let gross_re = Regex::new(r"Brutto\s+([A-Z]{3})\s+([\d'.,]+)").ok();
        let (currency, gross_amount) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("EUR".to_string(), 0.0));

        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                currency,
                note: Some("LGT Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for LgtParser {
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
        "LGT Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = LgtParser::new();
        assert!(parser.detect("LGT Bank AG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = LgtParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
LGT Bank AG
Abrechnung Kauf
Titel A.P. Moeller - Maersk A/S
ISIN DK0010244508
Wertpapierkennnummer 861837
Abschlussdatum 14.04.2020 09:00:02
Anzahl 12 Stück
Kurs DKK 6'732.00
Kurswert DKK 80'784.00
Eidg. Umsatzabgabe DKK 121.19
Courtage DKK 1'534.90
Broker Kommission DKK 12.12
Belastung DKK Konto 0037156.021 DKK 82'452.21
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("DK0010244508".to_string()));
    }
}
