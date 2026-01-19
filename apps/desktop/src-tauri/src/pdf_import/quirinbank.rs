//! Quirin Privatbank PDF Parser
//!
//! Parses broker statements from quirin bank AG (Germany).

use super::{
    extract_isin, extract_wkn, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct QuirinBankParser {
    detect_patterns: Vec<&'static str>,
}

impl QuirinBankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "quirin bank",
                "quirinprivatbank",
                "Kurfürstendamm 119",
                "10711 Berlin",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Wertpapierabrechnung" + "Kauf" or "Verkauf"
        let has_abrechnung = content.contains("Wertpapierabrechnung");
        let is_buy = has_abrechnung && content.contains("\nKauf\n");
        let is_sell = has_abrechnung && content.contains("\nVerkauf\n");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN - "ISIN LU0690964092"
        let isin_re = Regex::new(r"ISIN\s+([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract WKN - "WKN DBX0MF"
        let wkn_re = Regex::new(r"WKN\s+([A-Z0-9]{6})").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_wkn(content));

        // Extract security name - "Wertpapierbezeichnung db x-tr.II Gl Sovereign ETF..."
        let name_re = Regex::new(r"Wertpapierbezeichnung\s+([^\n]+)").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].trim().to_string());

        // Extract shares - "Nominal / Stück 140,0000 ST"
        let shares_re = Regex::new(r"Nominal / Stück\s+([\d.,]+)\s*ST").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract price - "Kurs EUR 214,899"
        let price_re = Regex::new(r"Kurs\s+([A-Z]{3})\s+([\d.,]+)").ok();
        let (currency, price_per_share) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), parse_german_decimal(&c[2])))
            .unwrap_or(("EUR".to_string(), None));

        // Extract gross amount - "Kurswert EUR - 30.085,86"
        let kurswert_re = Regex::new(r"Kurswert\s+EUR\s+-?\s*([\d.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract fees - "Abwicklungsgebühren * EUR - 4,90"
        let fees_re = Regex::new(r"Abwicklungsgebühren[^E]*EUR\s+-?\s*([\d.,]+)").ok();
        let fees = fees_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract total - "Ausmachender Betrag EUR - 30.090,76"
        let total_re = Regex::new(r"Ausmachender Betrag\s+EUR\s+-?\s*([\d.,]+)").ok();
        let net_amount = total_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount + fees);

        // Extract date - "Handelstag / Zeit 30.12.2016"
        let date_re = Regex::new(r"Handelstag / Zeit\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                note: Some("Quirin Bank Import".to_string()),
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
        let isin_re = Regex::new(r"ISIN\s+([A-Z]{2}[A-Z0-9]{10})").ok();
        let isin = isin_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_isin(content));

        // Extract WKN
        let wkn_re = Regex::new(r"WKN\s+([A-Z0-9]{6})").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| extract_wkn(content));

        // Extract shares
        let shares_re = Regex::new(r"Nominal / Stück\s+([\d.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]));

        // Extract gross amount
        let gross_re = Regex::new(r"(?:Brutto|Ausschüttung)\s+EUR\s+([\d.,]+)").ok();
        let gross_amount = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract taxes
        let tax_re = Regex::new(r"Kapitalertragsteuer\s+EUR\s+-?([\d.,]+)").ok();
        let taxes = tax_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(0.0);

        // Extract net amount
        let net_re = Regex::new(r"Ausmachender Betrag\s+EUR\s+([\d.,]+)").ok();
        let net_amount = net_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| parse_german_decimal(&c[1]))
            .unwrap_or(gross_amount - taxes);

        // Extract date
        let date_re = Regex::new(r"(?:Valuta|Zahlbar)\s+(\d{2}\.\d{2}\.\d{4})").ok();
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
                wkn,
                shares,
                price_per_share: None,
                gross_amount,
                fees: 0.0,
                taxes,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Quirin Bank Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for QuirinBankParser {
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
        "Quirin Privatbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = QuirinBankParser::new();
        assert!(parser.detect("quirin bank AG"));
        assert!(parser.detect("quirinprivatbank.de"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = QuirinBankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
quirin bank AG
Wertpapierabrechnung
Kauf
Wertpapierbezeichnung db x-tr.II Gl Sovereign ETF Inhaber-Anteile 1D EUR o.N.
ISIN LU0690964092
WKN DBX0MF
Handelstag / Zeit 30.12.2016 12:46:28
Nominal / Stück 140,0000 ST
Kurs EUR 214,899
Kurswert EUR - 30.085,86
Abwicklungsgebühren * EUR - 4,90
Ausmachender Betrag EUR - 30.090,76
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("LU0690964092".to_string()));
    }
}
