//! Postbank PDF Parser
//!
//! Parses broker statements from Deutsche Postbank AG.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct PostbankParser {
    detect_patterns: Vec<&'static str>,
}

impl PostbankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Postbank",
                "Deutsche Postbank AG",
                "51222 Köln",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Split by "Wertpapier Abrechnung" to handle multiple transactions
        let sections: Vec<&str> = content.split("Wertpapier Abrechnung").collect();

        for section in sections.iter().skip(1) {
            // Detect transaction type
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

            // Extract ISIN and WKN - "IE00BJ0KDQ92 (A1XB5U)"
            let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
            let (isin, wkn) = isin_wkn_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
                .unwrap_or_else(|| (extract_isin(section), None));

            // Extract security name - between "Stück" line and ISIN
            let name_re = Regex::new(r"Stück\s+\d+\s+([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
            let security_name = name_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .map(|c| c[1].trim().to_string());

            // Extract shares - "Stück 158"
            let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
            let shares = shares_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            // Extract price - "Ausführungskurs 62,821 EUR"
            let price_re = Regex::new(r"Ausführungskurs\s+([\d.,]+)\s*EUR").ok();
            let price_per_share = price_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            // Extract Kurswert (gross amount) - "Kurswert 9.925,72- EUR"
            let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)-?\s*EUR").ok();
            let gross_amount = kurswert_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(0.0);

            // Extract fees
            let provision_re = Regex::new(r"Provision\s+([\d.,]+)-?\s*EUR").ok();
            let abwicklung_re = Regex::new(r"Abwicklungskosten[^\n]*([\d.,]+)-?\s*EUR").ok();
            let transaktions_re = Regex::new(r"Transaktionsentgelt[^\n]*([\d.,]+)-?\s*EUR").ok();
            let liefer_re = Regex::new(r"(?:Übertragungs|Liefer)(?:-|/)gebühr\s+([\d.,]+)-?\s*EUR").ok();

            let mut fees = 0.0;
            if let Some(re) = &provision_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &abwicklung_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &transaktions_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &liefer_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract date - "Schlusstag/-Zeit 04.02.2020"
            let date_re = Regex::new(r"Schlusstag(?:/\-Zeit)?\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let date = date_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_date(&c[1]))
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

            // Extract total - "Ausmachender Betrag 9.978,18- EUR"
            let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)-?\s*EUR").ok();
            let net_amount = total_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(gross_amount + fees);

            if gross_amount > 0.0 || shares.is_some() {
                let txn = ParsedTransaction {
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
                    note: Some("Postbank Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                };
                transactions.push(txn);
            }
        }

        if transactions.is_empty() {
            ctx.warn("transaction", content, "Keine Transaktionen gefunden");
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Check for dividend document
        if !content.contains("Dividendengutschrift") && !content.contains("Ertragsgutschrift") {
            return transactions;
        }

        // Extract ISIN and WKN
        let isin_wkn_re = Regex::new(r"([A-Z]{2}[A-Z0-9]{10})\s*\(([A-Z0-9]{6})\)").ok();
        let (isin, wkn) = isin_wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Some(c[1].to_string()), Some(c[2].to_string())))
            .unwrap_or_else(|| (extract_isin(content), None));

        // Extract security name
        let name_re = Regex::new(r"Stück\s+\d+\s+([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück 12"
        let shares_re = Regex::new(r"Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract dividend info - "Dividendengutschrift 12,12 USD 10,17+ EUR"
        let dividend_re = Regex::new(r"Dividendengutschrift\s+([\d.,]+)\s+([A-Z]{3})\s+([\d.,]+)\+?\s*EUR").ok();
        let (_forex_amount, forex_currency, gross_eur) = dividend_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                let fx_amt = parse_german_decimal(&c[1]);
                let fx_cur = c[2].to_string();
                let eur = parse_german_decimal(&c[3]).unwrap_or(0.0);
                (fx_amt, Some(fx_cur), eur)
            })
            .unwrap_or((None, None, 0.0));

        // If no forex, try direct EUR pattern
        let gross_amount = if gross_eur > 0.0 {
            gross_eur
        } else {
            let eur_re = Regex::new(r"Dividendengutschrift[^\n]*([\d.,]+)\+?\s*EUR").ok();
            eur_re
                .as_ref()
                .and_then(|re| re.captures(content))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(0.0)
        };

        // Extract exchange rate - "Devisenkurs EUR / USD  1,1920"
        let fx_re = Regex::new(r"Devisenkurs\s+EUR\s*/\s*[A-Z]{3}\s+([\d.,]+)").ok();
        let exchange_rate = fx_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract taxes - "Einbehaltene Quellensteuer 15 % auf 12,12 USD 1,53- EUR"
        // The pattern needs to match the EUR amount at the end - look for last number before EUR on line
        let tax_re = Regex::new(r"Einbehaltene Quellensteuer[^\n]*\s([\d.,]+)-?\s*EUR").ok();
        let taxes = tax_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract net amount - "Ausmachender Betrag 8,64+ EUR"
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\+?\s*EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

        // Extract date - "Zahlbarkeitstag 09.03.2021"
        let date_re = Regex::new(r"Zahlbarkeitstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        if gross_amount > 0.0 {
            let txn = ParsedTransaction {
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
                taxes,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Postbank Dividende".to_string()),
                exchange_rate,
                forex_currency: forex_currency.filter(|c| c != "EUR"),
            };
            transactions.push(txn);
        } else {
            ctx.warn("dividend", content, "Dividende konnte nicht vollständig geparst werden");
        }

        transactions
    }
}

