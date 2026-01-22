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

        // Detect dividend documents - including "Wertpapierereignisse" and "Dividendenabrechnung" from Baader Bank
        if !content.contains("Dividende")
            && !content.contains("Ertragsgutschrift")
            && !content.contains("Ausschüttung")
            && !content.contains("Wertpapierereignisse")
            && !content.contains("Dividendenabrechnung") {
            return transactions;
        }

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

        // === BAADER BANK / SCALABLE CAPITAL FORMAT ===
        // Example: "STK 85 Ausschüttung"
        // Example: "Zahltag: 13.11.2025"
        // Example: "Bruttobetrag EUR 18,98"
        // Example: "US-Quellensteuer 2,85 -EUR"
        // Example: "16,13EURValuta: 13.11.2025"

        // 1. Extract shares - "STK 85" format (Baader Bank)
        if let Some(re) = Regex::new(r"STK\s+(\d+)").ok() {
            if let Some(caps) = re.captures(content) {
                txn.shares = parse_german_decimal(&caps[1]);
            }
        }
        // Fallback: "Stück: X" or "Nominale: X"
        if txn.shares.is_none() {
            if let Some(re) = Regex::new(r"(?:Nominale|St(?:ü|u)ck)\s*[:/]?\s*([\d.,]+)").ok() {
                if let Some(caps) = re.captures(content) {
                    txn.shares = parse_german_decimal(&caps[1]);
                }
            }
        }

        // 2. Extract ISIN - "ISIN: US0378331005" or "ISIN US0378331005"
        if let Some(re) = Regex::new(r"ISIN[:\s]+([A-Z]{2}[A-Z0-9]{10})").ok() {
            if let Some(caps) = re.captures(content) {
                txn.isin = Some(caps[1].to_string());
            }
        }
        if txn.isin.is_none() {
            txn.isin = extract_isin(content);
        }

        // 3. Extract WKN - "WKN: 865985" or "WKN 865985"
        if let Some(re) = Regex::new(r"WKN[:\s]+([A-Z0-9]{6})").ok() {
            if let Some(caps) = re.captures(content) {
                txn.wkn = Some(caps[1].to_string());
            }
        }

        // 4. Extract date - "Zahltag: 13.11.2025" or "Valuta: 13.11.2025"
        if let Some(re) = Regex::new(r"(?:Zahltag|Valuta|Zahlbarkeitstag)[:\s]+(\d{2}\.\d{2}\.\d{4})").ok() {
            if let Some(caps) = re.captures(content) {
                if let Some(date) = parse_german_date(&caps[1]) {
                    txn.date = date;
                }
            }
        }
        // Fallback: Ex-Tag
        if txn.date == chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap() {
            if let Some(re) = Regex::new(r"Ex-Tag[:\s]+(\d{2}\.\d{2}\.\d{4})").ok() {
                if let Some(caps) = re.captures(content) {
                    if let Some(date) = parse_german_date(&caps[1]) {
                        txn.date = date;
                    }
                }
            }
        }

        // 5. Extract exchange rate - "Umrechnungskurs: 1,16438EUR/USD"
        if let Some(re) = Regex::new(r"(?:Umrechnungskurs|Devisenkurs|Wechselkurs)[:\s]+([\d.,]+)").ok() {
            if let Some(caps) = re.captures(content) {
                txn.exchange_rate = parse_german_decimal(&caps[1]);
            }
        }

        // 6. Extract gross amount in EUR - "Bruttobetrag EUR 18,98"
        if let Some(re) = Regex::new(r"Bruttobetrag\s+EUR\s+([\d.,]+)").ok() {
            if let Some(caps) = re.captures(content) {
                txn.gross_amount = ctx.parse_amount("gross_eur", &caps[1]);
            }
        }

        // 7. Extract gross amount in foreign currency - "Bruttobetrag USD 22,10"
        if let Some(re) = Regex::new(r"Bruttobetrag\s+(USD|GBP|CHF)\s+([\d.,]+)").ok() {
            if let Some(caps) = re.captures(content) {
                txn.forex_currency = Some(caps[1].to_string());
                // If we don't have EUR gross yet, calculate from foreign
                if txn.gross_amount == 0.0 {
                    let foreign_gross = ctx.parse_amount("gross_foreign", &caps[2]);
                    if let Some(rate) = txn.exchange_rate {
                        if rate > 0.0 {
                            txn.gross_amount = foreign_gross / rate;
                        }
                    }
                }
            }
        }

        // Fallback gross patterns for other formats
        if txn.gross_amount == 0.0 {
            if let Some(re) = Regex::new(r"(?:Brutto|Bruttobetrag|Bruttodividende)[:\s]+(?:EUR\s+)?([\d.,]+)").ok() {
                if let Some(caps) = re.captures(content) {
                    txn.gross_amount = ctx.parse_amount("gross_amount", &caps[1]);
                }
            }
        }

        // 8. Extract taxes
        let mut total_tax = 0.0;

        // US-Quellensteuer: "US-Quellensteuer 2,85 -EUR" or "US-Quellensteuer 2,85-EUR"
        if let Some(re) = Regex::new(r"US-Quellensteuer\s+([\d.,]+)\s*-?\s*EUR").ok() {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("us_qst", &caps[1]);
            }
        }
        // General Quellensteuer: "Quellensteuer 15,00 % 2,85 -EUR"
        if total_tax == 0.0 {
            if let Some(re) = Regex::new(r"Quellensteuer(?:\s+[\d.,]+\s*%?)?\s+([\d.,]+)\s*-?\s*EUR").ok() {
                if let Some(caps) = re.captures(content) {
                    total_tax += ctx.parse_amount("quellensteuer", &caps[1]);
                }
            }
        }
        // Kapitalertragsteuer
        if let Some(re) = Regex::new(r"Kapitalertragsteuer\s*[:\s]*([\d.,]+)\s*-?\s*EUR").ok() {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("kest", &caps[1]);
            }
        }
        // Solidaritätszuschlag
        if let Some(re) = Regex::new(r"Solidarit(?:ä|a)tszuschlag\s*[:\s]*([\d.,]+)\s*-?\s*EUR").ok() {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("soli", &caps[1]);
            }
        }
        // Kirchensteuer
        if let Some(re) = Regex::new(r"Kirchensteuer\s*[:\s]*([\d.,]+)\s*-?\s*EUR").ok() {
            if let Some(caps) = re.captures(content) {
                total_tax += ctx.parse_amount("kist", &caps[1]);
            }
        }
        txn.taxes = total_tax;

        // 9. Extract net amount - "16,13EURValuta:" or "Zu Gunsten...16,13 EUR"
        // Baader Bank format: amount directly before "EUR" and "Valuta"
        if let Some(re) = Regex::new(r"(\d+,\d{2})EUR\s*Valuta").ok() {
            if let Some(caps) = re.captures(content) {
                txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
            }
        }
        // Alternative: "Ausmachender Betrag EUR 16,13" or "Gutschrift EUR 16,13"
        if txn.net_amount == 0.0 {
            if let Some(re) = Regex::new(r"(?:Ausmachender Betrag|Gutschrift|Netto)[:\s]+(?:EUR\s+)?([\d.,]+)").ok() {
                if let Some(caps) = re.captures(content) {
                    txn.net_amount = ctx.parse_amount("net_amount", &caps[1]);
                }
            }
        }
        // Calculate net from gross - taxes if still 0
        if txn.net_amount == 0.0 && txn.gross_amount > 0.0 {
            txn.net_amount = txn.gross_amount - txn.taxes;
        }

        // 10. Extract security name - "p.STKApple Inc." or line after ISIN/WKN
        if let Some(re) = Regex::new(r"p\.STK\s*([A-Za-z][A-Za-z0-9\s.,&'-]+?)(?:\n|Zahlungszeitraum)").ok() {
            if let Some(caps) = re.captures(content) {
                let name = caps[1].trim();
                if !name.is_empty() && name.len() < 100 {
                    txn.security_name = Some(name.to_string());
                }
            }
        }
        // Fallback: Wertpapierbezeichnung
        if txn.security_name.is_none() {
            if let Some(re) = Regex::new(r"(?:Wertpapier(?:bezeichnung)?|Gattungsbezeichnung)[:\s]+(.+?)(?:\n|ISIN|WKN)").ok() {
                if let Some(caps) = re.captures(content) {
                    let name = caps[1].trim();
                    if !name.is_empty() && name.len() < 100 {
                        txn.security_name = Some(name.to_string());
                    }
                }
            }
        }

        // Only add if we have ISIN and some amount
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
