//! Provider Search for Symbol Validation
//!
//! Searches various quote providers to find matching symbols for securities.

use super::types::{ApiKeys, ProviderSearchResult, SecurityForValidation};
use crate::quotes::{coingecko, tradingview, yahoo};
use anyhow::Result;

/// Search all available providers for a security
///
/// Returns results from all providers sorted by confidence
pub async fn search_all_providers(
    security: &SecurityForValidation,
    api_keys: &ApiKeys,
) -> Vec<ProviderSearchResult> {
    let mut all_results = Vec::new();

    // Run searches in parallel
    let (yahoo_results, tv_results, cg_results) = tokio::join!(
        search_yahoo(security),
        search_tradingview(security),
        search_coingecko(security, api_keys.coingecko_api_key.as_deref())
    );

    // Collect results
    all_results.extend(yahoo_results.unwrap_or_default());
    all_results.extend(tv_results.unwrap_or_default());
    all_results.extend(cg_results.unwrap_or_default());

    // Sort by confidence descending
    all_results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

    all_results
}

/// Search Yahoo Finance
async fn search_yahoo(security: &SecurityForValidation) -> Result<Vec<ProviderSearchResult>> {
    let mut results = Vec::new();

    // Try ticker first
    if let Some(ref ticker) = security.ticker {
        if let Ok(yahoo_results) = yahoo::search(ticker).await {
            for r in yahoo_results {
                let confidence = calculate_confidence(security, &r.name, Some(&r.symbol), None);
                results.push(ProviderSearchResult {
                    provider: "YAHOO".to_string(),
                    symbol: r.symbol,
                    name: Some(r.name),
                    exchange: Some(r.exchange),
                    security_type: Some(r.security_type),
                    currency: None,
                    isin: None,
                    confidence,
                });
            }
        }
    }

    // Try ISIN
    if let Some(ref isin) = security.isin {
        if let Ok(yahoo_results) = yahoo::search(isin).await {
            for r in yahoo_results {
                let confidence = calculate_confidence(security, &r.name, Some(&r.symbol), None);
                // Boost confidence if we found via ISIN
                let boosted_confidence = (confidence + 0.2).min(1.0);
                results.push(ProviderSearchResult {
                    provider: "YAHOO".to_string(),
                    symbol: r.symbol,
                    name: Some(r.name),
                    exchange: Some(r.exchange),
                    security_type: Some(r.security_type),
                    currency: None,
                    isin: Some(isin.clone()),
                    confidence: boosted_confidence,
                });
            }
        }
    }

    // Try name
    if results.is_empty() {
        if let Ok(yahoo_results) = yahoo::search(&security.name).await {
            for r in yahoo_results.into_iter().take(5) {
                let confidence = calculate_confidence(security, &r.name, Some(&r.symbol), None);
                results.push(ProviderSearchResult {
                    provider: "YAHOO".to_string(),
                    symbol: r.symbol,
                    name: Some(r.name),
                    exchange: Some(r.exchange),
                    security_type: Some(r.security_type),
                    currency: None,
                    isin: None,
                    confidence,
                });
            }
        }
    }

    // Deduplicate by symbol
    results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    results.dedup_by(|a, b| a.symbol == b.symbol);

    Ok(results.into_iter().take(10).collect())
}

/// Search TradingView
async fn search_tradingview(security: &SecurityForValidation) -> Result<Vec<ProviderSearchResult>> {
    let mut results = Vec::new();

    // Try ticker
    if let Some(ref ticker) = security.ticker {
        if let Ok(tv_results) = tradingview::search_symbols(ticker, 10).await {
            for r in tv_results {
                let confidence = calculate_confidence(
                    security,
                    r.description.as_deref().unwrap_or(""),
                    Some(&r.symbol),
                    None,
                );
                results.push(ProviderSearchResult {
                    provider: "TRADINGVIEW".to_string(),
                    symbol: r.symbol,
                    name: r.description,
                    exchange: r.exchange,
                    security_type: r.security_type,
                    currency: r.currency_code,
                    isin: None,
                    confidence,
                });
            }
        }
    }

    // Try name if no ticker or no results
    if results.is_empty() {
        if let Ok(tv_results) = tradingview::search_symbols(&security.name, 10).await {
            for r in tv_results.into_iter().take(5) {
                let confidence = calculate_confidence(
                    security,
                    r.description.as_deref().unwrap_or(""),
                    Some(&r.symbol),
                    None,
                );
                results.push(ProviderSearchResult {
                    provider: "TRADINGVIEW".to_string(),
                    symbol: r.symbol,
                    name: r.description,
                    exchange: r.exchange,
                    security_type: r.security_type,
                    currency: r.currency_code,
                    isin: None,
                    confidence,
                });
            }
        }
    }

    // Deduplicate
    results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    results.dedup_by(|a, b| a.symbol == b.symbol);

    Ok(results.into_iter().take(10).collect())
}

