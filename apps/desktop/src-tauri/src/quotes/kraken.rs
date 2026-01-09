//! Kraken Quote Provider
//!
//! Fetches cryptocurrency prices from Kraken Exchange REST API.
//! - Public API: No authentication required for market data
//! - Rate limit: ~1 request/second for public endpoints
//!
//! API documentation: https://docs.kraken.com/rest/
//!
//! Kraken pair naming conventions:
//! - Crypto assets: X prefix (XXBT = Bitcoin, XETH = Ethereum)
//! - Fiat currencies: Z prefix (ZEUR = Euro, ZUSD = USD)
//! - Examples: BTC/EUR = XXBTZEUR, ETH/USD = XETHZUSD

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const BASE_URL: &str = "https://api.kraken.com/0/public";

/// Kraken API response wrapper
#[derive(Debug, Deserialize)]
struct KrakenResponse<T> {
    error: Vec<String>,
    result: Option<T>,
}

/// Ticker data from Kraken
/// Fields: a=ask, b=bid, c=last trade, v=volume, p=vwap, t=trades, l=low, h=high, o=open
#[derive(Debug, Deserialize)]
struct TickerData {
    /// Last trade closed [price, lot volume]
    c: Vec<String>,
    /// Volume [today, last 24h]
    v: Vec<String>,
    /// Low [today, last 24h]
    l: Vec<String>,
    /// High [today, last 24h]
    h: Vec<String>,
    /// Opening price today
    o: String,
}

/// OHLC response result
#[derive(Debug, Deserialize)]
struct OhlcResult {
    #[serde(flatten)]
    pairs: HashMap<String, serde_json::Value>,
}

/// Create HTTP client
fn create_client() -> Result<Client> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// Fetch current price for a cryptocurrency pair
///
/// # Arguments
/// * `pair` - Kraken pair (e.g., "XXBTZEUR", "XETHZUSD") or simplified (e.g., "BTCEUR")
/// * `currency` - Target currency for the quote (e.g., "EUR", "USD")
pub async fn fetch_quote(pair: &str, currency: &str) -> Result<LatestQuote> {
    let client = create_client()?;

    // Normalize pair format
    let kraken_pair = normalize_pair(pair, currency);

    let url = format!("{}/Ticker?pair={}", BASE_URL, kraken_pair);
    log::debug!("Fetching Kraken ticker for {} from {}", kraken_pair, url);

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", kraken_pair, e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Kraken API error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let data: KrakenResponse<HashMap<String, TickerData>> = response.json().await
        .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

    // Check for API errors
    if !data.error.is_empty() {
        return Err(anyhow!("Kraken API error: {}", data.error.join(", ")));
    }

    let result = data.result.ok_or_else(|| anyhow!("No result in response"))?;

    // Get the ticker data (Kraken may use different key than requested)
    let ticker = result.values().next()
        .ok_or_else(|| anyhow!("No ticker data for {}", kraken_pair))?;

    // Parse prices
    let close = ticker.c.first()
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| anyhow!("Invalid close price"))?;

    let open = ticker.o.parse::<f64>().ok();
    let high = ticker.h.first().and_then(|s| s.parse::<f64>().ok());
    let low = ticker.l.first().and_then(|s| s.parse::<f64>().ok());
    let volume = ticker.v.get(1) // Last 24h volume
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v as i64);

    let today = Utc::now().date_naive();
    let symbol = extract_base_asset(pair);

    Ok(LatestQuote {
        symbol: symbol.to_string(),
        name: Some(get_asset_name(&symbol)),
        currency: Some(currency.to_uppercase()),
        quote: Quote {
            date: today,
            close,
            high,
            low,
            open,
            volume,
        },
    })
}

/// Fetch historical OHLC data for a cryptocurrency pair
///
/// # Arguments
/// * `pair` - Kraken pair
/// * `currency` - Target currency
/// * `from` - Start date
/// * `to` - End date
///
/// Note: Kraken only returns the 720 most recent entries
pub async fn fetch_historical(
    pair: &str,
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Quote>> {
    let client = create_client()?;

    // Normalize pair format
    let kraken_pair = normalize_pair(pair, currency);

    // Calculate since timestamp (Kraken returns data after this time)
    let since_ts = Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0).unwrap()).timestamp();

    // Use daily interval (1440 minutes)
    let url = format!(
        "{}/OHLC?pair={}&interval=1440&since={}",
        BASE_URL, kraken_pair, since_ts
    );
    log::debug!("Fetching Kraken OHLC for {} from {}", kraken_pair, url);

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", kraken_pair, e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Kraken API error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let data: KrakenResponse<OhlcResult> = response.json().await
        .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

    // Check for API errors
    if !data.error.is_empty() {
        return Err(anyhow!("Kraken API error: {}", data.error.join(", ")));
    }

    let result = data.result.ok_or_else(|| anyhow!("No result in response"))?;

    // Find the OHLC array (skip "last" key)
    let ohlc_data = result.pairs.iter()
        .find(|(k, _)| *k != "last")
        .and_then(|(_, v)| v.as_array())
        .ok_or_else(|| anyhow!("No OHLC data found"))?;

    let mut quotes = Vec::new();

    for entry in ohlc_data {
        let arr = entry.as_array().ok_or_else(|| anyhow!("Invalid OHLC entry"))?;

        if arr.len() < 5 {
            continue;
        }

        let timestamp = arr[0].as_i64().unwrap_or(0);
        let date = Utc.timestamp_opt(timestamp, 0)
            .single()
            .map(|dt| dt.date_naive())
            .unwrap_or(from);

        // Skip if outside date range
        if date < from || date > to {
            continue;
        }

        let open = arr[1].as_str().and_then(|s| s.parse::<f64>().ok());
        let high = arr[2].as_str().and_then(|s| s.parse::<f64>().ok());
        let low = arr[3].as_str().and_then(|s| s.parse::<f64>().ok());
        let close = arr[4].as_str().and_then(|s| s.parse::<f64>().ok());
        let volume = arr[6].as_str().and_then(|s| s.parse::<f64>().ok()).map(|v| v as i64);

        if let Some(close) = close {
            quotes.push(Quote {
                date,
                close,
                high,
                low,
                open,
                volume,
            });
        }
    }

    // Sort by date
    quotes.sort_by_key(|q| q.date);

    Ok(quotes)
}

