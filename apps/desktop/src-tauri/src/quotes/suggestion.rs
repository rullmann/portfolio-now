//! Quote Provider Suggestion Service
//!
//! Rule-based suggestion system for optimal quote providers based on:
//! - ISIN prefix (country code)
//! - Security type (crypto, ETF, stock)
//! - Currency
//! - Ticker format

use serde::{Deserialize, Serialize};

/// Suggestion for a quote provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteSuggestion {
    pub security_id: i64,
    pub security_name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub suggested_feed: String,
    pub suggested_feed_url: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

/// Security info for suggestion analysis
#[derive(Debug, Clone)]
pub struct SecurityForSuggestion {
    pub id: i64,
    pub name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub currency: Option<String>,
}

/// Country-specific Yahoo Finance suffix mappings
const ISIN_YAHOO_MAPPINGS: &[(&str, &str, &str)] = &[
    // (ISIN prefix, Yahoo suffix, description)
    ("CH", ".SW", "Swiss Exchange (SIX)"),
    ("DE", ".DE", "XETRA/Frankfurt"),
    ("AT", ".VI", "Vienna Stock Exchange"),
    ("FR", ".PA", "Euronext Paris"),
    ("NL", ".AS", "Euronext Amsterdam"),
    ("BE", ".BR", "Euronext Brussels"),
    ("IT", ".MI", "Borsa Italiana Milan"),
    ("ES", ".MC", "Bolsa de Madrid"),
    ("PT", ".LS", "Euronext Lisbon"),
    ("GB", ".L", "London Stock Exchange"),
    ("IE", ".IR", "Euronext Dublin"),
    ("SE", ".ST", "Nasdaq Stockholm"),
    ("NO", ".OL", "Oslo Børs"),
    ("DK", ".CO", "Nasdaq Copenhagen"),
    ("FI", ".HE", "Nasdaq Helsinki"),
    ("AU", ".AX", "ASX Sydney"),
    ("JP", ".T", "Tokyo Stock Exchange"),
    ("HK", ".HK", "Hong Kong Stock Exchange"),
    ("CA", ".TO", "Toronto Stock Exchange"),
    ("US", "", "US exchanges (no suffix)"),
];

/// Known crypto symbols
const CRYPTO_SYMBOLS: &[&str] = &[
    "BTC", "ETH", "SOL", "ADA", "DOT", "AVAX", "MATIC", "LINK", "UNI", "ATOM",
    "XRP", "DOGE", "SHIB", "LTC", "BCH", "XLM", "ALGO", "VET", "FIL", "THETA",
    "BITCOIN", "ETHEREUM", "SOLANA", "CARDANO", "POLKADOT",
];

/// Suggest optimal quote provider for a security
pub fn suggest_quote_provider(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    // 1. Check for crypto
    if let Some(suggestion) = suggest_crypto_provider(security) {
        return Some(suggestion);
    }

    // 2. Check ISIN-based suggestion
    if let Some(suggestion) = suggest_by_isin(security) {
        return Some(suggestion);
    }

    // 3. Check ticker-based suggestion
    if let Some(suggestion) = suggest_by_ticker(security) {
        return Some(suggestion);
    }

    None
}

/// Check if security is a cryptocurrency and suggest CoinGecko
fn suggest_crypto_provider(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    let name_upper = security.name.to_uppercase();
    let ticker_upper = security.ticker.as_ref().map(|t| t.to_uppercase());

    // Check name for crypto keywords
    let is_crypto_name = name_upper.contains("BITCOIN")
        || name_upper.contains("ETHEREUM")
        || name_upper.contains("CRYPTO")
        || name_upper.contains("KRYPTO");

    // Check ticker against known crypto symbols
    let is_crypto_ticker = ticker_upper
        .as_ref()
        .map(|t| {
            // Extract base symbol (remove currency suffix like -EUR, /USD)
            let base = t.split(&['-', '/', '_'][..]).next().unwrap_or(t);
            CRYPTO_SYMBOLS.iter().any(|s| base == *s)
        })
        .unwrap_or(false);

    if is_crypto_name || is_crypto_ticker {
        let symbol = ticker_upper
            .as_ref()
            .and_then(|t| t.split(&['-', '/', '_'][..]).next())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "COINGECKO".to_string(),
            suggested_feed_url: security.currency.clone(),
            confidence: 0.95,
            reason: format!("Kryptowährung erkannt ({})", symbol),
        });
    }

    None
}

