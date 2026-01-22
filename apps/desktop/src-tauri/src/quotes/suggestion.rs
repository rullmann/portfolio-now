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
    /// Suggested ticker symbol if none exists
    pub suggested_ticker: Option<String>,
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
    // 1. Check for commodities (gold, silver) - highest priority for physical metals
    if let Some(suggestion) = suggest_commodity_provider(security) {
        return Some(suggestion);
    }

    // 2. Check for crypto
    if let Some(suggestion) = suggest_crypto_provider(security) {
        return Some(suggestion);
    }

    // 3. Check ISIN-based suggestion
    if let Some(suggestion) = suggest_by_isin(security) {
        return Some(suggestion);
    }

    // 4. Check ticker-based suggestion
    if let Some(suggestion) = suggest_by_ticker(security) {
        return Some(suggestion);
    }

    // 5. Fallback: No ISIN, no ticker - suggest based on name only
    suggest_by_name_only(security)
}

/// Fallback suggestion when security has neither ISIN nor ticker
/// Derives a ticker from the name and suggests Yahoo Finance
fn suggest_by_name_only(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    // Skip if name is too short or empty
    if security.name.trim().is_empty() {
        return None;
    }

    // Derive ticker from name
    let suggested_ticker = derive_ticker_from_name(&security.name);

    if suggested_ticker == "UNKNOWN" {
        return None;
    }

    Some(QuoteSuggestion {
        security_id: security.id,
        security_name: security.name.clone(),
        isin: None,
        ticker: None,
        suggested_feed: "YAHOO".to_string(),
        suggested_feed_url: None,
        suggested_ticker: Some(suggested_ticker.clone()),
        confidence: 0.40, // Low confidence - needs manual verification
        reason: format!(
            "Kein ISIN/Ticker vorhanden - Ticker-Vorschlag aus Name: {}",
            suggested_ticker
        ),
    })
}

/// Derive a ticker symbol from security name
fn derive_ticker_from_name(name: &str) -> String {
    // Common company suffixes to remove
    let suffixes = [
        " AG", " SE", " SA", " Inc.", " Inc", " Corp.", " Corp", " Ltd.", " Ltd",
        " GmbH", " KGaA", " & Co.", " PLC", " N.V.", " NV", " S.A.", " S.p.A.",
        " Holding", " Holdings", " Group", " Gruppe",
    ];

    let mut clean_name = name.to_string();
    for suffix in suffixes {
        if clean_name.to_lowercase().ends_with(&suffix.to_lowercase()) {
            clean_name = clean_name[..clean_name.len() - suffix.len()].to_string();
        }
    }

    // Take first word and uppercase, max 6 chars
    let ticker = clean_name
        .split_whitespace()
        .next()
        .unwrap_or(&clean_name)
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .take(6)
        .collect::<String>()
        .to_uppercase();

    if ticker.is_empty() {
        "UNKNOWN".to_string()
    } else {
        ticker
    }
}

