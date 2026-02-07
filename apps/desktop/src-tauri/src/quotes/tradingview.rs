//! TradingView Quote Provider
//!
//! Uses TradingView's internal scanner API to fetch quote data.
//! Note: This is an unofficial API and may change without notice.
//!
//! Supports:
//! - Current quotes (Latest)
//! - Historical data (Daily OHLCV)
//! - Global markets (Stocks, ETFs, Crypto, Forex)

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, Utc};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};

const SCANNER_URL: &str = "https://scanner.tradingview.com/global/scan";
const SYMBOL_SEARCH_URL: &str = "https://symbol-search.tradingview.com/symbol_search/v3/";

/// Create HTTP client with appropriate headers
fn create_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15"),
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// Scanner request body
#[derive(Debug, Serialize)]
struct ScannerRequest {
    symbols: ScannerSymbols,
    columns: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ScannerSymbols {
    tickers: Vec<String>,
    query: ScannerQuery,
}

#[derive(Debug, Serialize)]
struct ScannerQuery {
    types: Vec<&'static str>,
}

/// Scanner response
#[derive(Debug, Deserialize)]
struct ScannerResponse {
    data: Vec<ScannerData>,
}

#[derive(Debug, Deserialize)]
struct ScannerData {
    #[serde(rename = "s")]
    #[allow(dead_code)]
    symbol: String,
    #[serde(rename = "d")]
    data: Vec<serde_json::Value>,
}

/// Fetch current quote from TradingView
pub async fn fetch_quote(symbol: &str) -> Result<LatestQuote> {
    let client = create_client()?;

    // Normalize symbol format (TradingView uses EXCHANGE:SYMBOL format)
    let tv_symbol = normalize_symbol(symbol);

    log::debug!("Fetching TradingView quote for {} (normalized: {})", symbol, tv_symbol);

    let request_body = ScannerRequest {
        symbols: ScannerSymbols {
            tickers: vec![tv_symbol.clone()],
            query: ScannerQuery {
                types: vec!["stock", "fund", "etf", "index", "forex", "crypto"],
            },
        },
        columns: vec![
            "name",
            "close",
            "open",
            "high",
            "low",
            "volume",
            "currency",
            "change",
            "change_abs",
            "description",
        ],
    };

    let response = client
        .post(SCANNER_URL)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| anyhow!("TradingView request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::error!("TradingView API error for {}: {} - {}", symbol, status, body);
        return Err(anyhow!("TradingView HTTP error for {}: {}", symbol, status));
    }

    let scanner_response: ScannerResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse TradingView response for {}: {}", symbol, e))?;

    if scanner_response.data.is_empty() {
        return Err(anyhow!("No data found for symbol: {}", symbol));
    }

    let data = &scanner_response.data[0];
    parse_scanner_quote(symbol, data)
}

/// Parse quote data from scanner response
fn parse_scanner_quote(symbol: &str, data: &ScannerData) -> Result<LatestQuote> {
    // Column order: name, close, open, high, low, volume, currency, change, change_abs, description
    let values = &data.data;

    if values.len() < 7 {
        return Err(anyhow!("Insufficient data for symbol: {}", symbol));
    }

    let name = values.get(0).and_then(|v| v.as_str()).map(String::from);
    let close = values.get(1).and_then(|v| v.as_f64()).ok_or_else(|| anyhow!("Missing close price"))?;
    let open = values.get(2).and_then(|v| v.as_f64());
    let high = values.get(3).and_then(|v| v.as_f64());
    let low = values.get(4).and_then(|v| v.as_f64());
    let volume = values.get(5).and_then(|v| v.as_i64());
    let currency = values.get(6).and_then(|v| v.as_str()).map(String::from);

    let today = Utc::now().date_naive();

    Ok(LatestQuote {
        symbol: symbol.to_string(),
        name,
        currency,
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

/// Fetch historical data from TradingView
///
/// Note: TradingView's scanner API only provides current quotes. Historical OHLCV data
/// requires a WebSocket connection which is not implemented. This function fetches
/// the current quote and returns it as a single-day history as a fallback.
pub async fn fetch_historical(
    symbol: &str,
    _from: NaiveDate,
    _to: NaiveDate,
) -> Result<Vec<Quote>> {
    // TradingView historical data requires WebSocket (not HTTP). The scanner API
    // only provides current quotes. Return current quote as single-day fallback.
    log::debug!("TradingView historical requested for {} — returning current quote only (WebSocket not implemented)", symbol);

    match fetch_quote(symbol).await {
        Ok(latest) => Ok(vec![latest.quote]),
        Err(e) => {
            log::warn!("TradingView historical fallback failed for {}: {}", symbol, e);
            Ok(vec![])
        }
    }
}

/// Search for symbols on TradingView
pub async fn search_symbols(query: &str, limit: usize) -> Result<Vec<SymbolSearchResult>> {
    let client = create_client()?;

    let url = format!(
        "{}?text={}&hl=1&exchange=&lang=en&search_type=&domain=production&sort_by_country=US",
        SYMBOL_SEARCH_URL,
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("TradingView search failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("TradingView search HTTP error: {}", response.status()));
    }

    let data: SymbolSearchResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse search response: {}", e))?;

    Ok(data.symbols.into_iter().take(limit).collect())
}

#[derive(Debug, Deserialize)]
struct SymbolSearchResponse {
    symbols: Vec<SymbolSearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SymbolSearchResult {
    pub symbol: String,
    pub description: Option<String>,
    pub exchange: Option<String>,
    #[serde(rename = "type")]
    pub security_type: Option<String>,
    pub currency_code: Option<String>,
    pub provider_id: Option<String>,
}

/// Normalize symbol to TradingView format
///
/// TradingView uses EXCHANGE:SYMBOL format (e.g., "NASDAQ:AAPL", "XETR:SAP")
fn normalize_symbol(symbol: &str) -> String {
    // If already in EXCHANGE:SYMBOL format, return as-is
    if symbol.contains(':') {
        return symbol.to_uppercase();
    }

    // Try to infer exchange from symbol patterns
    let symbol_upper = symbol.to_uppercase();

    // German stocks (ending with .DE or containing German patterns)
    if symbol_upper.ends_with(".DE") {
        let base = symbol_upper.trim_end_matches(".DE");
        return format!("XETR:{}", base);
    }

    // UK stocks
    if symbol_upper.ends_with(".L") {
        let base = symbol_upper.trim_end_matches(".L");
        return format!("LSE:{}", base);
    }

    // Paris stocks
    if symbol_upper.ends_with(".PA") {
        let base = symbol_upper.trim_end_matches(".PA");
        return format!("EURONEXT:{}", base);
    }

    // Swiss stocks
    if symbol_upper.ends_with(".SW") {
        let base = symbol_upper.trim_end_matches(".SW");
        return format!("SIX:{}", base);
    }

    // Crypto (contains USD, EUR, etc.)
    if symbol_upper.contains("USD") || symbol_upper.contains("EUR") || symbol_upper.contains("BTC") {
        return format!("BINANCE:{}", symbol_upper.replace("-", "").replace("/", ""));
    }

    // Default: assume US stock on NASDAQ or NYSE
    // TradingView will resolve the correct exchange
    symbol_upper
}

/// Get TradingView chart URL for a symbol (for display purposes)
pub fn get_chart_url(symbol: &str) -> String {
    let tv_symbol = normalize_symbol(symbol);
    format!("https://www.tradingview.com/chart/?symbol={}", urlencoding::encode(&tv_symbol))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_symbol() {
        assert_eq!(normalize_symbol("AAPL"), "AAPL");
        assert_eq!(normalize_symbol("SAP.DE"), "XETR:SAP");
        assert_eq!(normalize_symbol("HSBA.L"), "LSE:HSBA");
        assert_eq!(normalize_symbol("NASDAQ:MSFT"), "NASDAQ:MSFT");
        assert_eq!(normalize_symbol("BTC-USD"), "BINANCE:BTCUSD");
    }
}
