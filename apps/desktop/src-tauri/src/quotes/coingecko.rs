//! CoinGecko Quote Provider
//!
//! Fetches cryptocurrency prices from CoinGecko API.
//! - Public API: 10-30 calls/minute (no API key)
//! - Demo API: 30 calls/minute (free API key required)
//! - Pro API: Higher limits (paid)
//!
//! API documentation: https://docs.coingecko.com/

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// Public API (no key required, limited)
const PUBLIC_BASE_URL: &str = "https://api.coingecko.com/api/v3";
/// Demo API (free key, higher limits)
const DEMO_BASE_URL: &str = "https://api.coingecko.com/api/v3";
/// Pro API (paid)
const PRO_BASE_URL: &str = "https://pro-api.coingecko.com/api/v3";

/// Get base URL based on API key
fn get_base_url(api_key: Option<&str>) -> &'static str {
    match api_key {
        Some(key) if key.starts_with("CG-") => DEMO_BASE_URL,
        Some(_) => PRO_BASE_URL,
        None => PUBLIC_BASE_URL,
    }
}

/// CoinGecko simple price response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SimplePriceResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinPrice>,
}

/// CoinPrice uses flattened HashMap to handle any currency dynamically
#[derive(Debug, Deserialize)]
struct CoinPrice {
    #[serde(flatten)]
    values: HashMap<String, f64>,
}

/// CoinGecko market chart response for historical data
#[derive(Debug, Deserialize)]
struct MarketChartResponse {
    /// [[timestamp_ms, price], ...]
    prices: Vec<[f64; 2]>,
    /// [[timestamp_ms, volume], ...]
    #[serde(default)]
    total_volumes: Vec<[f64; 2]>,
}

/// CoinGecko coin info response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CoinInfoResponse {
    id: String,
    symbol: String,
    name: String,
}

/// Fetch current price for a cryptocurrency
///
/// # Arguments
/// * `coin_id` - CoinGecko coin ID (e.g., "bitcoin", "ethereum")
/// * `currency` - Target currency (e.g., "EUR", "USD")
/// * `api_key` - Optional API key (Demo: "CG-...", Pro: other)
pub async fn fetch_quote(coin_id: &str, currency: &str, api_key: Option<&str>) -> Result<LatestQuote> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let base_url = get_base_url(api_key);
    let currency_lower = currency.to_lowercase();
    let url = format!(
        "{}/simple/price?ids={}&vs_currencies={}&include_24hr_high=true&include_24hr_low=true&include_24hr_vol=true",
        base_url, coin_id, currency_lower
    );

    let mut request = client
        .get(&url)
        .header("Accept", "application/json");

    // Add API key header if provided
    if let Some(key) = api_key {
        if key.starts_with("CG-") {
            request = request.header("x-cg-demo-api-key", key);
        } else {
            request = request.header("x-cg-pro-api-key", key);
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "CoinGecko API error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let data: HashMap<String, CoinPrice> = response.json().await?;

    let coin_data = data
        .get(coin_id)
        .ok_or_else(|| anyhow!("Coin {} not found in response", coin_id))?;

    // Extract price, high, low, volume dynamically based on currency
    let price = coin_data.values.get(&currency_lower)
        .copied()
        .ok_or_else(|| anyhow!("No {} price for {}", currency.to_uppercase(), coin_id))?;

    let high = coin_data.values.get(&format!("{}_24h_high", currency_lower)).copied();
    let low = coin_data.values.get(&format!("{}_24h_low", currency_lower)).copied();
    let volume = coin_data.values.get(&format!("{}_24h_vol", currency_lower)).map(|v| *v as i64);

    let today = Utc::now().date_naive();

    Ok(LatestQuote {
        symbol: coin_id.to_string(),
        name: Some(coin_id.to_string()),
        currency: Some(currency.to_uppercase()),
        quote: Quote {
            date: today,
            close: price,
            high,
            low,
            open: None,
            volume,
        },
    })
}

/// Fetch historical prices for a cryptocurrency
///
/// # Arguments
/// * `coin_id` - CoinGecko coin ID
/// * `currency` - Target currency
/// * `from` - Start date
/// * `to` - End date
/// * `api_key` - Optional API key
pub async fn fetch_historical(
    coin_id: &str,
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
    api_key: Option<&str>,
) -> Result<Vec<Quote>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let base_url = get_base_url(api_key);
    let currency_lower = currency.to_lowercase();

    // Convert dates to Unix timestamps
    let from_ts = Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0).unwrap()).timestamp();
    let to_ts = Utc.from_utc_datetime(&to.and_hms_opt(23, 59, 59).unwrap()).timestamp();

    let url = format!(
        "{}/coins/{}/market_chart/range?vs_currency={}&from={}&to={}",
        base_url, coin_id, currency_lower, from_ts, to_ts
    );

    let mut request = client
        .get(&url)
        .header("Accept", "application/json");

    // Add API key header if provided
    if let Some(key) = api_key {
        if key.starts_with("CG-") {
            request = request.header("x-cg-demo-api-key", key);
        } else {
            request = request.header("x-cg-pro-api-key", key);
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "CoinGecko API error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let data: MarketChartResponse = response.json().await?;

    // Build volume lookup
    let volumes: HashMap<i64, f64> = data
        .total_volumes
        .iter()
        .map(|[ts, vol]| ((*ts / 1000.0) as i64, *vol))
        .collect();

    // Convert to daily quotes (CoinGecko returns data at various intervals)
    let mut daily_quotes: HashMap<NaiveDate, Quote> = HashMap::new();

    for [timestamp_ms, price] in data.prices {
        let timestamp = (timestamp_ms / 1000.0) as i64;
        let date = Utc.timestamp_opt(timestamp, 0)
            .single()
            .map(|dt| dt.date_naive())
            .unwrap_or(from);

        let volume = volumes.get(&timestamp).map(|v| *v as i64);

        // Keep the last price of each day
        daily_quotes.insert(
            date,
            Quote {
                date,
                close: price,
                high: None, // CoinGecko doesn't provide OHLC in this endpoint
                low: None,
                open: None,
                volume,
            },
        );
    }

    let mut quotes: Vec<Quote> = daily_quotes.into_values().collect();
    quotes.sort_by_key(|q| q.date);

    Ok(quotes)
}

/// Search for a cryptocurrency by name or symbol
#[allow(dead_code, private_interfaces)]
pub async fn search_coin(query: &str, api_key: Option<&str>) -> Result<Vec<CoinInfoResponse>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let base_url = get_base_url(api_key);
    let url = format!("{}/search?query={}", base_url, query);

    let mut request = client
        .get(&url)
        .header("Accept", "application/json");

    // Add API key header if provided
    if let Some(key) = api_key {
        if key.starts_with("CG-") {
            request = request.header("x-cg-demo-api-key", key);
        } else {
            request = request.header("x-cg-pro-api-key", key);
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("CoinGecko search error: {}", response.status()));
    }

    #[derive(Deserialize)]
    struct SearchResponse {
        coins: Vec<CoinInfoResponse>,
    }

    let data: SearchResponse = response.json().await?;
    Ok(data.coins)
}