/// Search CoinGecko (for cryptocurrencies)
async fn search_coingecko(
    security: &SecurityForValidation,
    _api_key: Option<&str>,
) -> Result<Vec<ProviderSearchResult>> {
    let mut results = Vec::new();

    // Check if this looks like a crypto security
    let name_lower = security.name.to_lowercase();
    let is_crypto = is_crypto_security(security);

    if !is_crypto {
        return Ok(results);
    }

    // Try to match by ticker/name to known coin IDs
    if let Some(ref ticker) = security.ticker {
        if let Some(coin_id) = coingecko::symbol_to_coin_id(ticker) {
            results.push(ProviderSearchResult {
                provider: "COINGECKO".to_string(),
                symbol: coin_id.to_string(),
                name: Some(ticker.to_uppercase()),
                exchange: None,
                security_type: Some("cryptocurrency".to_string()),
                currency: Some(security.currency.clone()),
                isin: None,
                confidence: 0.9,
            });
        }
    }

    // Try name for known coins
    let known_coins = [
        ("bitcoin", "bitcoin", "BTC"),
        ("ethereum", "ethereum", "ETH"),
        ("solana", "solana", "SOL"),
        ("cardano", "cardano", "ADA"),
        ("polkadot", "polkadot", "DOT"),
        ("dogecoin", "dogecoin", "DOGE"),
        ("avalanche", "avalanche-2", "AVAX"),
        ("polygon", "matic-network", "MATIC"),
        ("chainlink", "chainlink", "LINK"),
        ("litecoin", "litecoin", "LTC"),
        ("ripple", "ripple", "XRP"),
        ("stellar", "stellar", "XLM"),
    ];

    for (name_pattern, coin_id, symbol) in known_coins {
        if name_lower.contains(name_pattern) {
            if !results.iter().any(|r| r.symbol == coin_id) {
                results.push(ProviderSearchResult {
                    provider: "COINGECKO".to_string(),
                    symbol: coin_id.to_string(),
                    name: Some(symbol.to_string()),
                    exchange: None,
                    security_type: Some("cryptocurrency".to_string()),
                    currency: Some(security.currency.clone()),
                    isin: None,
                    confidence: 0.85,
                });
            }
        }
    }

    Ok(results)
}

/// Check if a security appears to be a cryptocurrency
fn is_crypto_security(security: &SecurityForValidation) -> bool {
    let name_lower = security.name.to_lowercase();
    let ticker_lower = security.ticker.as_deref().unwrap_or("").to_lowercase();

    // Check for common crypto keywords
    let crypto_keywords = [
        "bitcoin", "ethereum", "crypto", "coin", "token",
        "solana", "cardano", "polkadot", "dogecoin", "litecoin",
        "ripple", "stellar", "chainlink", "avalanche", "polygon",
    ];

    for keyword in crypto_keywords {
        if name_lower.contains(keyword) || ticker_lower.contains(keyword) {
            return true;
        }
    }

    // Check for crypto ticker patterns
    let crypto_tickers = ["BTC", "ETH", "SOL", "ADA", "DOT", "DOGE", "AVAX", "MATIC", "LINK", "LTC", "XRP", "XLM"];
    let ticker_upper = security.ticker.as_deref().unwrap_or("").to_uppercase();

    for ct in crypto_tickers {
        if ticker_upper == ct || ticker_upper.starts_with(&format!("{}-", ct)) || ticker_upper.starts_with(&format!("{}/", ct)) {
            return true;
        }
    }

    // No ISIN = likely crypto (most crypto doesn't have ISINs)
    if security.isin.is_none() && security.wkn.is_none() {
        // Additional checks for crypto-like names
        if name_lower.ends_with("coin") || name_lower.ends_with("token") {
            return true;
        }
    }

    false
}

/// Calculate match confidence between security and search result
fn calculate_confidence(
    security: &SecurityForValidation,
    result_name: &str,
    result_symbol: Option<&str>,
    result_isin: Option<&str>,
) -> f64 {
    let mut confidence = 0.0;
    let mut factors = 0;

    // ISIN match (highest priority)
    if let (Some(sec_isin), Some(res_isin)) = (&security.isin, result_isin) {
        if sec_isin.to_uppercase() == res_isin.to_uppercase() {
            return 0.95; // Almost certain match
        }
    }

    // Ticker match
    if let (Some(sec_ticker), Some(res_symbol)) = (&security.ticker, result_symbol) {
        let sec_ticker_clean = clean_ticker(sec_ticker);
        let res_symbol_clean = clean_ticker(res_symbol);

        if sec_ticker_clean == res_symbol_clean {
            confidence += 0.8;
            factors += 1;
        } else if res_symbol_clean.contains(&sec_ticker_clean) || sec_ticker_clean.contains(&res_symbol_clean) {
            confidence += 0.5;
            factors += 1;
        }
    }

    // Name similarity
    let name_similarity = calculate_name_similarity(&security.name, result_name);
    if name_similarity > 0.3 {
        confidence += name_similarity * 0.6;
        factors += 1;
    }

    // Calculate average
    if factors > 0 {
        confidence / factors as f64
    } else {
        0.1 // Low default confidence
    }
}

