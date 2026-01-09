//! Yahoo Finance Quote Provider
//!
//! Unterstützt:
//! - Aktuelle Kurse (Latest)
//! - Historische Kurse (Daily)
//! - Adjusted Close (Dividenden-bereinigt)

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

const BASE_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

/// HTTP Client mit korrekten Headers erstellen
fn create_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// Aktuellen Kurs abrufen
pub async fn fetch_quote(symbol: &str, _adjusted: bool) -> Result<LatestQuote> {
    let url = format!("{}?interval=1d&range=1d", symbol_url(symbol));
    log::debug!("Fetching Yahoo quote for {} from {}", symbol, url);

    let client = create_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::error!("Yahoo API error for {}: {} - {}", symbol, status, body);
        return Err(anyhow!("HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse JSON for {}: {}", symbol, e))?;

    // Check for Yahoo API error in response
    if let Some(error) = data.get("chart").and_then(|c| c.get("error")).and_then(|e| e.as_object()) {
        let code = error.get("code").and_then(|c| c.as_str()).unwrap_or("unknown");
        let desc = error.get("description").and_then(|d| d.as_str()).unwrap_or("No description");
        log::error!("Yahoo API returned error for {}: {} - {}", symbol, code, desc);
        return Err(anyhow!("Yahoo API error for {}: {} - {}", symbol, code, desc));
    }

    parse_latest_quote(symbol, &data)
}

/// Historische Kurse abrufen
pub async fn fetch_historical(
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
    adjusted: bool,
) -> Result<Vec<Quote>> {
    // Yahoo verwendet Unix-Timestamps
    let from_ts = from
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);
    let to_ts = to
        .and_hms_opt(23, 59, 59)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);

    let url = format!(
        "{}?period1={}&period2={}&interval=1d&events=history",
        symbol_url(symbol),
        from_ts,
        to_ts
    );
    log::debug!("Fetching Yahoo historical for {} from {}", symbol, url);

    let client = create_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::error!("Yahoo API error for {}: {} - {}", symbol, status, body);
        return Err(anyhow!("HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse JSON for {}: {}", symbol, e))?;

    // Check for Yahoo API error in response
    if let Some(error) = data.get("chart").and_then(|c| c.get("error")).and_then(|e| e.as_object()) {
        let code = error.get("code").and_then(|c| c.as_str()).unwrap_or("unknown");
        let desc = error.get("description").and_then(|d| d.as_str()).unwrap_or("No description");
        log::error!("Yahoo API returned error for {}: {} - {}", symbol, code, desc);
        return Err(anyhow!("Yahoo API error for {}: {} - {}", symbol, code, desc));
    }

    parse_historical_quotes(&data, adjusted)
}

/// Symbol URL erstellen (encoded)
fn symbol_url(symbol: &str) -> String {
    let encoded = urlencoding::encode(symbol);
    format!("{}/{}", BASE_URL, encoded)
}

/// Latest Quote aus Yahoo Response parsen
fn parse_latest_quote(symbol: &str, data: &serde_json::Value) -> Result<LatestQuote> {
    let chart = data
        .get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.get(0))
        .ok_or_else(|| anyhow!("Invalid response format"))?;

    let meta = chart.get("meta").ok_or_else(|| anyhow!("Missing meta"))?;

    let quote_data = chart
        .get("indicators")
        .and_then(|i| i.get("quote"))
        .and_then(|q| q.get(0))
        .ok_or_else(|| anyhow!("Missing quote data"))?;

    // Aktueller Kurs
    let close = meta
        .get("regularMarketPrice")
        .and_then(|p| p.as_f64())
        .or_else(|| {
            quote_data
                .get("close")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.last())
                .and_then(|v| v.as_f64())
        })
        .ok_or_else(|| anyhow!("Missing close price"))?;

    // Timestamp
    let timestamp = meta
        .get("regularMarketTime")
        .and_then(|t| t.as_i64())
        .ok_or_else(|| anyhow!("Missing timestamp"))?;

    let date = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| anyhow!("Invalid timestamp"))?
        .date_naive();

    // Optional: High/Low/Open/Volume aus den letzten Daten
    let high = quote_data
        .get("high")
        .and_then(|h| h.as_array())
        .and_then(|arr| arr.last())
        .and_then(|v| v.as_f64());

    let low = quote_data
        .get("low")
        .and_then(|l| l.as_array())
        .and_then(|arr| arr.last())
        .and_then(|v| v.as_f64());

    let open = quote_data
        .get("open")
        .and_then(|o| o.as_array())
        .and_then(|arr| arr.last())
        .and_then(|v| v.as_f64());

    let volume = quote_data
        .get("volume")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.last())
        .and_then(|v| v.as_i64());

    // Metadaten
    let name = meta.get("shortName").and_then(|n| n.as_str()).map(String::from);
    let currency = meta.get("currency").and_then(|c| c.as_str()).map(String::from);

    Ok(LatestQuote {
        symbol: symbol.to_string(),
        name,
        currency,
        quote: Quote {
            date,
            close,
            high,
            low,
            open,
            volume,
        },
    })
}

