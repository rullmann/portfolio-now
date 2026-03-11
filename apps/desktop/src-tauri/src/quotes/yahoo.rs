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
use serde::{Deserialize, Serialize};

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
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// HTTP Client with cookie store for Yahoo crumb auth (quoteSummary API)
fn create_cookie_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to create cookie client: {}", e))
}

/// Get Yahoo crumb+cookie for authenticated API calls (quoteSummary)
async fn get_yahoo_crumb(client: &reqwest::Client) -> Result<String> {
    // Step 1: Get consent cookie
    let _ = client.get("https://fc.yahoo.com")
        .send().await;

    // Step 2: Get crumb
    let crumb_response = client.get("https://query2.finance.yahoo.com/v1/test/getcrumb")
        .send().await
        .map_err(|e| anyhow!("Failed to get Yahoo crumb: {}", e))?;

    let crumb = crumb_response.text().await
        .map_err(|e| anyhow!("Failed to read crumb response: {}", e))?;

    if crumb.is_empty() || crumb.len() > 50 {
        return Err(anyhow!("Invalid Yahoo crumb received"));
    }

    Ok(crumb)
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
    let result = fetch_historical_with_splits(symbol, from, to, adjusted).await?;
    Ok(result.quotes)
}

/// Historische Kurse MIT Split-Events abrufen
///
/// Verwendet Yahoo's `events=history,splits` Parameter um sowohl Kurse
/// als auch Stock-Split Events zu erhalten.
pub async fn fetch_historical_with_splits(
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
    adjusted: bool,
) -> Result<super::HistoricalDataWithSplits> {
    // Yahoo verwendet Unix-Timestamps
    let from_ts = from
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);
    let to_ts = to
        .and_hms_opt(23, 59, 59)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);

    // Wichtig: events=history,splits um Split-Events zu erhalten
    let url = format!(
        "{}?period1={}&period2={}&interval=1d&events=history,splits",
        symbol_url(symbol),
        from_ts,
        to_ts
    );
    log::debug!("Fetching Yahoo historical with splits for {} from {}", symbol, url);

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

    // Parse quotes
    let quotes = parse_historical_quotes(&data, adjusted)?;

    // Parse split events
    let splits = parse_split_events(&data);

    if !splits.is_empty() {
        log::info!("Found {} split events for {}", splits.len(), symbol);
        for split in &splits {
            log::info!("  Split on {}: {}", split.date, split.ratio_str());
        }
    }

    Ok(super::HistoricalDataWithSplits { quotes, splits })
}

/// Split-Events aus Yahoo Response parsen
///
/// Yahoo liefert Splits im Format:
/// ```json
/// "events": {
///   "splits": {
///     "1598880600": {
///       "date": 1598880600,
///       "numerator": 4.0,
///       "denominator": 1.0,
///       "splitRatio": "4:1"
///     }
///   }
/// }
/// ```
fn parse_split_events(data: &serde_json::Value) -> Vec<super::SplitEvent> {
    let mut splits = Vec::new();

    let events = match data
        .get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.get(0))
        .and_then(|r| r.get("events"))
        .and_then(|e| e.get("splits"))
        .and_then(|s| s.as_object())
    {
        Some(e) => e,
        None => return splits,
    };

    for (_timestamp_key, split_data) in events {
        // Parse timestamp
        let timestamp = match split_data.get("date").and_then(|d| d.as_i64()) {
            Some(ts) => ts,
            None => continue,
        };

        let date = match chrono::DateTime::from_timestamp(timestamp, 0) {
            Some(dt) => dt.date_naive(),
            None => continue,
        };

        // Parse numerator and denominator
        let numerator = split_data
            .get("numerator")
            .and_then(|n| n.as_f64())
            .unwrap_or(1.0);
        let denominator = split_data
            .get("denominator")
            .and_then(|d| d.as_f64())
            .unwrap_or(1.0);

        // Skip invalid splits
        if denominator == 0.0 || numerator == 0.0 {
            continue;
        }

        splits.push(super::SplitEvent {
            date,
            numerator,
            denominator,
        });
    }

    // Sort by date
    splits.sort_by_key(|s| s.date);
    splits
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

/// Company profile data from Yahoo quoteSummary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YahooProfile {
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub country: Option<String>,
    pub market_cap: Option<f64>,
}

