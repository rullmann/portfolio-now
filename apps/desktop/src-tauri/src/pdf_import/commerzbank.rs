//! Commerzbank PDF Parser
//!
//! Parses broker statements from Commerzbank.
//! Handles OCR artifacts where letters are separated by spaces.

use super::{
    extract_isin, parse_german_date, parse_german_decimal, BankParser, ParseContext,
    ParsedTransaction, ParsedTransactionType,
};
use regex::Regex;

pub struct CommerzbankParser {
    detect_patterns: Vec<&'static str>,
}

impl CommerzbankParser {
    pub fn new() -> Self {
        Self {
            detect_patterns: vec![
                "COMMERZBANK",
                "Commerzbank",
                "C O M M E R Z B A N K",
                "C]ommerzbank", // OCR artifact
                "Aktiengesellschaft",
                "A k t i e n g e s e l l s c h a f t",
            ],
        }
    }

    /// Normalize OCR text with spaces between letters
    /// "W e r t p a p i e r k a u f" -> "Wertpapierkauf"
    fn normalize_ocr_text(text: &str) -> String {
        // Remove single-character spacing pattern
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            result.push(c);

            // Skip single space between letters
            if i + 2 < chars.len() && chars[i + 1] == ' ' && chars[i + 2].is_alphabetic() {
                i += 2; // Skip space, next char will be pushed in next iteration
            } else {
                i += 1;
            }
        }

