//! Consorsbank PDF Parser
//!
//! Parses broker statements from Consorsbank (BNP Paribas).

use super::{
    parse_german_date, parse_german_decimal, parse_time, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct ConsorsbankParser {
    detect_patterns: Vec<&'static str>,
}

impl ConsorsbankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Consorsbank",
                "consorsbank",
                "BNP Paribas S.A. Niederlassung Deutschland",
                "90318 Nürnberg",
                "ORDERABRECHNUNG",
            ],
        }
    }

    /// Parse buy/sell orders (including savings plans)
    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect transaction type - look for KAUF AM or VERKAUF AM pattern
        let is_buy = content.contains("KAUF AM") || content.contains("Wertpapierkauf");
        let is_sell = content.contains("VERKAUF AM") || content.contains("Wertpapierverkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        // Regex patterns for Consorsbank format
        // Date and Time: "KAUF AM 07.01.2026 UM 09:30:55" or "Orderdatum 07.01.2026"
        let date_time_re = Regex::new(r"(?:KAUF|VERKAUF)\s+AM\s+(\d{2}\.\d{2}\.\d{4})\s+UM\s+(\d{2}:\d{2}:\d{2})").ok();
        let date_re = Regex::new(r"(?:KAUF|VERKAUF)\s+AM\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let date_alt_re = Regex::new(r"Orderdatum\s+(\d{2}\.\d{2}\.\d{4})").ok();

        // ISIN: Find any valid ISIN pattern (2 letters + 10 alphanumeric)
        let isin_re = Regex::new(r"\b([A-Z]{2}[A-Z0-9]{10})\b").ok();

        // WKN: Look for pattern "WKN A0S9GB" or find 6-char alphanumeric right before ISIN
        // Consorsbank format: "A0S9GB DE000A0S9GB0" - WKN is right before ISIN
        let wkn_before_isin_re = Regex::new(r"\b([A-Z0-9]{6})\s+[A-Z]{2}[A-Z0-9]{10}\b").ok();

        // Shares: "ST 2,69152" or "Umsatz\nST 2,69152"
        let shares_re = Regex::new(r"(?:Umsatz\s*\n?\s*)?ST\s+([\d.,]+)").ok();

        // Price per share: "Preis pro Anteil 122,989900 EUR"
        let price_re = Regex::new(r"Preis pro Anteil\s+([\d.,]+)\s*EUR").ok();

        // Kurswert: "Kurswert 331,03 EUR"
        let kurswert_re = Regex::new(r"Kurswert\s+([\d.,]+)\s*EUR").ok();

        // Provision/Fees: "Provision 4,97 EUR"
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)\s*EUR").ok();

        // Total amount: "zulasten Konto-Nr. ... 336,00 EUR" or "zugunsten Konto-Nr. ... 336,00 EUR"
        let total_re = Regex::new(r"zu(?:lasten|gunsten)\s+Konto-Nr\.\s+\d+\s+([\d.,]+)\s*EUR").ok();

        // Security name: Line after "Bezeichnung" until WKN
        let name_re = Regex::new(r"Bezeichnung\s+WKN\s+ISIN\s*\n([^\n]+)").ok();

        // Sparplan detection
        let is_sparplan = content.contains("SPARPLAN");
        let sparplan_name_re = Regex::new(r"Sparplanname:\s*(.+)").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            time: None,
            txn_type: if is_sell { ParsedTransactionType::Sell } else { ParsedTransactionType::Buy },
            security_name: None,
            isin: None,
            wkn: None,
            shares: None,
            price_per_share: None,
            gross_amount: 0.0,
            fees: 0.0,
            taxes: 0.0,
            net_amount: 0.0,
            currency: "EUR".to_string(),
            note: None,
            exchange_rate: None,
            forex_currency: None,
        };

        // Extract date and time
        let mut date_found = false;
        // First try to extract date with time
        if let Some(re) = &date_time_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                    date_found = true;
                }
                txn.time = parse_time(&caps[2]);
            }
        }
        // Fall back to date only
        if !date_found {
            if let Some(re) = &date_re {
                if let Some(caps) = re.captures(content) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                        date_found = true;
                    }
                }
            }
        }
        if !date_found {
            if let Some(re) = &date_alt_re {
                if let Some(caps) = re.captures(content) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                    }
                }
            }
        }

        // Extract ISIN
        if let Some(re) = &isin_re {
            if let Some(caps) = re.captures(content) {
                txn.isin = Some(caps[1].to_string());
            }
        }

        // Extract WKN - look for 6-char pattern right before ISIN
        if let Some(re) = &wkn_before_isin_re {
            if let Some(caps) = re.captures(content) {
                txn.wkn = Some(caps[1].to_string());
            }
        }

        // Extract shares
        if let Some(re) = &shares_re {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }

        // Extract price per share
        if let Some(re) = &price_re {
            if let Some(caps) = re.captures(content) {
                txn.price_per_share = parse_german_decimal(&caps[1]);
            }
        }

        // Extract gross amount (Kurswert)
        if let Some(re) = &kurswert_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = ctx.parse_amount("gross_amount", &caps[1]);
            }
        }

        // Extract fees (Provision)
        if let Some(re) = &provision_re {
            if let Some(caps) = re.captures(content) {
                txn.fees = ctx.parse_amount("fees", &caps[1]);
            }
        }

        // Extract net amount (total)
        if let Some(re) = &total_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
            }
        }

        // Extract security name
        if let Some(re) = &name_re {
            if let Some(caps) = re.captures(content) {
                let name = caps[1].trim();
                // Remove WKN and ISIN from name if present
                let name_cleaned = name.split_whitespace()
                    .take_while(|s| !s.chars().all(|c| c.is_ascii_alphanumeric()) || s.len() > 10)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !name_cleaned.is_empty() {
                    txn.security_name = Some(name_cleaned);
                } else {
                    txn.security_name = Some(name.to_string());
                }
            }
        }

        // Add note for Sparplan
        if is_sparplan {
            let mut note = "Sparplan".to_string();
            if let Some(re) = &sparplan_name_re {
                if let Some(caps) = re.captures(content) {
                    note = format!("Sparplan: {}", caps[1].trim());
                }
            }
            txn.note = Some(note);
        }

        // Validate and add transaction
        if txn.isin.is_some() || txn.wkn.is_some() {
            transactions.push(txn);
        }

        transactions
    }

    /// Parse dividend statements
    fn parse_dividends(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect dividend/distribution
        if !content.contains("DIVIDENDE")
            && !content.contains("Dividende")
            && !content.contains("ERTRAGSGUTSCHRIFT")
            && !content.contains("Ertragsgutschrift")
            && !content.contains("AUSSCHÜTTUNG")
            && !content.contains("Ausschüttung")
        {
            return transactions;
        }

        // Valuta date
        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let isin_re = Regex::new(r"ISIN\s*\n?\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let wkn_re = Regex::new(r"WKN\s*\n?\s*([A-Z0-9]{6})").ok();
        let shares_re = Regex::new(r"ST\s+([\d.,]+)").ok();

        // Gross amount: "Brutto EUR 123,45" or "Brutto 123,45 EUR"
        let gross_re = Regex::new(r"Brutto\s+(?:EUR\s+)?([\d.,]+)(?:\s*EUR)?").ok();

        // Taxes
        let kest_re = Regex::new(r"Kapitalertragsteuer\s+(?:EUR\s+)?([\d.,]+)").ok();
        let soli_re = Regex::new(r"Solidaritätszuschlag\s+(?:EUR\s+)?([\d.,]+)").ok();
        let kist_re = Regex::new(r"Kirchensteuer\s+(?:EUR\s+)?([\d.,]+)").ok();

        // Net amount: "zugunsten Konto-Nr. ... 100,00 EUR"
        let net_re = Regex::new(r"zugunsten\s+Konto-Nr\.\s+\d+\s+([\d.,]+)\s*EUR").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            time: None,
            txn_type: ParsedTransactionType::Dividend,
            security_name: None,
            isin: None,
            wkn: None,
            shares: None,
            price_per_share: None,
            gross_amount: 0.0,
            fees: 0.0,
            taxes: 0.0,
            net_amount: 0.0,
            currency: "EUR".to_string(),
            note: None,
            exchange_rate: None,
            forex_currency: None,
        };

        // Extract date
        if let Some(re) = &date_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }

        // Extract ISIN
        if let Some(re) = &isin_re {
            if let Some(caps) = re.captures(content) {
                txn.isin = Some(caps[1].to_string());
            }
        }

        // Extract WKN
        if let Some(re) = &wkn_re {
            if let Some(caps) = re.captures(content) {
                txn.wkn = Some(caps[1].to_string());
            }
        }

        // Extract shares
        if let Some(re) = &shares_re {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }

        // Extract gross amount
        if let Some(re) = &gross_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = ctx.parse_amount("gross_amount", &caps[1]);
            }
        }

        // Extract taxes
        let mut total_tax = 0.0;
        if let Some(re) = &kest_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("kest", &caps[1]);
            }
        }
        if let Some(re) = &soli_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("soli", &caps[1]);
            }
        }
        if let Some(re) = &kist_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("kist", &caps[1]);
            }
        }
        txn.taxes = total_tax;

        // Extract net amount
        if let Some(re) = &net_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
            }
        }

        // Validate and add transaction
        if txn.isin.is_some() && (txn.gross_amount > 0.0 || txn.net_amount > 0.0) {
            transactions.push(txn);
        }

        transactions
    }

    /// Parse account statements (deposits/withdrawals)
    fn parse_account(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Einzahlung (Deposit)
        if content.contains("EINZAHLUNG") || content.contains("Einzahlung") {
            let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let amount_re = Regex::new(r"zugunsten\s+Konto-Nr\.\s+\d+\s+([\d.,]+)\s*EUR").ok();

            let mut txn = ParsedTransaction {
                date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                time: None,
                txn_type: ParsedTransactionType::Deposit,
                security_name: None,
                isin: None,
                wkn: None,
                shares: None,
                price_per_share: None,
                gross_amount: 0.0,
                fees: 0.0,
                taxes: 0.0,
                net_amount: 0.0,
                currency: "EUR".to_string(),
                note: Some("Einzahlung".to_string()),
                exchange_rate: None,
                forex_currency: None,
            };

            if let Some(re) = &date_re {
                if let Some(caps) = re.captures(content) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                    }
                }
            }

            if let Some(re) = &amount_re {
                if let Some(caps) = re.captures(content) {
                    let amount = ctx.parse_amount("amount", &caps[1]);
                    txn.gross_amount = amount;
                    txn.net_amount = amount;
                }
            }

            if txn.net_amount > 0.0 {
                transactions.push(txn);
            }
        }

        // Auszahlung (Withdrawal)
        if content.contains("AUSZAHLUNG") || content.contains("Auszahlung") {
            let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
            let amount_re = Regex::new(r"zulasten\s+Konto-Nr\.\s+\d+\s+([\d.,]+)\s*EUR").ok();

            let mut txn = ParsedTransaction {
                date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                time: None,
                txn_type: ParsedTransactionType::Withdrawal,
                security_name: None,
                isin: None,
                wkn: None,
                shares: None,
                price_per_share: None,
                gross_amount: 0.0,
                fees: 0.0,
                taxes: 0.0,
                net_amount: 0.0,
                currency: "EUR".to_string(),
                note: Some("Auszahlung".to_string()),
                exchange_rate: None,
                forex_currency: None,
            };

            if let Some(re) = &date_re {
                if let Some(caps) = re.captures(content) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                    }
                }
            }

            if let Some(re) = &amount_re {
                if let Some(caps) = re.captures(content) {
                    let amount = ctx.parse_amount("amount", &caps[1]);
                    txn.gross_amount = amount;
                    txn.net_amount = amount;
                }
            }

            if txn.net_amount > 0.0 {
                transactions.push(txn);
            }
        }

        transactions
    }
}