/// Fetch company profile from Yahoo quoteSummary endpoint
pub async fn fetch_profile(symbol: &str) -> Result<YahooProfile> {
    let encoded = urlencoding::encode(symbol);
    let url = format!(
        "https://query1.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=assetProfile,summaryDetail",
        encoded
    );
    log::debug!("Fetching Yahoo profile for {} from {}", symbol, url);

    let client = create_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Profile request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Yahoo profile HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse profile JSON for {}: {}", symbol, e))?;

    // Navigate: quoteSummary.result[0]
    let result = data
        .get("quoteSummary")
        .and_then(|q| q.get("result"))
        .and_then(|r| r.get(0));

    let result = match result {
        Some(r) => r,
        None => return Err(anyhow!("No profile data for {}", symbol)),
    };

    let asset_profile = result.get("assetProfile");
    let summary_detail = result.get("summaryDetail");

    let sector = asset_profile
        .and_then(|p| p.get("sector"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let industry = asset_profile
        .and_then(|p| p.get("industry"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let description = asset_profile
        .and_then(|p| p.get("longBusinessSummary"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let country = asset_profile
        .and_then(|p| p.get("country"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let market_cap = summary_detail
        .and_then(|d| d.get("marketCap"))
        .and_then(|m| m.get("raw"))
        .and_then(|v| v.as_f64());

    Ok(YahooProfile {
        sector,
        industry,
        description,
        country,
        market_cap,
    })
}

// ============================================================================
// Fundamental Data
// ============================================================================

/// Fundamental financial data from Yahoo Finance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YahooFundamentals {
    // Valuation
    pub trailing_pe: Option<f64>,
    pub forward_pe: Option<f64>,
    pub price_to_book: Option<f64>,
    pub price_to_sales: Option<f64>,
    pub enterprise_to_ebitda: Option<f64>,
    pub peg_ratio: Option<f64>,
    // Profitability
    pub return_on_equity: Option<f64>,
    pub return_on_assets: Option<f64>,
    pub profit_margin: Option<f64>,
    pub operating_margin: Option<f64>,
    pub gross_margin: Option<f64>,
    // Financial Health
    pub debt_to_equity: Option<f64>,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    // Cash Flow
    pub free_cash_flow: Option<f64>,
    pub operating_cash_flow: Option<f64>,
    // Growth
    pub revenue_growth: Option<f64>,
    pub earnings_growth: Option<f64>,
    // Size
    pub market_cap: Option<f64>,
    pub enterprise_value: Option<f64>,
    pub revenue: Option<f64>,
    pub ebitda: Option<f64>,
    // Dividend
    pub dividend_yield: Option<f64>,
    pub dividend_rate: Option<f64>,
    pub payout_ratio: Option<f64>,
    // Per Share
    pub book_value: Option<f64>,
    pub earnings_per_share: Option<f64>,
    pub revenue_per_share: Option<f64>,
    // Shares
    pub shares_outstanding: Option<f64>,
    pub float_shares: Option<f64>,
    // Target
    pub target_mean_price: Option<f64>,
    pub target_high_price: Option<f64>,
    pub target_low_price: Option<f64>,
    pub recommendation_mean: Option<f64>,
    pub recommendation_key: Option<String>,
    pub number_of_analyst_opinions: Option<i64>,
}

fn extract_raw_f64(obj: Option<&serde_json::Value>, field: &str) -> Option<f64> {
    obj.and_then(|o| o.get(field))
        .and_then(|v| v.get("raw").and_then(|r| r.as_f64()).or_else(|| v.as_f64()))
}

fn extract_raw_i64(obj: Option<&serde_json::Value>, field: &str) -> Option<i64> {
    obj.and_then(|o| o.get(field))
        .and_then(|v| v.get("raw").and_then(|r| r.as_i64()).or_else(|| v.as_i64()))
}

fn extract_raw_str(obj: Option<&serde_json::Value>, field: &str) -> Option<String> {
    obj.and_then(|o| o.get(field))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Fetch fundamental financial data from Yahoo Finance quoteSummary
pub async fn fetch_fundamentals(symbol: &str) -> Result<YahooFundamentals> {
    let encoded = urlencoding::encode(symbol);

    // Yahoo quoteSummary now requires crumb+cookie authentication
    let client = create_cookie_client()?;
    let crumb = get_yahoo_crumb(&client).await
        .map_err(|e| anyhow!("Failed to get Yahoo auth for fundamentals: {}", e))?;

    let url = format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=defaultKeyStatistics,financialData,summaryDetail&crumb={}",
        encoded, urlencoding::encode(&crumb)
    );
    log::debug!("Fetching Yahoo fundamentals for {} (with crumb auth)", symbol);

    let response = client.get(&url).send().await
        .map_err(|e| anyhow!("Fundamentals request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Yahoo fundamentals HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: serde_json::Value = response.json().await
        .map_err(|e| anyhow!("Failed to parse fundamentals JSON for {}: {}", symbol, e))?;

    let result = data.get("quoteSummary")
        .and_then(|q| q.get("result"))
        .and_then(|r| r.get(0));

    let result = match result {
        Some(r) => r,
        None => return Err(anyhow!("No fundamentals data for {}", symbol)),
    };

    let key_stats = result.get("defaultKeyStatistics");
    let fin_data = result.get("financialData");
    let summary = result.get("summaryDetail");

    Ok(YahooFundamentals {
        trailing_pe: extract_raw_f64(summary, "trailingPE"),
        forward_pe: extract_raw_f64(key_stats, "forwardPE"),
        price_to_book: extract_raw_f64(key_stats, "priceToBook"),
        price_to_sales: extract_raw_f64(key_stats, "priceToSalesTrailing12Months")
            .or_else(|| extract_raw_f64(summary, "priceToSalesTrailing12Months")),
        enterprise_to_ebitda: extract_raw_f64(key_stats, "enterpriseToEbitda"),
        peg_ratio: extract_raw_f64(key_stats, "pegRatio"),
        return_on_equity: extract_raw_f64(fin_data, "returnOnEquity"),
        return_on_assets: extract_raw_f64(fin_data, "returnOnAssets"),
        profit_margin: extract_raw_f64(fin_data, "profitMargins")
            .or_else(|| extract_raw_f64(key_stats, "profitMargins")),
        operating_margin: extract_raw_f64(fin_data, "operatingMargins"),
        gross_margin: extract_raw_f64(fin_data, "grossMargins"),
        debt_to_equity: extract_raw_f64(fin_data, "debtToEquity"),
        current_ratio: extract_raw_f64(fin_data, "currentRatio"),
        quick_ratio: extract_raw_f64(fin_data, "quickRatio"),
        free_cash_flow: extract_raw_f64(fin_data, "freeCashflow"),
        operating_cash_flow: extract_raw_f64(fin_data, "operatingCashflow"),
        revenue_growth: extract_raw_f64(fin_data, "revenueGrowth"),
        earnings_growth: extract_raw_f64(fin_data, "earningsGrowth"),
        market_cap: extract_raw_f64(summary, "marketCap"),
        enterprise_value: extract_raw_f64(key_stats, "enterpriseValue"),
        revenue: extract_raw_f64(fin_data, "totalRevenue"),
        ebitda: extract_raw_f64(fin_data, "ebitda"),
        dividend_yield: extract_raw_f64(summary, "dividendYield"),
        dividend_rate: extract_raw_f64(summary, "dividendRate"),
        payout_ratio: extract_raw_f64(summary, "payoutRatio"),
        book_value: extract_raw_f64(key_stats, "bookValue"),
        earnings_per_share: extract_raw_f64(key_stats, "trailingEps"),
        revenue_per_share: extract_raw_f64(fin_data, "revenuePerShare"),
        shares_outstanding: extract_raw_f64(key_stats, "sharesOutstanding"),
        float_shares: extract_raw_f64(key_stats, "floatShares"),
        target_mean_price: extract_raw_f64(fin_data, "targetMeanPrice"),
        target_high_price: extract_raw_f64(fin_data, "targetHighPrice"),
        target_low_price: extract_raw_f64(fin_data, "targetLowPrice"),
        recommendation_mean: extract_raw_f64(fin_data, "recommendationMean"),
        recommendation_key: extract_raw_str(fin_data, "recommendationKey"),
        number_of_analyst_opinions: extract_raw_i64(fin_data, "numberOfAnalystOpinions"),
    })
}

// ============================================================================
// Earnings Data
// ============================================================================

/// Quarterly earnings data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsQuarter {
    pub quarter: String,       // e.g. "2024Q4"
    pub date: Option<String>,  // earnings date
    pub eps_actual: Option<f64>,
    pub eps_estimate: Option<f64>,
    pub eps_surprise: Option<f64>,      // actual - estimate
    pub eps_surprise_pct: Option<f64>,  // surprise as percentage
    pub revenue_actual: Option<f64>,
    pub revenue_estimate: Option<f64>,
}

/// Earnings data from Yahoo Finance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YahooEarnings {
    /// Historical quarterly earnings
    pub quarterly_earnings: Vec<EarningsQuarter>,
    /// Next earnings date (if available)
    pub next_earnings_date: Option<String>,
    /// Earnings trend - current year EPS estimate
    pub current_year_estimate: Option<f64>,
    /// Next year EPS estimate
    pub next_year_estimate: Option<f64>,
    /// Current quarter EPS estimate
    pub current_quarter_estimate: Option<f64>,
    /// EPS growth estimate (year over year)
    pub eps_growth_estimate: Option<f64>,
    /// Revenue growth estimate
    pub revenue_growth_estimate: Option<f64>,
}

/// Fetch earnings data from Yahoo Finance quoteSummary
pub async fn fetch_earnings(symbol: &str) -> Result<YahooEarnings> {
    let encoded = urlencoding::encode(symbol);

    // Yahoo quoteSummary now requires crumb+cookie authentication
    let client = create_cookie_client()?;
    let crumb = get_yahoo_crumb(&client).await
        .map_err(|e| anyhow!("Failed to get Yahoo auth for earnings: {}", e))?;

    let url = format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=earningsHistory,earningsTrend,calendarEvents&crumb={}",
        encoded, urlencoding::encode(&crumb)
    );
    log::debug!("Fetching Yahoo earnings for {} (with crumb auth)", symbol);

    let response = client.get(&url).send().await
        .map_err(|e| anyhow!("Earnings request failed for {}: {}", symbol, e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Yahoo earnings HTTP error for {}: {} - {}", symbol, status, body));
    }

    let data: serde_json::Value = response.json().await
        .map_err(|e| anyhow!("Failed to parse earnings JSON for {}: {}", symbol, e))?;

    let result = data.get("quoteSummary")
        .and_then(|q| q.get("result"))
        .and_then(|r| r.get(0));

    let result = match result {
        Some(r) => r,
        None => return Err(anyhow!("No earnings data for {}", symbol)),
    };

    // Parse quarterly earnings history
    let mut quarterly_earnings = Vec::new();
    if let Some(history) = result.get("earningsHistory")
        .and_then(|h| h.get("history"))
        .and_then(|h| h.as_array())
    {
        for q in history {
            let quarter = q.get("quarter")
                .and_then(|v| v.get("fmt").and_then(|f| f.as_str()))
                .unwrap_or("")
                .to_string();
            let eps_actual = extract_raw_f64(Some(q), "epsActual");
            let eps_estimate = extract_raw_f64(Some(q), "epsEstimate");
            let eps_surprise = extract_raw_f64(Some(q), "epsDifference");
            let eps_surprise_pct = extract_raw_f64(Some(q), "surprisePercent");

            if !quarter.is_empty() || eps_actual.is_some() {
                // Use period.fmt as date, fallback to quarter (which is often YYYY-MM-DD)
                let date = q.get("period")
                    .and_then(|v| v.get("fmt").and_then(|f| f.as_str()))
                    .map(|s| s.to_string())
                    .or_else(|| if quarter.len() >= 10 { Some(quarter[..10].to_string()) } else { None });
                quarterly_earnings.push(EarningsQuarter {
                    quarter,
                    date,
                    eps_actual,
                    eps_estimate,
                    eps_surprise,
                    eps_surprise_pct,
                    revenue_actual: None,
                    revenue_estimate: None,
                });
            }
        }
    }

    // Parse earnings trend for estimates
    let mut current_year_estimate = None;
    let mut next_year_estimate = None;
    let mut current_quarter_estimate = None;
    let mut eps_growth_estimate = None;
    let mut revenue_growth_estimate = None;

    if let Some(trend) = result.get("earningsTrend")
        .and_then(|t| t.get("trend"))
        .and_then(|t| t.as_array())
    {
        for t in trend {
            let period = t.get("period").and_then(|p| p.as_str()).unwrap_or("");
            let eps_est = t.get("earningsEstimate")
                .and_then(|e| extract_raw_f64(Some(e), "avg"));

            match period {
                "0y" => {
                    current_year_estimate = eps_est;
                    eps_growth_estimate = t.get("earningsEstimate")
                        .and_then(|e| extract_raw_f64(Some(e), "growth"));
                    revenue_growth_estimate = t.get("revenueEstimate")
                        .and_then(|e| extract_raw_f64(Some(e), "growth"));
                }
                "+1y" => next_year_estimate = eps_est,
                "0q" => current_quarter_estimate = eps_est,
                _ => {}
            }
        }
    }

    // Next earnings date from calendar
    let next_earnings_date = result.get("calendarEvents")
        .and_then(|c| c.get("earnings"))
        .and_then(|e| e.get("earningsDate"))
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("fmt").and_then(|f| f.as_str()))
        .map(|s| s.to_string());

    Ok(YahooEarnings {
        quarterly_earnings,
        next_earnings_date,
        current_year_estimate,
        next_year_estimate,
        current_quarter_estimate,
        eps_growth_estimate,
        revenue_growth_estimate,
    })
}

// ============================================================================
// Chart Events: Dividends + Earnings from Yahoo
// ============================================================================

/// A dividend event from Yahoo's chart API
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YahooDividendEvent {
    pub date: String,      // YYYY-MM-DD
    pub amount: f64,       // Dividend amount per share
    pub currency: String,  // From the quote metadata
}

/// Fetch dividend dates from Yahoo v8 chart API (events=div)
pub async fn fetch_dividend_events(symbol: &str) -> Result<Vec<YahooDividendEvent>> {
    let from_ts = NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);
    let to_ts = chrono::Utc::now().timestamp();

    let url = format!(
        "{}?period1={}&period2={}&interval=1d&events=div",
        symbol_url(symbol), from_ts, to_ts
    );
    log::debug!("Fetching Yahoo dividend events for {} from {}", symbol, url);

    let client = create_client()?;
    let response = client.get(&url).send().await
        .map_err(|e| anyhow!("Dividend events request failed for {}: {}", symbol, e))?;

    if !response.status().is_success() {
        return Err(anyhow!("Yahoo div events HTTP error for {}: {}", symbol, response.status()));
    }

    let data: serde_json::Value = response.json().await
        .map_err(|e| anyhow!("Failed to parse div events JSON for {}: {}", symbol, e))?;

    let result = data.get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.get(0));

    let result = match result {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    // Get currency from metadata
    let currency = result.get("meta")
        .and_then(|m| m.get("currency"))
        .and_then(|c| c.as_str())
        .unwrap_or("USD")
        .to_string();

    let mut events = Vec::new();

    if let Some(dividends) = result.get("events")
        .and_then(|e| e.get("dividends"))
        .and_then(|d| d.as_object())
    {
        for (_ts, div) in dividends {
            let timestamp = div.get("date").and_then(|d| d.as_i64()).unwrap_or(0);
            let amount = div.get("amount").and_then(|a| a.as_f64()).unwrap_or(0.0);
            if timestamp > 0 && amount > 0.0 {
                let date = chrono::DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                if !date.is_empty() {
                    events.push(YahooDividendEvent { date, amount, currency: currency.clone() });
                }
            }
        }
    }

    events.sort_by(|a, b| a.date.cmp(&b.date));
    log::info!("Found {} dividend events for {}", events.len(), symbol);
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

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

    #[tokio::test]
    async fn test_fetch_historical_with_splits_apple() {
        // Apple hatte einen 4:1 Split am 2020-08-31
        let from = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2020, 12, 31).unwrap();

        let result = fetch_historical_with_splits("AAPL", from, to, false).await;
        assert!(result.is_ok(), "Failed to fetch with splits: {:?}", result.err());

        let data = result.unwrap();
        assert!(!data.quotes.is_empty(), "No quotes returned");

        // Should find the 4:1 split
        println!("Found {} splits for AAPL in 2020:", data.splits.len());
        for split in &data.splits {
            println!("  {} - {} (multiplier: {})", split.date, split.ratio_str(), split.multiplier());
        }

        // Apple 4:1 split on Aug 31, 2020
        let aug_split = data.splits.iter().find(|s| s.date.month() == 8 && s.date.year() == 2020);
        assert!(aug_split.is_some(), "Apple 4:1 split not found");

        let split = aug_split.unwrap();
        assert!((split.multiplier() - 4.0).abs() < 0.01, "Expected 4:1 split, got {}", split.ratio_str());
    }

    #[tokio::test]
    async fn test_fetch_historical_with_splits_amazon() {
        // Amazon hatte einen 20:1 Split am 2022-06-06
        let from = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2022, 12, 31).unwrap();

        let result = fetch_historical_with_splits("AMZN", from, to, false).await;
        assert!(result.is_ok(), "Failed to fetch with splits: {:?}", result.err());

        let data = result.unwrap();

        println!("Found {} splits for AMZN in 2022:", data.splits.len());
        for split in &data.splits {
            println!("  {} - {} (multiplier: {})", split.date, split.ratio_str(), split.multiplier());
        }

        // Amazon 20:1 split on June 6, 2022
        let june_split = data.splits.iter().find(|s| s.date.month() == 6 && s.date.year() == 2022);
        assert!(june_split.is_some(), "Amazon 20:1 split not found");

        let split = june_split.unwrap();
        assert!((split.multiplier() - 20.0).abs() < 0.01, "Expected 20:1 split, got {}", split.ratio_str());
    }
}