/// Convert common crypto symbols to CoinGecko IDs
pub fn symbol_to_coin_id(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "BTC" | "BITCOIN" => Some("bitcoin"),
        "ETH" | "ETHEREUM" => Some("ethereum"),
        "BNB" => Some("binancecoin"),
        "XRP" | "RIPPLE" => Some("ripple"),
        "ADA" | "CARDANO" => Some("cardano"),
        "SOL" | "SOLANA" => Some("solana"),
        "DOT" | "POLKADOT" => Some("polkadot"),
        "DOGE" | "DOGECOIN" => Some("dogecoin"),
        "AVAX" | "AVALANCHE" => Some("avalanche-2"),
        "MATIC" | "POLYGON" => Some("matic-network"),
        "LINK" | "CHAINLINK" => Some("chainlink"),
        "LTC" | "LITECOIN" => Some("litecoin"),
        "UNI" | "UNISWAP" => Some("uniswap"),
        "ATOM" | "COSMOS" => Some("cosmos"),
        "XLM" | "STELLAR" => Some("stellar"),
        "ALGO" | "ALGORAND" => Some("algorand"),
        "VET" | "VECHAIN" => Some("vechain"),
        "FIL" | "FILECOIN" => Some("filecoin"),
        "AAVE" => Some("aave"),
        "XMR" | "MONERO" => Some("monero"),
        "EOS" => Some("eos"),
        "THETA" => Some("theta-token"),
        "XTZ" | "TEZOS" => Some("tezos"),
        "NEAR" => Some("near"),
        "FTM" | "FANTOM" => Some("fantom"),
        "SAND" | "SANDBOX" => Some("the-sandbox"),
        "MANA" | "DECENTRALAND" => Some("decentraland"),
        "APE" | "APECOIN" => Some("apecoin"),
        "CRO" | "CRONOS" => Some("crypto-com-chain"),
        "SHIB" | "SHIBAINU" => Some("shiba-inu"),
        "USDT" | "TETHER" => Some("tether"),
        "USDC" => Some("usd-coin"),
        "DAI" => Some("dai"),
        "BUSD" => Some("binance-usd"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_mapping() {
        assert_eq!(symbol_to_coin_id("BTC"), Some("bitcoin"));
        assert_eq!(symbol_to_coin_id("eth"), Some("ethereum"));
        assert_eq!(symbol_to_coin_id("UNKNOWN"), None);
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_fetch_bitcoin_price() {
        let result = fetch_quote("bitcoin", "EUR", None).await;
        assert!(result.is_ok());
        let quote = result.unwrap();
        assert!(quote.quote.close > 0.0);
    }
}