impl BankParser for ConsorsbankParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        transactions.extend(self.parse_buy_sell(content, ctx));
        transactions.extend(self.parse_dividends(content, ctx));
        transactions.extend(self.parse_account(content, ctx));

        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Consorsbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_detect_consorsbank() {
        let parser = ConsorsbankParser::new();

        assert!(parser.detect("Consorsbank • 90318 Nürnberg"));
        assert!(parser.detect("BNP Paribas S.A. Niederlassung Deutschland"));
        assert!(parser.detect("ORDERABRECHNUNG"));
        assert!(!parser.detect("Deutsche Bank AG"));
    }

    #[test]
    fn test_parse_sparplan_buy() {
        let parser = ConsorsbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
Consorsbank • 90318 Nürnberg
ORDERABRECHNUNG
KAUF AM 07.01.2026 UM 09:30:55 SPARPLAN NR. 357329422.001
Bezeichnung WKN ISIN
0,0 % DT.BOERSE COM. XETRA-GOLD 00.0000 A0S9GB DE000A0S9GB0
Einheit Umsatz
ST 2,69152
Preis pro Anteil 122,989900 EUR
Kurswert 331,03 EUR
Provision 4,97 EUR
zulasten Konto-Nr. 0870740617 336,00 EUR
Sparplanname: Xetra Gold
"#;

        let transactions = parser.parse(content, &mut ctx).unwrap();

        assert_eq!(transactions.len(), 1);
        let txn = &transactions[0];

        assert_eq!(txn.txn_type, ParsedTransactionType::Buy);
        assert_eq!(txn.isin, Some("DE000A0S9GB0".to_string()));
        assert_eq!(txn.wkn, Some("A0S9GB".to_string()));
        assert!((txn.shares.unwrap() - 2.69152).abs() < 0.0001);
        assert!((txn.price_per_share.unwrap() - 122.9899).abs() < 0.0001);
        assert!((txn.gross_amount - 331.03).abs() < 0.01);
        assert!((txn.fees - 4.97).abs() < 0.01);
        assert!((txn.net_amount - 336.0).abs() < 0.01);
        assert!(txn.note.as_ref().unwrap().contains("Sparplan"));
        // Check time extraction
        assert!(txn.time.is_some());
        let time = txn.time.unwrap();
        assert_eq!(time.hour(), 9);
        assert_eq!(time.minute(), 30);
        assert_eq!(time.second(), 55);
    }
}