/// Check if security is a physical commodity (gold, silver, etc.) and suggest Yahoo Finance futures
/// NOTE: ETCs with ISIN (like Xetra-Gold) are NOT matched here - they have their own price per share
/// and should use the normal ISIN-based suggestion instead.
fn suggest_commodity_provider(security: &SecurityForSuggestion) -> Option<QuoteSuggestion> {
    let name_lower = security.name.to_lowercase();
    let ticker_lower = security
        .ticker
        .as_ref()
        .map(|t| t.to_lowercase())
        .unwrap_or_default();

    // If security has an ISIN, it's likely an ETC (like Xetra-Gold, EUWAX Gold)
    // ETCs have their own price per share, NOT the commodity futures price
    // Let the ISIN-based suggestion handle these
    if security.isin.is_some() {
        return None;
    }

    // Only match physical commodities without ISIN, or explicit futures tickers

    // Gold detection (physical gold or futures ticker)
    let is_gold_futures = ticker_lower == "gc=f"
        || ticker_lower.contains("xau")
        || (name_lower.contains("gold") && name_lower.contains("physisch"))
        || (name_lower.contains("gold") && name_lower.contains("barren"))
        || (name_lower.contains("gold") && name_lower.contains("münze"));

    if is_gold_futures {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: Some("GC=F".to_string()),
            suggested_ticker: None,
            confidence: 0.90,
            reason: "Erkannt als physisches Gold - Yahoo Finance Gold Futures".to_string(),
        });
    }

    // Silver detection (physical silver or futures ticker)
    let is_silver_futures = ticker_lower == "si=f"
        || ticker_lower.contains("xag")
        || (name_lower.contains("silber") && name_lower.contains("physisch"))
        || (name_lower.contains("silber") && name_lower.contains("barren"))
        || (name_lower.contains("silver") && name_lower.contains("physical"));

    if is_silver_futures {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: Some("SI=F".to_string()),
            suggested_ticker: None,
            confidence: 0.90,
            reason: "Erkannt als physisches Silber - Yahoo Finance Silver Futures".to_string(),
        });
    }

    // Platinum detection (physical or futures ticker)
    let is_platinum_futures = ticker_lower == "pl=f"
        || ticker_lower.contains("xpt")
        || (name_lower.contains("platin") && name_lower.contains("physisch"))
        || (name_lower.contains("platin") && name_lower.contains("barren"));

    if is_platinum_futures {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: Some("PL=F".to_string()),
            suggested_ticker: None,
            confidence: 0.90,
            reason: "Erkannt als physisches Platin - Yahoo Finance Platinum Futures".to_string(),
        });
    }

    // Palladium detection (physical or futures ticker)
    let is_palladium_futures = ticker_lower == "pa=f"
        || ticker_lower.contains("xpd")
        || (name_lower.contains("palladium") && name_lower.contains("physisch"))
        || (name_lower.contains("palladium") && name_lower.contains("barren"));

    if is_palladium_futures {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: Some("PA=F".to_string()),
            suggested_ticker: None,
            confidence: 0.90,
            reason: "Erkannt als physisches Palladium - Yahoo Finance Palladium Futures".to_string(),
        });
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
            suggested_ticker: None,
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
            // Build feed URL (suffix)
            let feed_url = if suffix.is_empty() {
                None
            } else {
                Some(suffix.to_string())
            };

            // If ticker exists, use it directly
            if let Some(ticker) = security.ticker.as_ref() {
                return Some(QuoteSuggestion {
                    security_id: security.id,
                    security_name: security.name.clone(),
                    isin: security.isin.clone(),
                    ticker: security.ticker.clone(),
                    suggested_feed: "YAHOO".to_string(),
                    suggested_feed_url: feed_url,
                    suggested_ticker: None,
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

            // No ticker - suggest one based on ISIN (WKN = chars 6-11)
            // or derive from security name
            let suggested_ticker = derive_ticker_from_isin_or_name(isin, &security.name);

            return Some(QuoteSuggestion {
                security_id: security.id,
                security_name: security.name.clone(),
                isin: security.isin.clone(),
                ticker: None,
                suggested_feed: "YAHOO".to_string(),
                suggested_feed_url: feed_url,
                suggested_ticker: Some(suggested_ticker.clone()),
                confidence: 0.70, // Lower confidence without ticker
                reason: format!(
                    "ISIN {} → {} (Yahoo Finance) - Ticker-Vorschlag: {}{}",
                    country_code,
                    description,
                    suggested_ticker,
                    suffix
                ),
            });
        }
    }

    // Unknown country code - still suggest Yahoo without suffix
    if let Some(ticker) = security.ticker.as_ref() {
        return Some(QuoteSuggestion {
            security_id: security.id,
            security_name: security.name.clone(),
            isin: security.isin.clone(),
            ticker: security.ticker.clone(),
            suggested_feed: "YAHOO".to_string(),
            suggested_feed_url: None,
            suggested_ticker: None,
            confidence: 0.60,
            reason: format!("ISIN {} - Yahoo Finance {} (unbekannter Markt)", country_code, ticker),
        });
    }

    // No ticker, unknown country - suggest ticker based on name
    let suggested_ticker = derive_ticker_from_isin_or_name(isin, &security.name);

    Some(QuoteSuggestion {
        security_id: security.id,
        security_name: security.name.clone(),
        isin: security.isin.clone(),
        ticker: None,
        suggested_feed: "YAHOO".to_string(),
        suggested_feed_url: None,
        suggested_ticker: Some(suggested_ticker.clone()),
        confidence: 0.50, // Low confidence
        reason: format!(
            "ISIN {} - Yahoo Finance (unbekannter Markt) - Ticker-Vorschlag: {}",
            country_code,
            suggested_ticker
        ),
    })
}

