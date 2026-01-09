//! Finnhub Quote Provider
//!
//! Unterstützt:
//! - Aktuelle Kurse (Quote API)
//! - Historische Kurse (Stock Candles API - Premium erforderlich)
//!
//! API-Dokumentation: https://finnhub.io/docs/api

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

const BASE_URL: &str = "https://finnhub.io/api/v1";

/// Response from Finnhub Quote API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QuoteResponse {
    /// Current price
    c: f64,
    /// Change
    d: Option<f64>,
    /// Percent change
    dp: Option<f64>,
    /// High price of the day
    h: f64,
    /// Low price of the day
    l: f64,
    /// Open price of the day
    o: f64,
    /// Previous close price
    pc: f64,
    /// Timestamp (Unix)
    t: Option<i64>,
}

/// Response from Finnhub Stock Candles API
#[derive(Debug, Deserialize)]
struct CandleResponse {
    /// Close prices
    c: Option<Vec<f64>>,
    /// High prices
    h: Option<Vec<f64>>,
    /// Low prices
    l: Option<Vec<f64>>,
    /// Open prices
    o: Option<Vec<f64>>,
    /// Timestamps
    t: Option<Vec<i64>>,
    /// Volume
    v: Option<Vec<i64>>,
    /// Status ("ok" or "no_data")
    s: String,
}

/// HTTP Client mit API-Key Header erstellen
fn create_client(api_key: &str) -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Finnhub-Token",
        HeaderValue::from_str(api_key)
            .map_err(|e| anyhow!("Invalid API key format: {}", e))?,
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// Exchange-Suffixe entfernen (z.B. ".DE", ".F", ".PA" -> nur Ticker)
/// Finnhub verwendet nur US-Ticker ohne Exchange-Suffix
fn normalize_symbol(symbol: &str) -> String {
    // Common exchange suffixes to strip
    let suffixes = [".DE", ".F", ".PA", ".L", ".SW", ".AS", ".MI", ".MC", ".TO", ".AX", ".HK", ".T", ".SS", ".SZ"];

    let upper = symbol.to_uppercase();
    for suffix in suffixes {
        if upper.ends_with(suffix) {
            return upper[..upper.len() - suffix.len()].to_string();
        }
    }
    upper
}

/// Aktuellen Kurs abrufen
pub async fn fetch_quote(symbol: &str, api_key: &str) -> Result<LatestQuote> {
    if api_key.is_empty() {
        return Err(anyhow!("Finnhub API key required"));
    }

    let normalized_symbol = normalize_symbol(symbol);
    let url = format!("{}/quote?symbol={}", BASE_URL, urlencoding::encode(&normalized_symbol));
    log::info!("Fetching Finnhub quote for {} (normalized: {}) from {}", symbol, normalized_symbol, url);

    let client = create_client(api_key)?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::error!("Finnhub API error for {}: {} - {}", symbol, status, body);
        return Err(anyhow!("HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: QuoteResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse JSON for {}: {}", symbol, e))?;

    // Check if we got valid data (c=0 typically means no data)
    if data.c == 0.0 && data.h == 0.0 && data.l == 0.0 {
        return Err(anyhow!("No quote data available for {}", symbol));
    }

    // Use timestamp from response or current date
    let date = if let Some(ts) = data.t {
        chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| dt.date_naive())
            .unwrap_or_else(|| chrono::Utc::now().date_naive())
    } else {
        chrono::Utc::now().date_naive()
    };

    Ok(LatestQuote {
        symbol: symbol.to_string(),
        name: None, // Finnhub Quote API doesn't return name
        currency: Some("USD".to_string()), // Finnhub primarily returns USD prices
        quote: Quote {
            date,
            close: data.c,
            high: Some(data.h),
            low: Some(data.l),
            open: Some(data.o),
            volume: None, // Quote API doesn't return volume
        },
    })
}

/// Historische Kurse abrufen (erfordert Premium-Zugang)
pub async fn fetch_historical(
    symbol: &str,
    api_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Quote>> {
    if api_key.is_empty() {
        return Err(anyhow!("Finnhub API key required"));
    }

    let normalized_symbol = normalize_symbol(symbol);

    // Convert dates to Unix timestamps
    let from_ts = from
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);
    let to_ts = to
        .and_hms_opt(23, 59, 59)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);

    let url = format!(
        "{}/stock/candle?symbol={}&resolution=D&from={}&to={}",
        BASE_URL,
        urlencoding::encode(&normalized_symbol),
        from_ts,
        to_ts
    );
    log::info!("Fetching Finnhub historical for {} (normalized: {}) from {}", symbol, normalized_symbol, url);

    let client = create_client(api_key)?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::error!("Finnhub API error for {}: {} - {}", symbol, status, body);
        return Err(anyhow!("HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: CandleResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse JSON for {}: {}", symbol, e))?;

    if data.s != "ok" {
        return Err(anyhow!("No historical data available for {} (status: {})", symbol, data.s));
    }

    let timestamps = data.t.ok_or_else(|| anyhow!("Missing timestamps"))?;
    let closes = data.c.ok_or_else(|| anyhow!("Missing close prices"))?;
    let highs = data.h;
    let lows = data.l;
    let opens = data.o;
    let volumes = data.v;

    let mut quotes = Vec::new();

    for (i, ts) in timestamps.iter().enumerate() {
        let date = match chrono::DateTime::from_timestamp(*ts, 0) {
            Some(dt) => dt.date_naive(),
            None => continue,
        };

        let close = match closes.get(i) {
            Some(&c) => c,
            None => continue,
        };

        quotes.push(Quote {
            date,
            close,
            high: highs.as_ref().and_then(|h| h.get(i).copied()),
            low: lows.as_ref().and_then(|l| l.get(i).copied()),
            open: opens.as_ref().and_then(|o| o.get(i).copied()),
            volume: volumes.as_ref().and_then(|v| v.get(i).copied()),
        });
    }

    Ok(quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a valid Finnhub API key
    // Set FINNHUB_API_KEY environment variable to run them

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_fetch_apple_quote() {
        let api_key = std::env::var("FINNHUB_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            println!("Skipping test - FINNHUB_API_KEY not set");
            return;
        }

        let result = fetch_quote("AAPL", &api_key).await;
        assert!(result.is_ok(), "Failed to fetch AAPL: {:?}", result.err());

        let quote = result.unwrap();
        assert_eq!(quote.symbol, "AAPL");
        assert!(quote.quote.close > 0.0);
        println!("AAPL: ${:.2} on {}", quote.quote.close, quote.quote.date);
    }

    #[tokio::test]
    #[ignore] // Requires API key (Premium for historical)
    async fn test_fetch_historical() {
        let api_key = std::env::var("FINNHUB_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            println!("Skipping test - FINNHUB_API_KEY not set");
            return;
        }

        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let result = fetch_historical("AAPL", &api_key, from, to).await;
        // Note: This may fail without Premium access
        if let Ok(quotes) = result {
            println!("Got {} historical quotes for AAPL", quotes.len());
        } else {
            println!("Historical fetch failed (may require Premium): {:?}", result.err());
        }
    }
}
