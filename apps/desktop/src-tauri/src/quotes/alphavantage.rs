//! Alpha Vantage Quote Provider
//!
//! Fetches stock and ETF prices from Alpha Vantage API.
//! Free tier: 25 API calls/day, Premium plans available.
//! API key required - get one at https://www.alphavantage.co/support/#api-key

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const BASE_URL: &str = "https://www.alphavantage.co/query";

/// Global Quote response
#[derive(Debug, Deserialize)]
struct GlobalQuoteResponse {
    #[serde(rename = "Global Quote")]
    global_quote: Option<GlobalQuote>,
    #[serde(rename = "Note")]
    note: Option<String>,
    #[serde(rename = "Error Message")]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GlobalQuote {
    #[serde(rename = "01. symbol")]
    symbol: String,
    #[serde(rename = "02. open")]
    open: String,
    #[serde(rename = "03. high")]
    high: String,
    #[serde(rename = "04. low")]
    low: String,
    #[serde(rename = "05. price")]
    price: String,
    #[serde(rename = "06. volume")]
    volume: String,
    #[serde(rename = "07. latest trading day")]
    latest_trading_day: String,
    #[serde(rename = "08. previous close")]
    previous_close: String,
    #[serde(rename = "09. change")]
    change: String,
    #[serde(rename = "10. change percent")]
    change_percent: String,
}

/// Time Series Daily response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TimeSeriesDailyResponse {
    #[serde(rename = "Meta Data")]
    meta_data: Option<MetaData>,
    #[serde(rename = "Time Series (Daily)")]
    time_series: Option<HashMap<String, DailyData>>,
    #[serde(rename = "Note")]
    note: Option<String>,
    #[serde(rename = "Error Message")]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MetaData {
    #[serde(rename = "1. Information")]
    information: String,
    #[serde(rename = "2. Symbol")]
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct DailyData {
    #[serde(rename = "1. open")]
    open: String,
    #[serde(rename = "2. high")]
    high: String,
    #[serde(rename = "3. low")]
    low: String,
    #[serde(rename = "4. close")]
    close: String,
    #[serde(rename = "5. volume")]
    volume: String,
}

/// Symbol Search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "bestMatches")]
    best_matches: Option<Vec<SearchMatch>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchMatch {
    #[serde(rename = "1. symbol")]
    pub symbol: String,
    #[serde(rename = "2. name")]
    pub name: String,
    #[serde(rename = "3. type")]
    pub security_type: String,
    #[serde(rename = "4. region")]
    pub region: String,
    #[serde(rename = "8. currency")]
    pub currency: String,
}

/// Fetch current quote for a symbol
///
/// # Arguments
/// * `symbol` - Stock symbol (e.g., "AAPL", "MSFT")
/// * `api_key` - Alpha Vantage API key
pub async fn fetch_quote(symbol: &str, api_key: &str) -> Result<LatestQuote> {
    if api_key.is_empty() {
        return Err(anyhow!("Alpha Vantage API key required"));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let url = format!(
        "{}?function=GLOBAL_QUOTE&symbol={}&apikey={}",
        BASE_URL, symbol, api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Alpha Vantage API error: {}", response.status()));
    }

    let data: GlobalQuoteResponse = response.json().await?;

    // Check for rate limiting or errors
    if let Some(note) = data.note {
        if note.contains("call frequency") {
            return Err(anyhow!("Alpha Vantage rate limit exceeded"));
        }
    }

    if let Some(error) = data.error_message {
        return Err(anyhow!("Alpha Vantage error: {}", error));
    }

    let quote_data = data
        .global_quote
        .ok_or_else(|| anyhow!("No quote data for symbol {}", symbol))?;

    let price: f64 = quote_data.price.parse().unwrap_or(0.0);
    let high: f64 = quote_data.high.parse().unwrap_or(0.0);
    let low: f64 = quote_data.low.parse().unwrap_or(0.0);
    let open: f64 = quote_data.open.parse().unwrap_or(0.0);
    let volume: i64 = quote_data.volume.parse().unwrap_or(0);

    let date = NaiveDate::parse_from_str(&quote_data.latest_trading_day, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());

    Ok(LatestQuote {
        symbol: quote_data.symbol,
        name: None,
        currency: Some("USD".to_string()), // Alpha Vantage returns USD by default
        quote: Quote {
            date,
            close: price,
            high: Some(high),
            low: Some(low),
            open: Some(open),
            volume: Some(volume),
        },
    })
}

/// Fetch historical daily prices
///
/// # Arguments
/// * `symbol` - Stock symbol
/// * `api_key` - Alpha Vantage API key
/// * `from` - Start date
/// * `to` - End date
/// * `full` - If true, fetch full history (20+ years), otherwise compact (100 days)
pub async fn fetch_historical(
    symbol: &str,
    api_key: &str,
    from: NaiveDate,
    to: NaiveDate,
    full: bool,
) -> Result<Vec<Quote>> {
    if api_key.is_empty() {
        return Err(anyhow!("Alpha Vantage API key required"));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let output_size = if full { "full" } else { "compact" };
    let url = format!(
        "{}?function=TIME_SERIES_DAILY&symbol={}&outputsize={}&apikey={}",
        BASE_URL, symbol, output_size, api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Alpha Vantage API error: {}", response.status()));
    }

    let data: TimeSeriesDailyResponse = response.json().await?;

    if let Some(note) = data.note {
        if note.contains("call frequency") {
            return Err(anyhow!("Alpha Vantage rate limit exceeded"));
        }
    }

    if let Some(error) = data.error_message {
        return Err(anyhow!("Alpha Vantage error: {}", error));
    }

    let time_series = data
        .time_series
        .ok_or_else(|| anyhow!("No time series data for symbol {}", symbol))?;

    let mut quotes: Vec<Quote> = time_series
        .iter()
        .filter_map(|(date_str, daily)| {
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
            if date < from || date > to {
                return None;
            }

            Some(Quote {
                date,
                close: daily.close.parse().ok()?,
                high: daily.high.parse().ok(),
                low: daily.low.parse().ok(),
                open: daily.open.parse().ok(),
                volume: daily.volume.parse().ok(),
            })
        })
        .collect();

    quotes.sort_by_key(|q| q.date);
    Ok(quotes)
}

/// Search for symbols
///
/// # Arguments
/// * `query` - Search keywords
/// * `api_key` - Alpha Vantage API key
pub async fn search(query: &str, api_key: &str) -> Result<Vec<SearchMatch>> {
    if api_key.is_empty() {
        return Err(anyhow!("Alpha Vantage API key required"));
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let url = format!(
        "{}?function=SYMBOL_SEARCH&keywords={}&apikey={}",
        BASE_URL, query, api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Alpha Vantage search error: {}", response.status()));
    }

    let data: SearchResponse = response.json().await?;

    Ok(data.best_matches.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_fetch_quote() {
        let api_key = std::env::var("ALPHAVANTAGE_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return;
        }

        let result = fetch_quote("AAPL", &api_key).await;
        assert!(result.is_ok());
    }
}