impl BankParser for PostbankParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        // Try parsing as buy/sell
        let buy_sell = self.parse_buy_sell(content, ctx);
        if !buy_sell.is_empty() {
            transactions.extend(buy_sell);
        }

        // Try parsing as dividend
        let dividends = self.parse_dividends(content, ctx);
        if !dividends.is_empty() {
            transactions.extend(dividends);
        }

        // Sort by date
        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Postbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = PostbankParser::new();
        assert!(parser.detect("Deutsche Postbank AG"));
        assert!(parser.detect("Postbank Köln · 51222 Köln"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = PostbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Postbank Köln · 51222 Köln
Wertpapier Abrechnung Kauf
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 158 XTR.(IE) - MSCI WORLD IE00BJ0KDQ92 (A1XB5U)
Schlusstag/-Zeit 04.02.2020 08:00:04
Ausführungskurs 62,821 EUR
Kurswert 9.925,72- EUR
Provision 39,95- EUR
Ausmachender Betrag 9.978,18- EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Buy);
        assert_eq!(txn.isin, Some("IE00BJ0KDQ92".to_string()));
        assert_eq!(txn.wkn, Some("A1XB5U".to_string()));
        assert!((txn.shares.unwrap() - 158.0).abs() < 0.001);
        assert!((txn.gross_amount - 9925.72).abs() < 0.01);
    }

    #[test]
    fn test_parse_dividend() {
        let parser = PostbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Postbank Köln · 51222 Köln
Dividendengutschrift
Nominale Wertpapierbezeichnung ISIN (WKN)
Stück 12 JOHNSON & JOHNSON US4781601046 (853260)
Zahlbarkeitstag 09.03.2021
Devisenkurs EUR / USD  1,1920
Dividendengutschrift 12,12 USD 10,17+ EUR
Einbehaltene Quellensteuer 15 % auf 12,12 USD 1,53- EUR
Ausmachender Betrag 8,64+ EUR
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txn.isin, Some("US4781601046".to_string()));
        assert!((txn.shares.unwrap() - 12.0).abs() < 0.001);
        assert!((txn.gross_amount - 10.17).abs() < 0.01);
        assert!((txn.taxes - 1.53).abs() < 0.01);
    }
}
