//! DKB (Deutsche Kreditbank) PDF Parser
//!
//! Parses broker statements from DKB.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParsedTransaction,
    ParsedTransactionType,
};
use regex::Regex;

pub struct DkbParser {
    // Detection patterns
    detect_patterns: Vec<&'static str>,
}

impl DkbParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "Deutsche Kreditbank",
                "DKB AG",
                "DKB-Cash",
                "DKB Broker",
                "10919 Berlin",
            ],
        }
    }

    fn parse_buy_sell(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Pattern for buy/sell orders
        // Example: "Wertpapier Abrechnung Kauf" or "Wertpapier Abrechnung Verkauf"
        let txn_type_re = Regex::new(r"Wertpapier\s+Abrechnung\s+(Kauf|Verkauf)").ok();
        let date_re = Regex::new(r"Schlusstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let shares_re = Regex::new(r"St(?:ü|u)ck\s+([\d.,]+)").ok();
        let price_re = Regex::new(r"Ausf(?:ü|u)hrungskurs\s+([\d.,]+)\s*EUR").ok();
        let amount_re = Regex::new(r"Kurswert\s+([\d.,]+)\s*EUR").ok();
        let provision_re = Regex::new(r"Provision\s+([\d.,]+)\s*EUR").ok();
        let total_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\s*EUR").ok();

        // Split into sections by page or transaction
        let sections: Vec<&str> = content.split("Wertpapier Abrechnung").collect();

        for section in sections.iter().skip(1) {
            let mut txn = ParsedTransaction {
                date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                txn_type: ParsedTransactionType::Unknown,
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

            // Determine transaction type
            if let Some(re) = &txn_type_re {
                if let Some(caps) = re.captures(section) {
                    txn.txn_type = match &caps[1] {
                        "Kauf" => ParsedTransactionType::Buy,
                        "Verkauf" => ParsedTransactionType::Sell,
                        _ => ParsedTransactionType::Unknown,
                    };
                }
            }

            // Extract date
            if let Some(re) = &date_re {
                if let Some(caps) = re.captures(section) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                    }
                }
            }

            // Extract ISIN
            txn.isin = extract_isin(section);

            // Extract shares
            if let Some(re) = &shares_re {
                if let Some(caps) = re.captures(section) {
                    txn.shares = parse_german_decimal(&caps[1]);
                }
            }

            // Extract price per share
            if let Some(re) = &price_re {
                if let Some(caps) = re.captures(section) {
                    txn.price_per_share = parse_german_decimal(&caps[1]);
                }
            }

            // Extract gross amount (Kurswert)
            if let Some(re) = &amount_re {
                if let Some(caps) = re.captures(section) {
                    txn.gross_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract fees (Provision)
            if let Some(re) = &provision_re {
                if let Some(caps) = re.captures(section) {
                    txn.fees = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract total (Ausmachender Betrag)
            if let Some(re) = &total_re {
                if let Some(caps) = re.captures(section) {
                    txn.net_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                }
            }

            // Extract security name (line after ISIN usually)
            if let Some(isin) = &txn.isin {
                let name_re = Regex::new(&format!(r"{}\s*\n\s*(.+)", isin)).ok();
                if let Some(re) = name_re {
                    if let Some(caps) = re.captures(section) {
                        txn.security_name = Some(caps[1].trim().to_string());
                    }
                }
            }

            if txn.txn_type != ParsedTransactionType::Unknown {
                transactions.push(txn);
            }
        }

        transactions
    }

    fn parse_dividends(&self, content: &str) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Pattern for dividends
        let dividend_re = Regex::new(r"(Dividendengutschrift|Ertragsgutschrift|Aussch(?:ü|u)ttung)").ok();
        let date_re = Regex::new(r"Zahlbarkeitstag\s+(\d{2}\.\d{2}\.\d{4})").ok();
        let shares_re = Regex::new(r"St(?:ü|u)ck\s+([\d.,]+)").ok();
        let gross_re = Regex::new(r"Brutto\s+([\d.,]+)\s*EUR").ok();
        let tax_re = Regex::new(r"Kapitalertragsteuer\s+([\d.,]+)\s*EUR").ok();
        let soli_re = Regex::new(r"Solidarit(?:ä|a)tszuschlag\s+([\d.,]+)\s*EUR").ok();
        let net_re = Regex::new(r"Ausmachender Betrag\s+([\d.,]+)\s*EUR").ok();

        // Find dividend sections
        let joined_content = content
            .split(|c| c == '\n')
            .collect::<Vec<&str>>()
            .join("\n");
        let sections: Vec<&str> = joined_content
            .split("Dividendengutschrift")
            .collect();

        for section in sections.iter().skip(1) {
            let full_section = format!("Dividendengutschrift{}", section);

            if dividend_re.as_ref().map_or(false, |re| re.is_match(&full_section)) {
                let mut txn = ParsedTransaction {
                    date: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    txn_type: ParsedTransactionType::Dividend,
                    security_name: None,
                    isin: extract_isin(&full_section),
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
                    if let Some(caps) = re.captures(&full_section) {
                        if let Some(date) = parse_german_date(&caps[1]) {
                            txn.date = date;
                        }
                    }
                }

                // Extract shares
                if let Some(re) = &shares_re {
                    if let Some(caps) = re.captures(&full_section) {
                        txn.shares = parse_german_decimal(&caps[1]);
                    }
                }

                // Extract gross amount
                if let Some(re) = &gross_re {
                    if let Some(caps) = re.captures(&full_section) {
                        txn.gross_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                    }
                }

                // Extract taxes
                let mut total_tax = 0.0;
                if let Some(re) = &tax_re {
                    if let Some(caps) = re.captures(&full_section) {
                        total_tax += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                    }
                }
                if let Some(re) = &soli_re {
                    if let Some(caps) = re.captures(&full_section) {
                        total_tax += parse_german_decimal(&caps[1]).unwrap_or(0.0);
                    }
                }
                txn.taxes = total_tax;

                // Extract net amount
                if let Some(re) = &net_re {
                    if let Some(caps) = re.captures(&full_section) {
                        txn.net_amount = parse_german_decimal(&caps[1]).unwrap_or(0.0);
                    }
                }

                if txn.gross_amount > 0.0 || txn.net_amount > 0.0 {
                    transactions.push(txn);
                }
            }
        }

        transactions
    }
}

impl BankParser for DkbParser {
    fn detect(&self, content: &str) -> bool {
        self.detect_patterns
            .iter()
            .any(|pattern| content.contains(pattern))
    }

    fn parse(&self, content: &str) -> Result<Vec<ParsedTransaction>, String> {
        let mut transactions = Vec::new();

        // Parse buy/sell orders
        transactions.extend(self.parse_buy_sell(content));

        // Parse dividends
        transactions.extend(self.parse_dividends(content));

        // Sort by date
        transactions.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(transactions)
    }

    fn bank_name(&self) -> &'static str {
        "DKB"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = DkbParser::new();
        assert!(parser.detect("Deutsche Kreditbank AG"));
        assert!(parser.detect("DKB AG\nBerlin"));
        assert!(!parser.detect("ING-DiBa AG"));
    }
}
