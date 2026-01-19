//! DAB BNP Paribas PDF Parser
//!
//! Parses broker statements from DAB Bank AG (now part of BNP Paribas).

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct DabParser {
    detect_patterns: Vec<&'static str>,
}

impl DabParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "DAB Bank",
                "DAB BNP",
                "DABBDEMMXXX",
                "80687 München",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        let sections: Vec<&str> = content.split("Wertpapierabrechnung").collect();

        for section in sections.iter().skip(1) {
            let is_buy = section.contains("Kauf") && !section.contains("Verkauf");
            let is_sell = section.contains("Verkauf");

            if !is_buy && !is_sell {
                continue;
            }

            let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

            let isin = extract_isin(section);

            let name_re = Regex::new(r"Gattungsbezeichnung\s+ISIN\n([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
            let security_name = name_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .map(|c| c[1].trim().to_string());

            let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
            let shares = shares_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            let price_re = Regex::new(r"STK\s+[\d.,]+\s+EUR\s+([\d.,]+)").ok();
            let price_per_share = price_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
            let gross_amount = kurswert_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(0.0);

            let date_re = Regex::new(r"Handelstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let date = date_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_date(&c[1]))
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

            let total_re = Regex::new(r"Betrag zu Ihren (?:Lasten|Gunsten)\n[^\n]+EUR\s+([\d.,]+)").ok();
            let net_amount = total_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(gross_amount);

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
                    net_amount,
                    currency: "EUR".to_string(),
                    note: Some("DAB Bank Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                });
            }
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende") && !content.contains("Ertrag") {
            return transactions;
        }

        let isin = extract_isin(content);

        let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        let gross_re = Regex::new(r"Betrag zu Ihren Gunsten\n[^\n]+EUR\s+([\d.,]+)").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        let date_re = Regex::new(r"(?:Zahltag|Valuta)\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                wkn: None,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount: gross_amount,
                currency: "EUR".to_string(),
                note: Some("DAB Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for DabParser {
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
        "DAB Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = DabParser::new();
        assert!(parser.detect("DAB Bank AG"));
        assert!(parser.detect("BIC (SWIFT-Code): DABBDEMMXXX"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = DabParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
DAB Bank AG
Wertpapierabrechnung
Kauf
Gattungsbezeichnung ISIN
ARERO - Der Weltfonds Inhaber-Anteile o.N. LU0360863863
Nominal Kurs
STK 0,9192 EUR 163,1900
Handelstag 06.01.2015 Kurswert EUR 150,00-
Betrag zu Ihren Lasten
08.01.2015 8022574001 EUR 150,00
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("LU0360863863".to_string()));
    }
}