/// Clean ticker symbol for comparison
fn clean_ticker(ticker: &str) -> String {
    // Remove exchange suffixes like .DE, .SW, etc.
    let ticker = ticker.split('.').next().unwrap_or(ticker);
    // Remove exchange prefixes like XETR:, NASDAQ:, etc.
    let ticker = ticker.split(':').last().unwrap_or(ticker);
    ticker.to_uppercase().trim().to_string()
}

/// Calculate name similarity (simple Jaccard-like similarity)
fn calculate_name_similarity(name1: &str, name2: &str) -> f64 {
    let name1_lower = name1.to_lowercase();
    let name2_lower = name2.to_lowercase();

    let words1: std::collections::HashSet<&str> = name1_lower
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .collect();
    let words2: std::collections::HashSet<&str> = name2_lower
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Get the best exchange suffix for Yahoo based on security currency
pub fn get_yahoo_exchange_suffix(currency: &str, exchange: Option<&str>) -> Option<String> {
    // Use exchange if provided
    if let Some(exc) = exchange {
        let exc_upper = exc.to_uppercase();
        return match exc_upper.as_str() {
            "XETR" | "FRA" | "XFRA" | "GER" | "FRANKFURT" => Some(".DE".to_string()),
            "SWX" | "SIX" | "SWISS" => Some(".SW".to_string()),
            "LSE" | "LON" | "LONDON" => Some(".L".to_string()),
            "PAR" | "PARIS" | "EURONEXT" => Some(".PA".to_string()),
            "AMS" | "AMSTERDAM" => Some(".AS".to_string()),
            "MIL" | "MILAN" => Some(".MI".to_string()),
            "MAD" | "MADRID" | "BME" => Some(".MC".to_string()),
            "VIE" | "VIENNA" => Some(".VI".to_string()),
            "NYSE" | "NASDAQ" | "AMEX" | "US" => None, // US stocks don't need suffix
            _ => None,
        };
    }

    // Infer from currency
    match currency.to_uppercase().as_str() {
        "EUR" => Some(".DE".to_string()), // Default German exchange
        "CHF" => Some(".SW".to_string()),
        "GBP" | "GBX" => Some(".L".to_string()),
        "USD" => None, // US stocks
        _ => None,
    }
}

/// Get TradingView exchange prefix based on exchange name
pub fn get_tradingview_exchange_prefix(exchange: Option<&str>) -> Option<String> {
    exchange.map(|exc| {
        let exc_upper = exc.to_uppercase();
        match exc_upper.as_str() {
            "XETR" | "FRA" | "XFRA" | "GER" | "FRANKFURT" => "XETR".to_string(),
            "SWX" | "SIX" | "SWISS" => "SIX".to_string(),
            "LSE" | "LON" | "LONDON" => "LSE".to_string(),
            "PAR" | "PARIS" | "EURONEXT" => "EURONEXT".to_string(),
            "NYSE" => "NYSE".to_string(),
            "NASDAQ" => "NASDAQ".to_string(),
            _ => exc_upper,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_ticker() {
        assert_eq!(clean_ticker("AAPL"), "AAPL");
        assert_eq!(clean_ticker("SAP.DE"), "SAP");
        assert_eq!(clean_ticker("XETR:SAP"), "SAP");
        assert_eq!(clean_ticker("NASDAQ:AAPL"), "AAPL");
    }

    #[test]
    fn test_calculate_name_similarity() {
        assert!(calculate_name_similarity("Apple Inc.", "Apple Inc") > 0.8);
        assert!(calculate_name_similarity("Microsoft Corporation", "Microsoft Corp") > 0.5);
        assert!(calculate_name_similarity("Apple", "Google") < 0.3);
    }

    #[test]
    fn test_get_yahoo_exchange_suffix() {
        assert_eq!(get_yahoo_exchange_suffix("EUR", Some("XETR")), Some(".DE".to_string()));
        assert_eq!(get_yahoo_exchange_suffix("CHF", None), Some(".SW".to_string()));
        assert_eq!(get_yahoo_exchange_suffix("USD", None), None);
    }

    #[test]
    fn test_is_crypto_security() {
        let btc = SecurityForValidation {
            id: 1,
            name: "Bitcoin".to_string(),
            isin: None,
            wkn: None,
            ticker: Some("BTC".to_string()),
            currency: "EUR".to_string(),
            feed: None,
            feed_url: None,
            is_retired: false,
        };
        assert!(is_crypto_security(&btc));

        let apple = SecurityForValidation {
            id: 2,
            name: "Apple Inc.".to_string(),
            isin: Some("US0378331005".to_string()),
            wkn: None,
            ticker: Some("AAPL".to_string()),
            currency: "USD".to_string(),
            feed: None,
            feed_url: None,
            is_retired: false,
        };
        assert!(!is_crypto_security(&apple));
    }
}
