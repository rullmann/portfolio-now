//! Onvista Bank PDF Parser
//!
//! Parses broker statements from onvista bank (now part of comdirect).

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct OnvistaParser {
    detect_patterns: Vec<&'static str>,
}

impl OnvistaParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "onvista bank",
                "onvista-bank.de",
                "Wildunger Straße 6a",
                "60487 Frankfurt",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Split by "Wertpapierabrechnung" to handle multiple transactions
        let sections: Vec<&str> = content.split("Wertpapierabrechnung").collect();

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

            // Extract ISIN - on same line as security name
            let isin = extract_isin(section);

            // Extract security name - "Gattungsbezeichnung ISIN\nDWS Deutschland ... DE0008490962"
            let name_re = Regex::new(r"Gattungsbezeichnung\s+ISIN\n([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
            let security_name = name_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .map(|c| c[1].trim().to_string());

            // Extract shares - "STK 0,7445" or "STK 20,000"
            let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
            let shares = shares_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]));

            // Extract price - "EUR 200,1500" after Nominal/Kurs
            let price_re = Regex::new(r"(?:Nominal|Kurs)\n[^\n]*\nSTK[^\n]+EUR\s+([\d.,]+)").ok();
            let alt_price_re = Regex::new(r"STK\s+[\d.,]+\s+EUR\s+([\d.,]+)").ok();
            let price_per_share = price_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .or_else(|| {
                    alt_price_re
                        .as_ref()
                        .and_then(|re| re.captures(section))
                        .and_then(|c| parse_german_decimal(&c[1]))
                });

            // Extract Kurswert (gross amount) - "Kurswert EUR 149,01-"
            let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
            let gross_amount = kurswert_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(0.0);

            // Extract fees
            let provision_re = Regex::new(r"Orderprovision\s+EUR\s+([\d.,]+)").ok();
            let handelsplatz_re = Regex::new(r"Handelsplatzgebühr\s+EUR\s+([\d.,]+)").ok();

            let mut fees = 0.0;
            if let Some(re) = &provision_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &handelsplatz_re {
                if let Some(caps) = re.captures(section) {
                    fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract taxes
            let kest_re = Regex::new(r"Kapitalertragsteuer\s+EUR\s+([\d.,]+)").ok();
            let soli_re = Regex::new(r"Solidaritätszuschlag\s+EUR\s+([\d.,]+)").ok();

            let mut taxes = 0.0;
            if let Some(re) = &kest_re {
                if let Some(caps) = re.captures(section) {
                    taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }
            if let Some(re) = &soli_re {
                if let Some(caps) = re.captures(section) {
                    taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract date - "Handelstag 15.08.2019"
            let date_re = Regex::new(r"Handelstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let date = date_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_date(&c[1]))
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

            // Extract total - "Betrag zu Ihren Lasten EUR 150,01" or "Betrag zu Ihren Gunsten"
            let total_re = Regex::new(r"Betrag zu Ihren (?:Lasten|Gunsten)\n[^\n]+EUR\s+([\d.,]+)").ok();
            let net_amount = total_re
                .as_ref()
                .and_then(|re| re.captures(section))
                .and_then(|c| parse_german_decimal(&c[1]))
                .unwrap_or(gross_amount + fees + taxes);

            if gross_amount > 0.0 || shares.is_some() {
                let txn = ParsedTransaction {
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
                    taxes,
                    net_amount,
                    currency: "EUR".to_string(),
                    note: Some("Onvista Bank Import".to_string()),
                    exchange_rate: None,
                    forex_currency: None,
                };
                transactions.push(txn);
            }
        }

        if transactions.is_empty() && (content.contains("Kauf") || content.contains("Verkauf")) {
            ctx.warn("transaction", content, "Keine Transaktionen gefunden");
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Check for dividend document
        if !content.contains("Dividendengutschrift")
            && !content.contains("Erträgnisgutschrift")
            && !content.contains("Dividende für")
        {
            return transactions;
        }

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract security name
        let name_re = Regex::new(r"Gattungsbezeichnung\s+ISIN\n([^\n]+?)\s+[A-Z]{2}[A-Z0-9]{10}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "STK 50,000"
        let shares_re = Regex::new(r"STK\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract dividend amount - "Dividenden-Betrag pro Stück EUR 0,200000"
        let div_per_share_re = Regex::new(r"(?:Dividenden-Betrag|Dividende)\s+pro\s+Stück\s+EUR\s+([\d.,]+)").ok();
        let div_per_share = div_per_share_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Calculate gross from shares * div per share, or extract directly
        let gross_re = Regex::new(r"Betrag zu Ihren Gunsten\n[^\n]+EUR\s+([\d.,]+)").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .or_else(|| {
                shares.and_then(|s| div_per_share.map(|d| s * d))
            })
            .unwrap_or(0.0);

        // Extract date - "Zahltag 21.04.2016"
        let date_re = Regex::new(r"Zahltag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // For onvista, many dividends are tax-free from Einlagenkonto
        let net_amount = gross_amount;

        if gross_amount > 0.0 {
            let txn = ParsedTransaction {
                date,
                time: None,
                txn_type: ParsedTransactionType::Dividend,
                security_name,
                isin,
                wkn: None,
                shares,
                price_per_share: div_per_share,
                gross_amount,
                fees: 0.0,
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Onvista Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            };
            transactions.push(txn);
        } else {
            ctx.warn("dividend", content, "Dividende konnte nicht vollständig geparst werden");
        }

        transactions
    }
}

impl BankParser for OnvistaParser {
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
        "Onvista Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = OnvistaParser::new();
        assert!(parser.detect("onvista bank Wildunger Straße 6a"));
        assert!(parser.detect("www.onvista-bank.de"));
        assert!(!parser.detect("comdirect bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = OnvistaParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
onvista bank Wildunger Straße 6a 60487 Frankfurt
Wertpapierabrechnung
Kauf Sparplan
Gattungsbezeichnung ISIN
DWS Deutschland Inhaber-Anteile LC DE0008490962
Nominal Kurs
STK 0,7445 EUR 200,1500
Handelstag 15.08.2019 Kurswert EUR 149,01-
Orderprovision EUR 1,00-
Betrag zu Ihren Lasten
19.08.2019 123450042 EUR 150,01
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Buy);
        assert_eq!(txn.isin, Some("DE0008490962".to_string()));
        assert!((txn.shares.unwrap() - 0.7445).abs() < 0.0001);
        assert!((txn.gross_amount - 149.01).abs() < 0.01);
        assert!((txn.fees - 1.00).abs() < 0.01);
    }

    #[test]
    fn test_parse_sell() {
        let parser = OnvistaParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
onvista bank
Wertpapierabrechnung
Verkauf
Gattungsbezeichnung ISIN
adesso AG Inhaber-Aktien o.N. DE000A0Z23Q5
Nominal Kurs
STK 20,000 EUR 31,5000
Handelstag 02.09.2016 Kurswert EUR 630,00
Orderprovision EUR 5,00-
Kapitalertragsteuer EUR 1,43-
Solidaritätszuschlag EUR 0,08-
Betrag zu Ihren Gunsten
06.09.2016 243921041 EUR 623,49
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Sell);
        assert_eq!(txn.isin, Some("DE000A0Z23Q5".to_string()));
        assert!((txn.shares.unwrap() - 20.0).abs() < 0.001);
        assert!((txn.gross_amount - 630.0).abs() < 0.01);
        assert!((txn.taxes - 1.51).abs() < 0.01);
    }

    #[test]
    fn test_parse_dividend() {
        let parser = OnvistaParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
onvista bank
Erträgnisgutschrift aus Wertpapieren
Dividende für
Gattungsbezeichnung ISIN
Commerzbank AG Inhaber-Aktien o.N. DE000CBK1001
Nominal Ex-Tag Zahltag Dividenden-Betrag pro Stück
STK 50,000 21.04.2016 21.04.2016 EUR 0,200000
Betrag zu Ihren Gunsten
21.04.2016 172306238 EUR 10,00
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txn.isin, Some("DE000CBK1001".to_string()));
        assert!((txn.shares.unwrap() - 50.0).abs() < 0.001);
        assert!((txn.gross_amount - 10.0).abs() < 0.01);
    }
}