/// Normalize pair format to Kraken's format
///
/// Converts various formats to Kraken's internal naming:
/// - "BTCEUR" -> "XXBTZEUR"
/// - "BTC/EUR" -> "XXBTZEUR"
/// - "XBT/EUR" -> "XXBTZEUR"
/// - "XXBTZEUR" -> "XXBTZEUR" (already correct)
fn normalize_pair(pair: &str, currency: &str) -> String {
    let pair_upper = pair.to_uppercase().replace('/', "");

    // If already in Kraken format (starts with X and contains Z for fiat)
    if pair_upper.starts_with('X') && (pair_upper.contains("ZEUR") || pair_upper.contains("ZUSD")) {
        return pair_upper;
    }

    // Extract base asset from pair
    let base = extract_base_asset(&pair_upper);

    // Convert to Kraken format
    let kraken_base = symbol_to_kraken_asset(&base);
    let kraken_quote = currency_to_kraken(&currency.to_uppercase());

    format!("{}{}", kraken_base, kraken_quote)
}

/// Extract base asset from a pair string
fn extract_base_asset(pair: &str) -> String {
    let pair = pair.to_uppercase().replace('/', "");

    // Common quote currencies to strip
    let quotes = ["EUR", "USD", "GBP", "CHF", "JPY", "CAD", "AUD"];

    for quote in quotes {
        if pair.ends_with(quote) {
            return pair[..pair.len() - quote.len()].to_string();
        }
    }

    // Handle Kraken's prefixed format
    if pair.starts_with('X') && pair.len() > 4 {
        // XXBTZEUR -> XBT, XETHZEUR -> ETH
        if pair.contains("ZEUR") || pair.contains("ZUSD") {
            let z_pos = pair.find('Z').unwrap_or(pair.len());
            return pair[1..z_pos].to_string();
        }
    }

    pair
}

/// Convert common crypto symbols to Kraken's internal asset codes
fn symbol_to_kraken_asset(symbol: &str) -> String {
    match symbol.to_uppercase().as_str() {
        // Bitcoin uses XBT on Kraken (ISO 4217 style)
        "BTC" | "BITCOIN" => "XXBT".to_string(),
        "XBT" => "XXBT".to_string(),
        // Ethereum
        "ETH" | "ETHEREUM" => "XETH".to_string(),
        // Other major cryptos (X prefix)
        "XRP" | "RIPPLE" => "XXRP".to_string(),
        "LTC" | "LITECOIN" => "XLTC".to_string(),
        "XLM" | "STELLAR" => "XXLM".to_string(),
        "XMR" | "MONERO" => "XXMR".to_string(),
        "ETC" => "XETC".to_string(),
        "REP" => "XREP".to_string(),
        "ZEC" => "XZEC".to_string(),
        "MLN" => "XMLN".to_string(),
        // Newer assets without X prefix
        "ADA" | "CARDANO" => "ADA".to_string(),
        "SOL" | "SOLANA" => "SOL".to_string(),
        "DOT" | "POLKADOT" => "DOT".to_string(),
        "ATOM" | "COSMOS" => "ATOM".to_string(),
        "LINK" | "CHAINLINK" => "LINK".to_string(),
        "DOGE" | "DOGECOIN" => "XDG".to_string(), // Kraken uses XDG for Doge
        "AVAX" | "AVALANCHE" => "AVAX".to_string(),
        "MATIC" | "POLYGON" => "MATIC".to_string(),
        "UNI" | "UNISWAP" => "UNI".to_string(),
        "AAVE" => "AAVE".to_string(),
        "ALGO" | "ALGORAND" => "ALGO".to_string(),
        "FIL" | "FILECOIN" => "FIL".to_string(),
        "SAND" | "SANDBOX" => "SAND".to_string(),
        "MANA" | "DECENTRALAND" => "MANA".to_string(),
        "APE" | "APECOIN" => "APE".to_string(),
        "SHIB" | "SHIBAINU" => "SHIB".to_string(),
        "NEAR" => "NEAR".to_string(),
        "FTM" | "FANTOM" => "FTM".to_string(),
        // Stablecoins
        "USDT" | "TETHER" => "USDT".to_string(),
        "USDC" => "USDC".to_string(),
        "DAI" => "DAI".to_string(),
        // Default: return as-is with X prefix if it looks like a crypto
        other => {
            if other.len() <= 4 {
                format!("X{}", other)
            } else {
                other.to_string()
            }
        }
    }
}