/// Historische Kurse aus Yahoo Response parsen
fn parse_historical_quotes(data: &serde_json::Value, adjusted: bool) -> Result<Vec<Quote>> {
    let chart = data
        .get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.get(0))
        .ok_or_else(|| anyhow!("Invalid response format"))?;

    let timestamps = chart
        .get("timestamp")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow!("Missing timestamps"))?;

    let quote_data = chart
        .get("indicators")
        .and_then(|i| i.get("quote"))
        .and_then(|q| q.get(0))
        .ok_or_else(|| anyhow!("Missing quote data"))?;

    // Adjusted Close für Dividenden-Bereinigung
    let adj_close = if adjusted {
        chart
            .get("indicators")
            .and_then(|i| i.get("adjclose"))
            .and_then(|a| a.get(0))
            .and_then(|a| a.get("adjclose"))
            .and_then(|c| c.as_array())
    } else {
        None
    };

    let closes = quote_data
        .get("close")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("Missing close prices"))?;
    let highs = quote_data.get("high").and_then(|h| h.as_array());
    let lows = quote_data.get("low").and_then(|l| l.as_array());
    let opens = quote_data.get("open").and_then(|o| o.as_array());
    let volumes = quote_data.get("volume").and_then(|v| v.as_array());

    let mut quotes = Vec::new();

    for (i, ts) in timestamps.iter().enumerate() {
        let timestamp = ts.as_i64().unwrap_or(0);
        let date = match chrono::DateTime::from_timestamp(timestamp, 0) {
            Some(dt) => dt.date_naive(),
            None => continue,
        };

        // Close-Preis (adjusted oder normal)
        let close = if adjusted {
            adj_close
                .and_then(|arr| arr.get(i))
                .and_then(|v| v.as_f64())
                .or_else(|| closes.get(i).and_then(|v| v.as_f64()))
        } else {
            closes.get(i).and_then(|v| v.as_f64())
        };

        let close = match close {
            Some(c) => c,
            None => continue, // Skip if no close price
        };

        let high = highs.and_then(|arr| arr.get(i)).and_then(|v| v.as_f64());
        let low = lows.and_then(|arr| arr.get(i)).and_then(|v| v.as_f64());
        let open = opens.and_then(|arr| arr.get(i)).and_then(|v| v.as_f64());
        let volume = volumes.and_then(|arr| arr.get(i)).and_then(|v| v.as_i64());

        quotes.push(Quote {
            date,
            close,
            high,
            low,
            open,
            volume,
        });
    }

    Ok(quotes)
}

/// Yahoo Search Result
#[derive(Debug, Clone)]
pub struct YahooSearchResult {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub security_type: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
}

/// Search Response from Yahoo Finance
#[derive(Debug, serde::Deserialize)]
struct SearchResponse {
    quotes: Option<Vec<SearchQuote>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchQuote {
    symbol: String,
    shortname: Option<String>,
    longname: Option<String>,
    exchange: Option<String>,
    #[serde(rename = "quoteType")]
    quote_type: Option<String>,
    #[serde(rename = "typeDisp")]
    type_disp: Option<String>,
    sector: Option<String>,
    industry: Option<String>,
}

/// Search for securities on Yahoo Finance
///
/// # Arguments
/// * `query` - Search keywords (company name, symbol, etc.)
pub async fn search(query: &str) -> Result<Vec<YahooSearchResult>> {
    let client = create_client()?;

    let url = format!(
        "https://query1.finance.yahoo.com/v1/finance/search?q={}&quotesCount=20&newsCount=0",
        urlencoding::encode(query)
    );

    log::debug!("Yahoo search for: {}", query);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Yahoo search request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("Yahoo search error: {}", response.status()));
    }

    let data: SearchResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse Yahoo search response: {}", e))?;

    let results = data.quotes.unwrap_or_default()
        .into_iter()
        .map(|q| YahooSearchResult {
            symbol: q.symbol,
            name: q.longname.or(q.shortname).unwrap_or_default(),
            exchange: q.exchange.unwrap_or_default(),
            security_type: q.type_disp.or(q.quote_type).unwrap_or_else(|| "Equity".to_string()),
            sector: q.sector,
            industry: q.industry,
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search() {
        let results = search("apple").await;
        assert!(results.is_ok(), "Search failed: {:?}", results.err());

        let results = results.unwrap();
        assert!(!results.is_empty(), "No results found");

        // Should find AAPL
        let aapl = results.iter().find(|r| r.symbol == "AAPL");
        assert!(aapl.is_some(), "AAPL not found in results");
        println!("Found {} results, AAPL: {:?}", results.len(), aapl);
    }

    #[tokio::test]
    async fn test_fetch_apple_quote() {
        let result = fetch_quote("AAPL", false).await;
        assert!(result.is_ok(), "Failed to fetch AAPL: {:?}", result.err());

        let quote = result.unwrap();
        assert_eq!(quote.symbol, "AAPL");
        assert!(quote.quote.close > 0.0);
        println!("AAPL: ${:.2} on {}", quote.quote.close, quote.quote.date);
    }

    #[tokio::test]
    async fn test_fetch_historical() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let result = fetch_historical("AAPL", from, to, false).await;
        assert!(result.is_ok(), "Failed to fetch historical: {:?}", result.err());

        let quotes = result.unwrap();
        assert!(!quotes.is_empty());
        println!("Got {} historical quotes for AAPL", quotes.len());
    }
}
