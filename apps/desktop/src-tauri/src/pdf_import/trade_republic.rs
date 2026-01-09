//! Trade Republic PDF Parser
//!
//! Parses broker statements from Trade Republic.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParsedTransaction,
    ParsedTransactionType,
};
use regex::Regex;

pub struct TradeRepublicParser {
    detect_patterns: Vec<&'static str>,
}

impl TradeRepublicParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Trade Republic",
                "TRADE REPUBLIC",
                "Trade Republic Bank GmbH",
                "Brunnenstraße 19-21",
                "10119 Berlin",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Trade Republic patterns - they have a very specific format
        let is_buy = content.contains("Kauf") || content.contains("Order Kauf") || content.contains("Sparplanausführung");
        let is_sell = content.contains("Verkauf") || content.contains("Order Verkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        // Trade Republic uses a cleaner format
        let date_re = Regex::new(r"AUSF(?:Ü|U)HRUNG\s*(\d{2}\.\d{2}\.\d{4})").ok()
            .or_else(|| Regex::new(r"Datum\s*(\d{2}\.\d{2}\.\d{4})").ok());
        let _isin_re = Regex::new(r"ISIN:\s*([A-Z]{2}[A-Z0-9]{10})").ok()
            .or_else(|| Regex::new(r"([A-Z]{2}[A-Z0-9]{10})").ok());
        let shares_re = Regex::new(r"(?:Anzahl|St(?:ü|u)ck)\s*([\d.,]+)").ok();
        let price_re = Regex::new(r"(?:Kurs|Ausf(?:ü|u)hrungskurs)\s*([\d.,]+)\s*EUR").ok();
        let amount_re = Regex::new(r"Gesamt\s*-?\s*([\d.,]+)\s*EUR").ok();

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
            note: if content.contains("Sparplan") { Some("Sparplanausführung".to_string()) } else { None },
            exchange_rate: None,
            forex_currency: None,
        };

        // Extract date
        if let Some(re) = date_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }

        // Extract ISIN
        txn.isin = extract_isin(content);

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

        // Extract total amount
        if let Some(re) = &amount_re {
            if let Some(caps) = re.captures(content) {
                let amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                txn.net_amount = amount;
                txn.gross_amount = amount; // Trade Republic usually doesn't charge fees
            }
        }

        // Calculate gross from shares * price if we have both
        if txn.gross_amount == 0.0 {
            if let (Some(shares), Some(price)) = (txn.shares, txn.price_per_share) {
                txn.gross_amount = shares * price;
            }
        }

        // Extract security name (usually the first prominent line)
        let name_re = Regex::new(r"(?:Kauf|Verkauf|Sparplan)[^A-Za-z]*([A-Za-z][^\n]+)").ok();
        if let Some(re) = name_re {
            if let Some(caps) = re.captures(content) {
                let name = caps[1].trim();
                if !name.contains("ISIN") && !name.contains("Datum") {
                    txn.security_name = Some(name.to_string());
                }
            }
        }

        if txn.isin.is_some() {
            transactions.push(txn);
        }

        transactions
    }

    fn parse_dividends(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende")
            && !content.contains("Ausschüttung")
            && !content.contains("DIVIDENDE") {
            return transactions;
        }

        let date_re = Regex::new(r"(?:ZAHLTAG|Valuta)\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let shares_re = Regex::new(r"(?:Anzahl|St(?:ü|u)ck)\s*([\d.,]+)").ok();
        let gross_re = Regex::new(r"(?:Brutto|BRUTTO)\s*([\d.,]+)\s*EUR").ok();
        let tax_re = Regex::new(r"(?:Steuer|Quellensteuer)\s*-?\s*([\d.,]+)\s*EUR").ok();
        let net_re = Regex::new(r"(?:Gesamt|GESAMT)\s*([\d.,]+)\s*EUR").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            txn_type: ParsedTransactionType::Dividend,
            security_name: None,
            isin: extract_isin(content),
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

        if let Some(re) = date_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }

        if let Some(re) = &shares_re {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }

        if let Some(re) = &gross_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

        if let Some(re) = &tax_re {
            for caps in re.captures_iter(content) {
                txn.taxes += parse_german_decimal(&caps[1]).unwrap_or(0.0);
            }
        }

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

    fn parse_interest(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Zinsen") && !content.contains("ZINSEN") {
            return transactions;
        }

        let date_re = Regex::new(r"(?:ZAHLTAG|Valuta)\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let amount_re = Regex::new(r"(?:Gesamt|GESAMT)\s*([\d.,]+)\s*EUR").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            txn_type: ParsedTransactionType::Interest,
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
            note: Some("Zinsgutschrift".to_string()),
            exchange_rate: None,
            forex_currency: None,
        };

        if let Some(re) = date_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }

        if let Some(re) = &amount_re {
            if let Some(caps) = re.captures(content) {
                let amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                txn.gross_amount = amount;
                txn.net_amount = amount;
            }
        }

        if txn.net_amount > 0.0 {
            transactions.push(txn);
        }

        transactions
    }
}

impl BankParser for TradeRepublicParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        transactions.extend(self.parse_buy_sell(content));
        transactions.extend(self.parse_dividends(content));
        transactions.extend(self.parse_interest(content));

        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Trade Republic"
    }
}