/// Convert currency code to Kraken's format
fn currency_to_kraken(currency: &str) -> String {
    match currency {
        "EUR" => "ZEUR".to_string(),
        "USD" => "ZUSD".to_string(),
        "GBP" => "ZGBP".to_string(),
        "CAD" => "ZCAD".to_string(),
        "JPY" => "ZJPY".to_string(),
        "CHF" => "CHF".to_string(), // CHF doesn't use Z prefix on Kraken
        "AUD" => "ZAUD".to_string(),
        other => format!("Z{}", other),
    }
}

/// Get human-readable name for a crypto asset
fn get_asset_name(symbol: &str) -> String {
    match symbol.to_uppercase().as_str() {
        "BTC" | "XBT" | "XXBT" => "Bitcoin".to_string(),
        "ETH" | "XETH" => "Ethereum".to_string(),
        "XRP" | "XXRP" => "XRP".to_string(),
        "LTC" | "XLTC" => "Litecoin".to_string(),
        "ADA" => "Cardano".to_string(),
        "SOL" => "Solana".to_string(),
        "DOT" => "Polkadot".to_string(),
        "DOGE" | "XDG" => "Dogecoin".to_string(),
        "AVAX" => "Avalanche".to_string(),
        "MATIC" => "Polygon".to_string(),
        "LINK" => "Chainlink".to_string(),
        "UNI" => "Uniswap".to_string(),
        "ATOM" => "Cosmos".to_string(),
        "XLM" | "XXLM" => "Stellar".to_string(),
        "ALGO" => "Algorand".to_string(),
        "XMR" | "XXMR" => "Monero".to_string(),
        "AAVE" => "Aave".to_string(),
        "FIL" => "Filecoin".to_string(),
        "NEAR" => "NEAR Protocol".to_string(),
        "FTM" => "Fantom".to_string(),
        "SAND" => "The Sandbox".to_string(),
        "MANA" => "Decentraland".to_string(),
        "APE" => "ApeCoin".to_string(),
        "SHIB" => "Shiba Inu".to_string(),
        "USDT" => "Tether".to_string(),
        "USDC" => "USD Coin".to_string(),
        "DAI" => "Dai".to_string(),
        other => other.to_string(),
    }
}

/// Convert common crypto symbols to Kraken pair format
/// Used by the quote sync logic to map symbols to Kraken pairs
pub fn symbol_to_pair(symbol: &str, currency: &str) -> String {
    normalize_pair(symbol, currency)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pair() {
        assert_eq!(normalize_pair("BTC", "EUR"), "XXBTZEUR");
        assert_eq!(normalize_pair("BTCEUR", "EUR"), "XXBTZEUR");
        assert_eq!(normalize_pair("BTC/EUR", "EUR"), "XXBTZEUR");
        assert_eq!(normalize_pair("ETH", "USD"), "XETHZUSD");
        assert_eq!(normalize_pair("XXBTZEUR", "EUR"), "XXBTZEUR");
        assert_eq!(normalize_pair("SOL", "EUR"), "SOLZEUR");
        assert_eq!(normalize_pair("ADA", "EUR"), "ADAZEUR");
    }

    #[test]
    fn test_extract_base_asset() {
        assert_eq!(extract_base_asset("BTCEUR"), "BTC");
        assert_eq!(extract_base_asset("BTC/EUR"), "BTC");
        assert_eq!(extract_base_asset("XXBTZEUR"), "XBT");
        assert_eq!(extract_base_asset("XETHZUSD"), "ETH");
    }

    #[test]
    fn test_symbol_to_kraken_asset() {
        assert_eq!(symbol_to_kraken_asset("BTC"), "XXBT");
        assert_eq!(symbol_to_kraken_asset("ETH"), "XETH");
        assert_eq!(symbol_to_kraken_asset("SOL"), "SOL");
        assert_eq!(symbol_to_kraken_asset("DOGE"), "XDG");
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_fetch_bitcoin_price() {
        let result = fetch_quote("BTC", "EUR").await;
        assert!(result.is_ok(), "Failed to fetch BTC/EUR: {:?}", result.err());
        let quote = result.unwrap();
        assert!(quote.quote.close > 0.0);
        println!("BTC/EUR: {:.2} on {}", quote.quote.close, quote.quote.date);
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_fetch_historical() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let result = fetch_historical("BTC", "EUR", from, to).await;
        assert!(result.is_ok(), "Failed to fetch historical: {:?}", result.err());

        let quotes = result.unwrap();
        assert!(!quotes.is_empty());
        println!("Got {} historical quotes for BTC/EUR", quotes.len());
    }
}
