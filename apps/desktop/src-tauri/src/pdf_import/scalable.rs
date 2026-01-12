//! Scalable Capital PDF Parser
//!
//! Parses broker statements from Scalable Capital.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct ScalableParser {
    detect_patterns: Vec<&'static str>,
}

impl ScalableParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Scalable Capital",
                "SCALABLE CAPITAL",
                "Scalable Capital GmbH",
                "Scalable Capital Vermögensverwaltung",
                "Baader Bank",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        let is_buy = content.contains("Kauf") || content.contains("Wertpapierkauf") || content.contains("Sparplanausführung");
        let is_sell = content.contains("Verkauf") || content.contains("Wertpapierverkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        // Scalable/Baader Bank patterns
        let date_re = Regex::new(r"(?:Ausf(?:ü|u)hrungstag|Handelstag|Schlusstag)\s*[:/]?\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let isin_re = Regex::new(r"ISIN\s*[:/]?\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let wkn_re = Regex::new(r"WKN\s*[:/]?\s*([A-Z0-9]{6})").ok();
        let shares_re = Regex::new(r"(?:Nominale|St(?:ü|u)ck)\s*[:/]?\s*([\d.,]+)").ok();
        let price_re = Regex::new(r"(?:Kurs|Ausf(?:ü|u)hrungskurs)\s*[:/]?\s*([\d.,]+)\s*EUR").ok();
        let amount_re = Regex::new(r"Kurswert\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let provision_re = Regex::new(r"(?:Provision|Orderentgelt)\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let total_re = Regex::new(r"(?:Ausmachender Betrag|Gesamtbetrag)\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();

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
        if let Some(re) = &isin_re {
            if let Some(caps) = re.captures(content) {
                txn.isin = Some(caps[1].to_string());
            }
        } else {
            txn.isin = extract_isin(content);
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
                txn.gross_amount = ctx.parse_amount("gross_amount", &caps[1]);
            }
        }

        // Extract fees
        if let Some(re) = &provision_re {
            if let Some(caps) = re.captures(content) {
                txn.fees = ctx.parse_amount("fees", &caps[1]);
            }
        }

        // Extract total
        if let Some(re) = &total_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
            }
        }

        // Extract security name
        let name_re = Regex::new(r"(?:Wertpapier|Wertpapierbezeichnung)\s*[:/]?\s*(.+?)(?:\n|ISIN|WKN)").ok();
        if let Some(re) = name_re {
            if let Some(caps) = re.captures(content) {
                let name = caps[1].trim();
                if !name.is_empty() {
                    txn.security_name = Some(name.to_string());
                }
            }
        }

        if txn.isin.is_some() || txn.wkn.is_some() {
            transactions.push(txn);
        }

        transactions
    }

    fn parse_dividends(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        if !content.contains("Dividende")
            && !content.contains("Ertragsgutschrift")
            && !content.contains("Ausschüttung") {
            return transactions;
        }

        let date_re = Regex::new(r"(?:Valuta|Zahltag)\s*[:/]?\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let isin_re = Regex::new(r"ISIN\s*[:/]?\s*([A-Z]{2}[A-Z0-9]{10})").ok();
        let shares_re = Regex::new(r"(?:Nominale|St(?:ü|u)ck)\s*[:/]?\s*([\d.,]+)").ok();
        let gross_re = Regex::new(r"(?:Brutto|Bruttobetrag)\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let tax_re = Regex::new(r"Kapitalertragsteuer\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let soli_re = Regex::new(r"Solidarit(?:ä|a)tszuschlag\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let qst_re = Regex::new(r"Quellensteuer\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();
        let net_re = Regex::new(r"(?:Gutschrift|Ausmachender Betrag)\s*[:/]?\s*EUR?\s*([\d.,]+)").ok();

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

        if let Some(re) = date_re {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }

        if let Some(re) = &isin_re {
            if let Some(caps) = re.captures(content) {
                txn.isin = Some(caps[1].to_string());
            }
        } else {
            txn.isin = extract_isin(content);
        }

        if let Some(re) = &shares_re {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }

        if let Some(re) = &gross_re {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = ctx.parse_amount("gross_amount", &caps[1]);
            }
        }

        // Collect all taxes
        let mut total_tax = 0.0;
        if let Some(re) = &tax_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("tax", &caps[1]);
            }
        }
        if let Some(re) = &soli_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("soli", &caps[1]);
            }
        }
        if let Some(re) = &qst_re {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("quellensteuer", &caps[1]);
            }
        }
        txn.taxes = total_tax;

        if let Some(re) = &net_re {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
            }
        }

        if txn.isin.is_some() && (txn.gross_amount > 0.0 || txn.net_amount > 0.0) {
            transactions.push(txn);
        }

        transactions
    }

    fn parse_fees(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Scalable Capital management fee
        if !content.contains("Verwaltungsentgelt") && !content.contains("Serviceentgelt") {
            return transactions;
        }

        let date_re = Regex::new(r"(?:Valuta|Buchungstag)\s*[:/]?\s*(\d{2}\.\d{2}\.\d{4})").ok();
        let amount_re = Regex::new(r"(?:Betrag|Entgelt)\s*[:/]?\s*-?\s*EUR?\s*([\d.,]+)").ok();

        let mut txn = ParsedTransaction {
            date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            time: None,
            txn_type: ParsedTransactionType::Fee,
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
            note: Some("Verwaltungsentgelt Scalable Capital".to_string()),
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
                let amount = ctx.parse_amount("fees", &caps[1]);
                txn.fees = amount;
                txn.net_amount = amount;
            }
        }

        if txn.fees > 0.0 {
            transactions.push(txn);
        }

        transactions
    }
}

impl BankParser for ScalableParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str, ctx: &mut ParseContext) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        transactions.extend(self.parse_buy_sell(content, ctx));
        transactions.extend(self.parse_dividends(content, ctx));
        transactions.extend(self.parse_fees(content, ctx));

        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "Scalable Capital"
    }
}