/// Suggest provider based on ISIN prefix (country code)
fn suggest_by_isin(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    let isin = security.isin.as_ref()?;

    if isin.len() < 2 {
        return None;
    }

    let country_code = &isin[0..2].to_uppercase();

    // Find matching country
    for (prefix, suffix, description) in ISIN_YAHOO_MAPPINGS {
        if country_code == *prefix {
            // For Yahoo, we need a ticker
            let ticker = security.ticker.as_ref()?;

            // Build feed URL (suffix)
            let feed_url = if suffix.is_empty() {
                None
            } else {
                Some(suffix.to_string())
            };

            return Some(QuoteSuggestion {
                security_id: security.id,
                security_name: security.name.clone(),
                isin: security.isin.clone(),
                ticker: security.ticker.clone(),
                suggested_feed: "YAHOO".to_string(),
                suggested_feed_url: feed_url,
                confidence: 0.90,
                reason: format!(
                    "ISIN {} → {} (Yahoo Finance {}{})",
                    country_code,
                    description,
                    ticker,
                    suffix
                ),
            });
        }
    }

    // Unknown country code - still suggest Yahoo without suffix
    if security.ticker.is_some() {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: None,
            confidence: 0.60,
            reason: format!("ISIN {} - Yahoo Finance (unbekannter Markt)", country_code),
        });
    }

    None
}

/// Suggest provider based on ticker format
fn suggest_by_ticker(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    let ticker = security.ticker.as_ref()?;

    // Check if ticker already has exchange suffix
    if ticker.contains('.') {
        let parts: Vec<&str> = ticker.split('.').collect();
        if parts.len() == 2 {
            let suffix = format!(".{}", parts[1]);

            // Validate known suffix
            let is_known_suffix = ISIN_YAHOO_MAPPINGS
                .iter()
                .any(|(_, s, _)| *s == suffix);

            if is_known_suffix {
                return Some(QuoteSuggestion {
                    security_id: security.id,
                    security_name: security.name.clone(),
                    isin: security.isin.clone(),
                    ticker: security.ticker.clone(),
                    suggested_feed: "YAHOO".to_string(),
                    suggested_feed_url: None, // Suffix already in ticker
                    confidence: 0.85,
                    reason: format!("Ticker {} enthält bereits Börsen-Suffix", ticker),
                });
            }
        }
    }

    // US-style tickers (1-5 uppercase letters, no special chars)
    let is_us_style = ticker.len() >= 1
        && ticker.len() <= 5
        && ticker.chars().all(|c| c.is_ascii_uppercase());

    if is_us_style {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: None,
            confidence: 0.70,
            reason: format!("US-Style Ticker {} (Yahoo Finance)", ticker),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_security(
        id: i64,
        name: &str,
        isin: Option<&str>,
        ticker: Option<&str>,
        currency: Option<&str>,
    ) -> SecurityForSuggestion {
        SecurityForSuggestion {
            id,
            name: name.to_string(),
            isin: isin.map(|s| s.to_string()),
            ticker: ticker.map(|s| s.to_string()),
            currency: currency.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_swiss_stock_suggestion() {
        let security = make_security(1, "Nestlé SA", Some("CH0038863350"), Some("NESN"), Some("CHF"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some(".SW".to_string()));
        assert!(suggestion.confidence >= 0.90);
    }

    #[test]
    fn test_german_stock_suggestion() {
        let security = make_security(2, "SAP SE", Some("DE0007164600"), Some("SAP"), Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some(".DE".to_string()));
    }

    #[test]
    fn test_us_stock_suggestion() {
        let security = make_security(3, "Apple Inc.", Some("US0378331005"), Some("AAPL"), Some("USD"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, None); // No suffix for US
    }

    #[test]
    fn test_crypto_by_ticker() {
        let security = make_security(4, "Bitcoin", None, Some("BTC-EUR"), Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "COINGECKO");
        assert!(suggestion.confidence >= 0.90);
    }

    #[test]
    fn test_crypto_by_name() {
        let security = make_security(5, "Bitcoin BTC", None, Some("BTCUSD"), Some("USD"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "COINGECKO");
    }

    #[test]
    fn test_ticker_with_suffix() {
        let security = make_security(6, "Novartis", None, Some("NOVN.SW"), None);
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, None); // Suffix already in ticker
    }

    #[test]
    fn test_no_suggestion_without_ticker() {
        let security = make_security(7, "Unknown Fund", Some("LU1234567890"), None, None);
        let suggestion = suggest_quote_provider(&security);

        // Should return None because no ticker available for Yahoo
        assert!(suggestion.is_none() || suggestion.as_ref().unwrap().confidence < 0.7);
    }
}
