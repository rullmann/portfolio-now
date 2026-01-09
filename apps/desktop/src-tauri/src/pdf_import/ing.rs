//! ING (ING-DiBa) PDF Parser
//!
//! Parses broker statements from ING Germany.

use super::{
    parse_german_date, parse_german_decimal, BankParser, ParsedTransaction,
    ParsedTransactionType,
};
use regex::Regex;

pub struct IngParser {
    detect_patterns: Vec<&'static str>,
}

impl IngParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "ING-DiBa AG",
                "ING Deutschland",
                "60628 Frankfurt am Main",
                "ING Wholesale Banking",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // ING patterns
        let _txn_type_re = Regex::new(r"(Wertpapierabrechnung|Wertpapierkauf|Wertpapierverkauf)\s*(Kauf|Verkauf)?").ok();
        let date_re = Regex::new(r"Ausf(?:ü|u)hrungstag\s*/?\s*-zeit\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let isin_re = Regex::new(r"ISIN\s+([A-Z]{2}[A-Z0-9]{10})").ok();
        let wkn_re = Regex::new(r"WKN\s+([A-Z0-9]{6})").ok();
        let shares_re = Regex::new(r"Nominale\s+St(?:ü|u)ck\s+([\d.,]+)").ok();
        let price_re = Regex::new(r"Kurs\s+([\d.,]+)\s*EUR").ok();
        let amount_re = Regex::new(r"Kurswert\s+EUR\s+([\d.,]+)").ok();
        let provision_re = Regex::new(r"Provision\s+EUR\s+([\d.,]+)").ok();
        let total_re = Regex::new(r"Endbetrag.*?EUR\s+([\d.,]+)").ok();

        // Check if it's a buy or sell
        let is_buy = content.contains("Kauf") || content.contains("Wertpapierkauf");
        let is_sell = content.contains("Verkauf") || content.contains("Wertpapierverkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
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

        // Extract price
        if let Some(re) = &price_re {
            if let Some(caps) = re.captures(content) {
                txn.price_per_share = parse_german_decimal(&caps[1]);
            }
        }

        // Extract gross amount
        if let Some(re) = &amount_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract fees
        if let Some(re) = &provision_re {
            if let Some(caps) = re.captures(content) {
                txn.fees = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract total
        if let Some(re) = &total_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract security name (usually between WKN and other info)
        let name_re = Regex::new(r"WKN\s+[A-Z0-9]{6}\s+(.+?)(?:\n|Nominale)").ok();
        if let Some(re) = name_re {
            if let Some(caps) = re.captures(content) {
                txn.security_name = Some(caps[1].trim().to_string());
            }
        }

        if txn.isin.is_some() || txn.wkn.is_some() {
            transactions.push(txn);
        }

        transactions
    }

    fn parse_dividends(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Check for dividend
        if !content.contains("Dividendengutschrift")
            && !content.contains("Ertragsgutschrift")
            && !content.contains("Ausschüttung") {
            return transactions;
        }

        let date_re = Regex::new(r"Valuta\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let isin_re = Regex::new(r"ISIN\s+([A-Z]{2}[A-Z0-9]{10})").ok();
        let shares_re = Regex::new(r"Nominale\s+St(?:ü|u)ck\s+([\d.,]+)").ok();
        let gross_re = Regex::new(r"Bruttobetrag\s+EUR\s+([\d.,]+)").ok();
        let tax_re = Regex::new(r"Kapitalertragsteuer.*?EUR\s+([\d.,]+)").ok();
        let soli_re = Regex::new(r"Solidarit(?:ä|a)tszuschlag.*?EUR\s+([\d.,]+)").ok();
        let net_re = Regex::new(r"Gutschrift.*?EUR\s+([\d.,]+)").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
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

        // Extract shares
        if let Some(re) = &shares_re {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }

        // Extract gross
        if let Some(re) = &gross_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        // Extract taxes
        let mut total_tax = 0.0;
        if let Some(re) = &tax_re {
            if let Some(caps) = re.captures(content) {
                total_tax += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        if let Some(re) = &soli_re {
            if let Some(caps) = re.captures(content) {
                total_tax += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }
        txn.taxes = total_tax;

        // Extract net
        if let Some(re) = &net_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        if txn.isin.is_some() && (txn.gross_amount > 0.0 || txn.net_amount > 0.0) {
            transactions.push(txn);
        }

        transactions
    }
}

impl BankParser for IngParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        transactions.extend(self.parse_buy_sell(content));
        transactions.extend(self.parse_dividends(content));

        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "ING"
    }
}
