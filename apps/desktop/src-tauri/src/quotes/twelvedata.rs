//! Twelve Data Quote Provider
//!
//! Fetches stock prices from Twelve Data API.
//! Free tier: 800 API credits/day, 8 requests/minute
//! Supports Swiss stocks (SIX), European markets, and more.
//! API key required - get one at https://twelvedata.com/pricing
//!
//! Documentation: https://twelvedata.com/docs

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://api.twelvedata.com";

/// Quote response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QuoteResponse {
    symbol: Option<String>,
    name: Option<String>,
    exchange: Option<String>,
    currency: Option<String>,
    datetime: Option<String>,
    open: Option<String>,
    high: Option<String>,
    low: Option<String>,
    close: Option<String>,
    volume: Option<String>,
    previous_close: Option<String>,
    // Error fields
    code: Option<i32>,
    message: Option<String>,
    status: Option<String>,
}

/// Time series response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TimeSeriesResponse {
    meta: Option<TimeSeriesMeta>,
    values: Option<Vec<TimeSeriesValue>>,
    // Error fields
    code: Option<i32>,
    message: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TimeSeriesMeta {
    symbol: String,
    interval: String,
    currency: Option<String>,
    exchange: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimeSeriesValue {
    datetime: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: Option<String>,
}

/// Fetch current quote for a symbol
///
/// # Arguments
/// * `symbol` - Stock symbol with exchange suffix (e.g., "NESN:SIX" for Nestlé on SIX)
/// * `api_key` - Twelve Data API key
pub async fn fetch_quote(symbol: &str, api_key: &str) -> Result<LatestQuote> {
    if api_key.is_empty() {
        return Err(anyhow!("Twelve Data API key required"));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let url = format!(
        "{}/quote?symbol={}&apikey={}",
        BASE_URL, symbol, api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Twelve Data API error: {}", response.status()));
    }

    let data: QuoteResponse = response.json().await?;

    // Check for errors
    if let Some(code) = data.code {
        let msg = data.message.unwrap_or_else(|| format!("Error code {}", code));
        return Err(anyhow!("Twelve Data error: {}", msg));
    }

    let close: f64 = data.close
        .as_ref()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow!("No price data for {}", symbol))?;

    let date = data.datetime
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    Ok(LatestQuote {
        symbol: data.symbol.unwrap_or_else(|| symbol.to_string()),
        name: data.name,
        currency: data.currency,
        quote: Quote {
            date,
            close,
            high: data.high.as_ref().and_then(|s| s.parse().ok()),
            low: data.low.as_ref().and_then(|s| s.parse().ok()),
            open: data.open.as_ref().and_then(|s| s.parse().ok()),
            volume: data.volume.as_ref().and_then(|s| s.parse().ok()),
        },
    })
}

/// Fetch historical daily prices
///
/// # Arguments
/// * `symbol` - Stock symbol with optional exchange (e.g., "NESN:SIX")
/// * `api_key` - Twelve Data API key
/// * `from` - Start date
/// * `to` - End date
pub async fn fetch_historical(
    symbol: &str,
    api_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Quote>> {
    if api_key.is_empty() {
        return Err(anyhow!("Twelve Data API key required"));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let url = format!(
        "{}/time_series?symbol={}&interval=1day&start_date={}&end_date={}&apikey={}",
        BASE_URL,
        symbol,
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d"),
        api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Twelve Data API error: {}", response.status()));
    }

    let data: TimeSeriesResponse = response.json().await?;

    // Check for errors
    if let Some(code) = data.code {
        let msg = data.message.unwrap_or_else(|| format!("Error code {}", code));
        return Err(anyhow!("Twelve Data error: {}", msg));
    }

    let values = data.values
        .ok_or_else(|| anyhow!("No time series data for {}", symbol))?;

    let mut quotes: Vec<Quote> = values
        .iter()
        .filter_map(|v| {
            let date = NaiveDate::parse_from_str(&v.datetime, "%Y-%m-%d").ok()?;
            Some(Quote {
                date,
                close: v.close.parse().ok()?,
                high: v.high.parse().ok(),
                low: v.low.parse().ok(),
                open: v.open.parse().ok(),
                volume: v.volume.as_ref().and_then(|s| s.parse().ok()),
            })
        })
        .collect();

    quotes.sort_by_key(|q| q.date);
    Ok(quotes)
}

/// Convert Yahoo-style symbol to Twelve Data format
///
/// Examples:
/// - "NESN.SW" -> "NESN:SIX" (Swiss)
/// - "NOVN.SW" -> "NOVN:SIX" (Swiss)
/// - "BMW.DE" -> "BMW:XETR" (Germany XETRA)
/// - "AAPL" -> "AAPL" (US, no change)
pub fn convert_symbol(symbol: &str) -> String {
    if let Some(pos) = symbol.rfind('.') {
        let base = &symbol[..pos];
        let suffix = &symbol[pos + 1..];

        let exchange = match suffix.to_uppercase().as_str() {
            "SW" => "SIX",      // Swiss
            "DE" => "XETR",     // Germany XETRA
            "F" => "FSX",       // Frankfurt
            "PA" => "XPAR",     // Paris
            "AS" => "XAMS",     // Amsterdam
            "MI" => "XMIL",     // Milan
            "MC" => "XMAD",     // Madrid
            "L" => "LSE",       // London
            "TO" => "TSX",      // Toronto
            "AX" => "ASX",      // Australia
            "HK" => "HKEX",     // Hong Kong
            _ => return symbol.to_string(),
        };

        format!("{}:{}", base, exchange)
    } else {
        symbol.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_symbol() {
        assert_eq!(convert_symbol("NESN.SW"), "NESN:SIX");
        assert_eq!(convert_symbol("BMW.DE"), "BMW:XETR");
        assert_eq!(convert_symbol("AAPL"), "AAPL");
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_fetch_quote() {
        let api_key = std::env::var("TWELVEDATA_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return;
        }

        let result = fetch_quote("AAPL", &api_key).await;
        assert!(result.is_ok());
    }
}