        result
    }

    /// Parse OCR-spaced numbers like "1 . 4 3 9 , 1 3" or "1 0 , 1 9 5"
    fn parse_ocr_number(text: &str) -> Option<f64> {
        // Remove all spaces first
        let cleaned = text.replace(' ', "");
        parse_german_decimal(&cleaned)
    }

    /// Parse OCR-spaced date like "1 7 . 0 2 . 2 0 2 1"
    fn parse_ocr_date(text: &str) -> Option<chrono::NaiveDate> {
        let cleaned = text.replace(' ', "");
        parse_german_date(&cleaned)
    }

    fn parse_buy_sell(&self, content: &str, ctx: &mut ParseContext) -> Vec<ParsedTransaction> {
        let mut transactions = Vec::new();

        // Detect buy or sell - handles OCR spaces
        let is_buy = content.contains("W e r t p a p i e r k a u f")
            || content.contains("Wertpapierkauf");
        let is_sell = content.contains("W e r t p a p i e r v e r k a u f")
            || content.contains("Wertpapierverkauf");

        if !is_buy && !is_sell {
            return transactions;
        }

        let txn_type = if is_buy {
            ParsedTransactionType::Buy
        } else {
            ParsedTransactionType::Sell
        };

        // Extract WKN (6 alphanumeric at end of security description line)
        // Pattern: "W e r t p a p i e r k e n n n u m m e r" followed by line with WKN
        let wkn_re = Regex::new(r"(?:Wertpapierkennnummer|W e r t p a p i e r k e n n n u m m e r)[^\n]*\n[^\n]*?([A-Z0-9]{6})\s*\n").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string());

        // Extract ISIN if present
        let isin = extract_isin(content);

        // Extract security name - line before WKN typically has name
        // Look for pattern after Wertpapier-Bezeichnung
        let name_re = Regex::new(r"(?:W e r t p a p i e r - B e z e i c h n u n g|Wertpapier-Bezeichnung)[^\n]*\n\s*([^\n]+?)\s+[A-Z0-9]{6}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| Self::normalize_ocr_text(c[1].trim()));

        // Extract shares - "S t . 0 , 5 7 2" or "St. 0,572"
        let shares_re = Regex::new(r"S\s*t\s*\.?\s*([\d\s.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_number(&c[1]));

        // Extract price per share - after shares, "EUR 4 3 , 6 4" or "EUR 43,64"
        let price_re = Regex::new(r"S\s*t\s*\.?\s*[\d\s.,]+\s*EUR\s*([\d\s.,]+)").ok();
        let price_per_share = price_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_number(&c[1]));

        // Extract Kurswert (gross amount)
        let kurswert_re = Regex::new(r"K\s*u\s*r\s*s\s*w\s*e\s*r\s*t\s*:?\s*EUR\s*([\d\s.,]+)").ok();
        let gross_amount = kurswert_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_number(&c[1]))
            .unwrap_or(0.0);

        // Extract date - Geschäftstag or Valuta
        let date_re = Regex::new(r"(?:G\s*e\s*s\s*c\s*h\s*ä\s*f\s*t\s*s\s*t\s*a\s*g|Geschäftstag)\s*:?\s*([\d\s.]+)").ok();
        let valuta_re = Regex::new(r"V\s*a\s*l\s*u\s*t\s*a\s*([\d\s.]+)").ok();

        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_date(&c[1]))
            .or_else(|| {
                valuta_re
                    .as_ref()
                    .and_then(|re| re.captures(content))
                    .and_then(|c| Self::parse_ocr_date(&c[1]))
            })
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // For Commerzbank, the Kurswert usually equals net amount (fees on separate statement)
        let net_amount = gross_amount;

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
                fees: 0.0, // Commerzbank shows fees on separate tax statement
                taxes: 0.0,
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Commerzbank Import".to_string()),
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
        let is_dividend = content.contains("E r t r a g s g u t s c h r i f t")
            || content.contains("Ertragsgutschrift")
            || content.contains("Dividendengutschrift")
            || content.contains("D i v i d e n d e n g u t s c h r i f t");

        if !is_dividend {
            return transactions;
        }

        // Extract ISIN
        let isin = extract_isin(content);

        // Extract WKN - in dividend docs often after WKN/ISIN label
        let wkn_re = Regex::new(r"WKN/?ISIN[^\n]*\n[^\n]*?([A-Z0-9]{6})").ok();
        let wkn = wkn_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| c[1].to_string())
            .or_else(|| {
                // Alternative: WKN standalone at start of security line
                let alt_re = Regex::new(r"(?:STK|Stück)[^\n]*([A-Z0-9]{6})\n").ok();
                alt_re
                    .as_ref()
                    .and_then(|re| re.captures(content))
                    .map(|c| c[1].to_string())
            });

        // Extract security name
        let name_re = Regex::new(r"(?:per\s+[\d.\s]+|STK\s+[\d.,\s]+)\s*([A-Za-z][^\n]+?)\s+[A-Z0-9]{6}").ok();
        let security_name = name_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| Self::normalize_ocr_text(c[1].trim()));

        // Extract shares - "STK 1 2 3 , 0 0 0" or "STK 123,000"
        let shares_re = Regex::new(r"STK\s*([\d\s.,]+)").ok();
        let shares = shares_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_number(&c[1]));

        // Extract Bruttobetrag (gross amount in original currency)
        let brutto_re = Regex::new(r"B\s*r\s*u\s*t\s*t\s*o\s*b\s*e\s*t\s*r\s*a\s*g\s*:?\s*([A-Z]{3})\s*([\d\s.,]+)").ok();
        let (forex_currency, forex_gross) = brutto_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                let currency = c[1].replace(' ', "");
                let amount = Self::parse_ocr_number(&c[2]).unwrap_or(0.0);
                (Some(currency), amount)
            })
            .unwrap_or((None, 0.0));

        // Extract exchange rate and EUR amount
        // Pattern: "zum D e v i s e n k u r s : EUR/USD 1 ,142700 EUR 1 2 3 , 4 5"
        let fx_re = Regex::new(r"(?:Devisenkurs|D e v i s e n k u r s)\s*:?\s*EUR/[A-Z]{3}\s*([\d\s.,]+)\s*EUR\s*([\d\s.,]+)").ok();
        let (exchange_rate, gross_eur) = fx_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .map(|c| {
                let rate = Self::parse_ocr_number(&c[1]);
                let eur = Self::parse_ocr_number(&c[2]).unwrap_or(0.0);
                (rate, eur)
            })
            .unwrap_or((None, forex_gross));

        // Gross amount in EUR (either directly or converted)
        let gross_amount = if gross_eur > 0.0 {
            gross_eur
        } else if forex_gross > 0.0 && exchange_rate.is_some() {
            forex_gross / exchange_rate.unwrap()
        } else {
            forex_gross
        };

        // Extract date - Valuta or "zahlbar ab"
        let date_re = Regex::new(r"V\s*a\s*l\s*u\s*t\s*a\s*([\d\s.]+)").ok();
        let zahlbar_re = Regex::new(r"zahlbar\s+ab\s+([\d\s.]+)").ok();

        let date = date_re
            .as_ref()
            .and_then(|re| re.captures(content))
            .and_then(|c| Self::parse_ocr_date(&c[1]))
            .or_else(|| {
                zahlbar_re
                    .as_ref()
                    .and_then(|re| re.captures(content))
                    .and_then(|c| Self::parse_ocr_date(&c[1]))
            })
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // For Commerzbank dividends, taxes are on separate statement
        // The "Zu Ihren Gunsten vor Steuern" is the pre-tax amount
        let net_amount = gross_amount; // Taxes handled separately

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
                taxes: 0.0, // On separate tax statement
                net_amount,
                currency: "EUR".to_string(),
                note: Some("Commerzbank Ertragsgutschrift".to_string()),
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

