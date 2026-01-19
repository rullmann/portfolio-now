//! Credit Suisse PDF Parser
//!
//! Parses broker statements from Credit Suisse (Schweiz) AG.

use super::{
    extract_isin, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;
use chrono::NaiveDate;

pub struct CreditSuisseParser {
    detect_patterns: Vec<&'static str>,
}

impl CreditSuisseParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "CREDIT SUISSE",
                "Credit Suisse",
                "CRESCHZZ",
                "credit-suisse.com",
                "CH-8070 Zürich",
            ],
        }
    }

    /// Parse Swiss date format (DD.MM.YYYY)
    fn parse_date(s: &str) -> Option<NaiveDate> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let day: u32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let year: i32 = parts[2].parse().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)
    }

    /// Parse number with comma as thousands separator
    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim()
            .replace(',', "")
            .replace('\'', "");
        cleaned.parse::<f64>().ok()
    }

    fn parse_buy_sell(&self, content: &str, _ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect type from "Ihr Kauf" or "Ihr Verkauf"
        let is_buy = content.contains("Ihr Kauf") && !content.contains("Ihr Verkauf");
        let is_sell = content.contains("Ihr Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy { ParsedTransactionType::Buy } else { ParsedTransactionType::Sell };

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract Valor
        let valor_re = Regex::new(r"Valor\s+(\d+)").ok();
        let _valor = valor_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string());

        // Extract security name - "900 Registered Shs Iron Mountain Inc USD 0.01"
        let name_re = Regex::new(r"(\d+)\s+(Registered Shs|Namen-Aktien|Bearer Shs)\s+([^\n]+?)(?:\s+[A-Z]{3}\s+[\d.]+)?$").ok();
        let (shares_from_name, security_name) = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (Self::parse_number(&c[1]), Some(c[3].trim().to_string())))
            .unwrap_or((None, None));

        // Extract price - "zum Kurs von USD 30.30"
        let price_re = Regex::new(r"zum Kurs von\s+([A-Z]{3})\s+([\d.,]+)").ok();
        let (currency, price_per_share) = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2])))
            .unwrap_or(("USD".to_string(), None));

        // Extract Kurswert - "Kurswert USD 27,270.00"
        let kurswert_re = Regex::new(r"Kurswert\s+[A-Z]{3}\s+([\d,.']+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]))
            .unwrap_or(0.0);

        // Extract shares (fallback)
        let shares = shares_from_name.or_else(|| {
            match (gross_amount, price_per_share) {
                (g, Some(p)) if g > 0.0 && p > 0.0 => Some(g / p),
                _ => None,
            }
        });

        // Extract fees
        let commission_re = Regex::new(r"Kommission[^\d]+([\d,.']+)").ok();
        let kosten_re = Regex::new(r"Kosten und Abgaben[^\d]+([\d,.']+)").ok();
        let umsatz_re = Regex::new(r"Umsatzabgabe[^\d]+([\d,.']+)").ok();

        let mut fees = 0.0;
        for re_opt in [&commission_re, &kosten_re, &umsatz_re] {
            if let Some(re) = re_opt {
                if let Some(caps) = re.captures(content) {
                    fees += Self::parse_number(&caps[1]).unwrap_or(0.0);
                }
            }
        }

        // Subtract internet discount
        let discount_re = Regex::new(r"Internet-Vergünstigung[^\d]+([\d,.']+)").ok();
        if let Some(re) = &discount_re {
            if let Some(caps) = re.captures(content) {
                fees -= Self::parse_number(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract date - "Datum 08.06.2020"
        let date_re = Regex::new(r"Datum\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Extract total - "Belastung USD 27,734.70"
        let total_re = Regex::new(r"(?:Belastung|Gutschrift)\s+[A-Z]{3}\s+([\d,.']+)").ok();
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
                wkn: None,
                shares,
                price_per_share,
                gross_amount,
                fees,
                taxes: 0.0,
                net_amount,
                currency,
                note: Some("Credit Suisse Import".to_string()),
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

        let shares_re = Regex::new(r"(\d+)\s+(?:Registered Shs|Namen-Aktien)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_number(&c[1]));

        let gross_re = Regex::new(r"Brutto\s+([A-Z]{3})\s+([\d,.']+)").ok();
        let (currency, gross_amount) = gross_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| (c[1].to_string(), Self::parse_number(&c[2]).unwrap_or(0.0)))
            .unwrap_or(("CHF".to_string(), 0.0));

        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_date(&c[1]))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

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
                currency,
                note: Some("Credit Suisse Dividende".to_string()),
                exchange_rate: None,
                forex_currency: None,
            });
        }

        transactions
    }
}

impl BankParser for CreditSuisseParser {
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
        "Credit Suisse"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = CreditSuisseParser::new();
        assert!(parser.detect("CREDIT SUISSE (Schweiz) AG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_buy() {
        let parser = CreditSuisseParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
CREDIT SUISSE (Schweiz) AG
Wertschriftenabrechnung 08.06.2020
Ihr Kauf
Datum 08.06.2020
900 Registered Shs Iron Mountain Inc USD 0.01
Valor 26754105, IRM, ISIN US46284V1017
zum Kurs von USD 30.30
Kurswert USD 27,270.00
Kommission Schweiz/Ausland USD 463.60
Kosten und Abgaben Ausland USD 2.00
Eidgenössische Umsatzabgabe USD 40.91
Internet-Vergünstigung USD - 41.81
Belastung USD 27,734.70
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].txn_type, ParsedTransactionType::Buy);
        assert_eq!(txns[0].isin, Some("US46284V1017".to_string()));
    }
}
