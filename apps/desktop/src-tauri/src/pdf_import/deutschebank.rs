//! Deutsche Bank PDF Parser
//!
//! Parses broker statements from Deutsche Bank Privat- und Geschäftskunden AG.

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct DeutscheBankParser {
    detect_patterns: Vec<&'static str>,
}

impl DeutscheBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Deutsche Bank Privat- und Geschäftskunden",
                "Deutsche Bank AG",
                "deutsche-bank.de",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect transaction type
        let is_buy = content.contains("Kauf von Wertpapieren")
            || content.contains("Abrechnung: Kauf");
        let is_sell = content.contains("Verkauf von Wertpapieren")
            || content.contains("Abrechnung: Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy {
            ParsedTransactionType::Buy
        } else {
            ParsedTransactionType::Sell
        };

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract WKN - "WKN BASF11"
        let wkn_re = Regex::new(r"WKN\s+([A-Z0-9]{6})").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_wkn(content));

        // Extract security name - line with Filialnummer contains name
        let name_re = Regex::new(r"Filialnummer[^\n]+\n[^\n]*(\d+\s+\d+\s+\d+\s+)([A-Za-z][^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[2].trim().to_string());

        // Extract shares - "Nominal ST 19" or "Nominal STK 19"
        let shares_re = Regex::new(r"Nominal\s+(?:ST|STK|Stück)\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs EUR 35,00"
        let price_re = Regex::new(r"Kurs\s+EUR\s+([\d.,]+)").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract Kurswert (gross amount)
        let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees (Provision + other fees)
        let provision_re = Regex::new(r"Provision\s+EUR\s+([\d.,]+)").ok();
        let weitere_re = Regex::new(r"Weitere Provision[^\n]*EUR\s+([\d.,]+)").ok();
        let xetra_re = Regex::new(r"XETRA-Kosten\s+EUR\s+([\d.,]+)").ok();

        let mut fees = 0.0;
        if let Some(re) = &provision_re {
            if let Some(caps) = re.captures(content) {
                fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &weitere_re {
            if let Some(caps) = re.captures(content) {
                fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &xetra_re {
            if let Some(caps) = re.captures(content) {
                fees += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract date - "Schlusstag/-zeit MEZ 02.04.2015"
        let date_re = Regex::new(r"Schlusstag(?:/\-zeit)?\s+(?:MEZ\s+)?(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_date(&c[1]))
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Buchung auf Kontonummer ... EUR 675,50"
        let total_re = Regex::new(r"(?:Buchung auf Kontonummer|Belastung)[^\n]*EUR\s+([\d.,]+)").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
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
                note: Some("Deutsche Bank Import".to_string()),
                exchange_rate: None,
                forex_currency: None,
            };
            transactions.push(txn);
        } else {
            ctx.warn("transaction", content, "Konnte Transaktion nicht vollständig parsen");
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Check for dividend document
        if !content.contains("Dividendengutschrift") && !content.contains("Ertragsgutschrift") {
            return transactions;
        }

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract WKN
        let wkn_re = Regex::new(r"(\d{6})\s+[A-Z]{2}[A-Z0-9]{10}").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_wkn(content));

        // Extract security name - line after WKN/ISIN
        let name_re = Regex::new(r"[A-Z]{2}[A-Z0-9]{10}\n([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Stück" or "380,000000"
        let shares_re = Regex::new(r"(\d+[.,]\d+)\s+\d{6}\s+[A-Z]{2}").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross amount - "Bruttoertrag 98,80 USD 87,13 EUR"
        let gross_re = Regex::new(r"Bruttoertrag[^\n]*?([\d.,]+)\s+EUR").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract taxes
        let kest_re = Regex::new(r"Kapitalertragsteuer[^\n]*?([\d.,]+)\s+EUR").ok();
        let soli_re = Regex::new(r"Solidaritätszuschlag[^\n]*?([\d.,]+)\s+EUR").ok();

        let mut taxes = 0.0;
        if let Some(re) = &kest_re {
            if let Some(caps) = re.captures(content) {
                taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &soli_re {
            if let Some(caps) = re.captures(content) {
                taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract exchange rate
        let fx_re = Regex::new(r"Umrechnungskurs\s+[A-Z]+\s+zu\s+EUR\s+([\d.,]+)").ok();
        let exchange_rate = fx_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract forex currency
        let forex_re = Regex::new(r"Bruttoertrag\s+([\d.,]+)\s+([A-Z]{3})").ok();
        let forex_currency = forex_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[2].to_string())
            .filter(|c| c != "EUR");

        // Extract net amount - "Gutschrift mit Wert ... EUR"
        let net_re = Regex::new(r"Gutschrift mit Wert[^\n]*([\d.,]+)\s+EUR").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

        // Extract date - "Zahlbar 15.12.2014"
        let date_re = Regex::new(r"Zahlbar\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("Deutsche Bank Dividende".to_string()),
                exchange_rate,
                forex_currency,
            };
            transactions.push(txn);
        } else {
            ctx.warn("dividend", content, "Dividende konnte nicht vollständig geparst werden");
        }

        transactions
    }
}

impl BankParser for DeutscheBankParser {
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
        "Deutsche Bank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = DeutscheBankParser::new();
        assert!(parser.detect("Deutsche Bank Privat- und Geschäftskunden AG"));
        assert!(parser.detect("www.deutsche-bank.de"));
        assert!(!parser.detect("Commerzbank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = DeutscheBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Deutsche Bank Privat- und Geschäftskunden AG
Abrechnung: Kauf von Wertpapieren
Filialnummer Depotnummer Wertpapierbezeichnung
123 1234567 00 BASF SE
WKN BASF11 Nominal ST 19
ISIN DE000BASF111 Kurs EUR 35,00
Schlusstag/-zeit MEZ 02.04.2015 / 09:04
Kurswert EUR 665,00
Provision EUR 7,90
Buchung auf Kontonummer 1234567 EUR 675,50
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Buy);
        assert_eq!(txn.wkn, Some("BASF11".to_string()));
        assert_eq!(txn.isin, Some("DE000BASF111".to_string()));
        assert!((txn.shares.unwrap() - 19.0).abs() < 0.001);
        assert!((txn.gross_amount - 665.0).abs() < 0.01);
        assert!((txn.fees - 7.90).abs() < 0.01);
    }

    #[test]
    fn test_parse_dividend() {
        let parser = DeutscheBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Deutsche Bank Privat- und Geschäftskunden AG
Dividendengutschrift
Stück WKN ISIN
380,000000 878841 US17275R1023
CISCO SYSTEMS INC.
Bruttoertrag 98,80 USD 87,13 EUR
Kapitalertragsteuer (KESt) 8,71 EUR
Solidaritätszuschlag 0,47 EUR
Umrechnungskurs USD zu EUR 1,1339000000
Gutschrift mit Wert 15.12.2014 64,88 EUR
Zahlbar 15.12.2014
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txn.isin, Some("US17275R1023".to_string()));
        assert!((txn.gross_amount - 87.13).abs() < 0.01);
        assert!((txn.taxes - 9.18).abs() < 0.01); // KESt + Soli
    }
}