impl BankParser for CommerzbankParser {
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
        "Commerzbank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect() {
        let parser = CommerzbankParser::new();
        assert!(parser.detect("C O M M E R Z B A N K\nAktiengesellschaft"));
        assert!(parser.detect("COMMERZBANK AG"));
        assert!(!parser.detect("Deutsche Kreditbank AG"));
        assert!(!parser.detect("ING-DiBa AG"));
    }

    #[test]
    fn test_normalize_ocr_text() {
        assert_eq!(
            CommerzbankParser::normalize_ocr_text("W e r t p a p i e r"),
            "Wertpapier"
        );
        assert_eq!(
            CommerzbankParser::normalize_ocr_text("K a u f"),
            "Kauf"
        );
    }

    #[test]
    fn test_parse_ocr_number() {
        assert_eq!(CommerzbankParser::parse_ocr_number("1 2 3 , 4 5"), Some(123.45));
        assert_eq!(CommerzbankParser::parse_ocr_number("1 . 4 3 9 , 1 3"), Some(1439.13));
        assert_eq!(CommerzbankParser::parse_ocr_number("0 , 5 7 2"), Some(0.572));
    }

    #[test]
    fn test_parse_ocr_date() {
        assert_eq!(
            CommerzbankParser::parse_ocr_date("1 7 . 0 2 . 2 0 2 1"),
            Some(chrono::NaiveDate::from_ymd_opt(2021, 2, 17).unwrap())
        );
        assert_eq!(
            CommerzbankParser::parse_ocr_date("18.04.2017"),
            Some(chrono::NaiveDate::from_ymd_opt(2017, 4, 18).unwrap())
        );
    }

    #[test]
    fn test_parse_buy() {
        let parser = CommerzbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
W e r t p a p i e r k a u f
G e s c h ä f t s n u m m e r : 7 2 0 0 1 4 1 1
G e s c h ä f t s t a g : 1 8 . 0 4 . 2 0 1 7
W e r t p a p i e r - B e z e i c h n u n g W e r t p a p i e r k e n n n u m m e r
i S h s I I I - C o r e MSCI W o r l d U . E T F A0RPWH
S t . 0 , 5 7 2 EUR 4 3 , 6 4
K u r s w e r t : EUR 2 4 , 9 6
C O M M E R Z B A N K
Aktiengesellschaft
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Buy);
        assert_eq!(txn.wkn, Some("A0RPWH".to_string()));
        assert!((txn.shares.unwrap() - 0.572).abs() < 0.001);
        assert!((txn.gross_amount - 24.96).abs() < 0.01);
        assert_eq!(txn.date, chrono::NaiveDate::from_ymd_opt(2017, 4, 18).unwrap());
    }

    #[test]
    fn test_parse_sell() {
        let parser = CommerzbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
W e r t p a p i e r v e r k a u f
G e s c h ä f t s t a g : 1 7 . 0 2 . 2 0 2 1
W e r t p a p i e r - B e z e i c h n u n g W e r t p a p i e r k e n n n u m m e r
V e r m ö g e n s M a n a g e m e n t B a l a n c e A0M16S
S t . 1 0 , 1 9 5 EUR 1 4 1 , 1 6
K u r s w e r t : EUR 1 . 4 3 9 , 1 3
C O M M E R Z B A N K
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Sell);
        assert_eq!(txn.wkn, Some("A0M16S".to_string()));
        assert!((txn.shares.unwrap() - 10.195).abs() < 0.001);
        assert!((txn.gross_amount - 1439.13).abs() < 0.01);
    }

    #[test]
    fn test_parse_dividend() {
        let parser = CommerzbankParser::new();
        let mut ctx = ParseContext::new();

        let content = r#"
E r t r a g s g u t s c h r i f t
Depo tbes tand W e r t p a p i e r - B e z e i c h n u n g WKN/ISIN
p e r 2 7 . 0 5 . 2 0 1 5 iShs-MSCI N . America UCITS ETF A0J206
STK 1 2 3 , 0 0 0 Bearer Shares ( D t . Z e r t . ) o . N . DE000A0J2060
B r u t t o b e t r a g : USD 1 2 3 , 4 5
zum D e v i s e n k u r s : EUR/USD 1 ,142700 EUR 1 2 3 , 4 5
V a l u t a 2 2 . 0 6 . 2 0 1 5
C O M M E R Z B A N K
"#;

        let txns = parser.parse(content, &mut ctx).unwrap();
        assert_eq!(txns.len(), 1);

        let txn = &txns[0];
        assert_eq!(txn.txn_type, ParsedTransactionType::Dividend);
        assert_eq!(txn.isin, Some("DE000A0J2060".to_string()));
        assert!((txn.shares.unwrap() - 123.0).abs() < 0.001);
        assert!((txn.gross_amount - 123.45).abs() < 0.01);
        assert_eq!(txn.forex_currency, Some("USD".to_string()));
    }
}