/// Derive a ticker symbol from ISIN or security name
fn derive_ticker_from_isin_or_name(isin: &str, name: &str) -> String {
    // Try to extract WKN from ISIN (chars 6-11, excluding check digit)
    // German ISINs: DE000XXXXXX0 where XXXXXX is often the WKN
    if isin.len() >= 12 {
        let potential_wkn = &isin[6..12];
        // If it looks like a valid WKN (alphanumeric), use it
        if potential_wkn.chars().all(|c| c.is_ascii_alphanumeric()) {
            return potential_wkn.to_uppercase();
        }
    }

    // Fallback: derive from name using the common function
    derive_ticker_from_name(name)
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
                    suggested_ticker: None,
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
            suggested_ticker: None,
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

    #[test]
    fn test_physical_gold_by_name() {
        // Physical gold without ISIN -> Gold Futures
        let security = make_security(8, "Gold physisch", None, None, Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("GC=F".to_string()));
        assert!(suggestion.confidence >= 0.90);
    }

    #[test]
    fn test_xetra_gold_etc() {
        // Xetra-Gold ETC with ISIN -> normal ISIN-based suggestion (NOT futures!)
        // ETCs have their own price per share
        let security = make_security(9, "Xetra-Gold", Some("DE000A0S9GB0"), Some("4GLD"), Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        // Should use .DE suffix for German ISIN, NOT GC=F futures
        assert_eq!(suggestion.suggested_feed_url, Some(".DE".to_string()));
    }

    #[test]
    fn test_euwax_gold_etc() {
        // EUWAX Gold ETC with ISIN -> normal ISIN-based suggestion
        let security = make_security(10, "EUWAX Gold II", Some("DE000EWG2LD7"), Some("EWG2LD"), Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some(".DE".to_string()));
    }

    #[test]
    fn test_physical_silver_by_name() {
        // Physical silver without ISIN -> Silver Futures
        let security = make_security(11, "Silber physisch", None, None, Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("SI=F".to_string()));
    }

    #[test]
    fn test_platinum_barren() {
        // Physical platinum without ISIN -> Platinum Futures
        let security = make_security(12, "Platin Barren", None, None, Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("PL=F".to_string()));
    }

    #[test]
    fn test_gold_futures_ticker() {
        // Explicit futures ticker -> Gold Futures
        let security = make_security(13, "Gold Futures", None, Some("GC=F"), Some("USD"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("GC=F".to_string()));
    }

    #[test]
    fn test_gold_ticker_xau() {
        // XAU ticker (spot gold) -> Gold Futures
        let security = make_security(14, "Spot Gold", None, Some("XAUUSD"), Some("USD"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("GC=F".to_string()));
    }

    #[test]
    fn test_gold_muenze() {
        // Gold coin without ISIN -> Gold Futures
        let security = make_security(15, "Goldmünze Krügerrand", None, None, Some("EUR"));
        let suggestion = suggest_quote_provider(&security).unwrap();

        assert_eq!(suggestion.suggested_feed, "YAHOO");
        assert_eq!(suggestion.suggested_feed_url, Some("GC=F".to_string()));
    }
}
