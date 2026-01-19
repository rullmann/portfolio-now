//! MLP Bank PDF Parser
//!
//! Parses broker statements from MLP Banking AG (Germany).

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct MlpBankParser {
    detect_patterns: Vec<&'static str>,
}

impl MlpBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "MLP Banking AG",
                "MLP Finanzberatung",
                "MLPBDE61",
                "69168 Wiesloch",
                "Alte Heerstraße 40",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Wertpapier Abrechnung Kauf/Verkauf"
        let is_buy = content.contains("Abrechnung Kauf");
        let is_sell = content.contains("Abrechnung Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN and WKN - format: "ISIN (WKN)" e.g., "LU0106280836 (930920)"
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name - line after "Nominale Wertpapierbezeichnung"
        let name_re = Regex::new(r"Stück\s+[\d.,]+\s+([A-Z][^\n]+?)(?:\s+[A-Z]{2}[A-Z0-9]{10})").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück 4,929"
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Ausführungskurs 20,29 EUR"
        let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*([A-Z]{3})").ok();
        let (price_per_share, currency) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]), c[2].to_string()))
            .unwrap_or((None, "EUR".to_string()));

        // Extract gross amount - "Kurswert 100,01- EUR"
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)-?\s*EUR").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Provision X,XX EUR"
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
        let fees = provision_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Ausmachender Betrag 100,01- EUR"
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)-?\s*EUR").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Schlusstag 14.01.2021"
        let date_re = Regex::new(r"Schlusstag(?:/-Zeit)?\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                currency,
                note: Some("MLP Bank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Ausschüttung") && !content.contains("Dividende") {
            return transactions;
        }

        // Extract ISIN and WKN
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), extract_wkn(content)));

        // Extract security name
        let name_re = Regex::new(r"Stück\s+[\d.,]+\s+([A-Z][^\n]+?)(?:\s+[A-Z]{2}[A-Z0-9]{10})").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross amount - "Ausschüttung 8,39 USD 7,55+ EUR"
        let gross_re = Regex::new(r"Ausschüttung\s+([\d.,]+)\s*([A-Z]{3})(?:\s+[\d.,]+\+?\s*EUR)?").ok();
        let (gross_amount, forex_currency) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (parse_german_decimal(&c[1]).unwrap_or(0.0), Some(c[2].to_string())))
            .unwrap_or((0.0, None));

        // Extract net amount in EUR - "Ausmachender Betrag 7,55+ EUR"
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount);

        // Extract exchange rate - "Devisenkurs EUR / USD 1,1107"
        let fx_re = Regex::new(r"Devisenkurs\s+EUR\s*/\s*[A-Z]{3}\s+([\d.,]+)").ok();
        let exchange_rate = fx_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract date - "Zahlbarkeitstag 20.11.2019"
        let date_re = Regex::new(r"Zahlbarkeitstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                security_name,
                isin,
                wkn,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("MLP Bank Ausschüttung".to_string()),
                exchange_rate,
                forex_currency,
            });
        }

        transactions
    }
}

impl BankParser for MlpBankParser {
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
        "MLP Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = MlpBankParser::new();
        assert!(parser.detect("MLP Banking AG"));
        assert!(parser.detect("MLPBDE61XXX"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = MlpBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
MLP Banking AG · Alte Heerstraße 40 · 69168 Wiesloch
Wertpapier Abrechnung Kauf
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 4,929 SAUREN GLOBAL BALANCED LU0106280836 (930920)
INHABER-ANTEILE A O.N
Schlusstag 14.01.2021
Ausführungskurs 20,29 EUR
Kurswert 100,01- EUR
Ausmachender Betrag 100,01- EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("LU0106280836".to_string()));
        assert_eq!(txns[0].wkn, Some("930920".to_string()));
    }
}
