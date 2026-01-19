//! S-Broker (Sparkassen Broker) PDF Parser
//!
//! Parses broker statements from S-Broker AG & Co. KG.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct SBrokerParser {
    detect_patterns: Vec<&'static str>,
}

impl SBrokerParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "S Broker",
                "S-Broker",
                "sBroker",
                "Sparkassen Broker",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Split by "Wertpapierabrechnung"
        let sections: Vec<&str> = content.split("Wertpapierabrechnung").collect();

        for section in sections.iter().skip(1) {
            let is_buy = section.contains("Kauf") && !section.contains("Verkauf");
            let is_sell = section.contains("Verkauf");

            if !is_buy && !is_sell {
                continue;
            }

            let txn_type = if is_buy {
                ParsedTransactionType::Buy
            } else {
                ParsedTransactionType::Sell
            };

            // Extract ISIN
            let isin = extract_isin(section);

            // Extract security name
            let name_re = Regex::new(r"Gattungsbezeichnung\s+ISIN\n([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
            let security_name = name_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .map(|c| c[1].trim().to_string());

            // Extract shares
            let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
            let shares = shares_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            // Extract price
            let price_re = Regex::new(r"STK\s+[\d.,]+\s+EUR\s+([\d.,]+)").ok();
            let price_per_share = price_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            // Extract Kurswert
            let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
            let gross_amount = kurswert_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(0.0);

            // Extract fees
            let order_re = Regex::new(r"Orderentgelt\s+EUR\s+([\d.,]+)").ok();
            let boersen_re = Regex::new(r"Börsengebühr\s+EUR\s+([\d.,]+)").ok();

            let mut fees = 0.0;
            if let Some(re) = &order_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &boersen_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract date
            let date_re = Regex::new(r"Handelstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let date = date_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_date(&c[1]))
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

            // Extract total
            let total_re = Regex::new(r"Betrag zu Ihren (?:Lasten|Gunsten)\n[^\n]+EUR\s+([\d.,]+)").ok();
            let net_amount = total_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
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
                    currency: "EUR".to_string(),
                    note: Some("S-Broker Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                });
            }
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividendengutschrift") && !content.contains("Ertragsgutschrift") {
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

        let date_re = Regex::new(r"Zahltag\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("S-Broker Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for SBrokerParser {
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
        "S-Broker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = SBrokerParser::new();
        assert!(parser.detect("S Broker AG & Co. KG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = SBrokerParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Wertpapierabrechnung
Kauf
Gattungsbezeichnung ISIN
iS.EO G.B.C.1.5-10.5y.U.ETF DE Inhaber-Anteile DE000A0H0785
Nominal Kurs
STK 16,000 EUR 120,4000
Handelstag 29.09.2014 Kurswert EUR 1.926,40-
Orderentgelt EUR 1,48-
Börsengebühr EUR 2,29-
Betrag zu Ihren Lasten
01.10.2014 10/0000/000 EUR 1.930,17
S Broker AG & Co. KG
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert!((txns[0].shares.unwrap() - 16.0).abs() < 0.001);
    }
}
