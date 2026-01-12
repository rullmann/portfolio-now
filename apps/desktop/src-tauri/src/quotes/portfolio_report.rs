//! Portfolio Report Quote Provider
//!
//! Fetches fund prices from Portfolio Report API.
//! This is the same provider used by Portfolio Performance.
//! Free to use, no API key required.

use super::{LatestQuote, Quote};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://www.portfolio-report.net/api";

/// Security info response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SecurityResponse {
    uuid: String,
    name: String,
    symbol_xfra: Option<String>,
    symbol_xnas: Option<String>,
    symbol_xnys: Option<String>,
    isin: Option<String>,
    wkn: Option<String>,
    security_type: Option<String>,
    markets: Option<Vec<MarketInfo>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct MarketInfo {
    market_code: String,
    currency_code: String,
    first_price_date: Option<String>,
    last_price_date: Option<String>,
}

/// Price history response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceResponse {
    date: String,
    close: f64,
    #[serde(default)]
    high: Option<f64>,
    #[serde(default)]
    low: Option<f64>,
}

/// Search result
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub uuid: String,
    pub name: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub symbol_xfra: Option<String>,
    pub security_type: Option<String>,
}

/// Fetch current price for a security by UUID
///
/// # Arguments
/// * `uuid` - Portfolio Report security UUID
pub async fn fetch_quote(uuid: &str) -> Result<LatestQuote> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    // First get security info
    let info_url = format!("{}/securities/uuid/{}", BASE_URL, uuid);
    let info_response = client
        .get(&info_url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !info_response.status().is_success() {
        return Err(anyhow!(
            "Portfolio Report API error: {}",
            info_response.status()
        ));
    }

    let security: SecurityResponse = info_response.json().await?;

    // Determine best market (prefer XFRA for German securities)
    let market = security
        .markets
        .as_ref()
        .and_then(|m| {
            m.iter()
                .find(|m| m.market_code == "XFRA")
                .or_else(|| m.first())
        })
        .map(|m| m.market_code.as_str())
        .unwrap_or("XFRA");

    let currency = security
        .markets
        .as_ref()
        .and_then(|m| m.iter().find(|m| m.market_code == market))
        .map(|m| m.currency_code.clone())
        .unwrap_or_else(|| "EUR".to_string());

    // Fetch latest price
    let price_url = format!(
        "{}/securities/uuid/{}/prices/{}?limit=1",
        BASE_URL, uuid, market
    );

    let price_response = client
        .get(&price_url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !price_response.status().is_success() {
        return Err(anyhow!(
            "Portfolio Report price error: {}",
            price_response.status()
        ));
    }

    let prices: Vec<PriceResponse> = price_response.json().await?;

    let latest = prices
        .first()
        .ok_or_else(|| anyhow!("No prices available"))?;

    let date = NaiveDate::parse_from_str(&latest.date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());

    let symbol = security.symbol_xfra
        .or(security.isin.clone())
        .unwrap_or_else(|| uuid.to_string());

    Ok(LatestQuote {
        symbol,
        name: Some(security.name),
        currency: Some(currency),
        quote: Quote {
            date,
            close: latest.close,
            high: latest.high,
            low: latest.low,
            open: None,
            volume: None,
        },
    })
}

/// Fetch price by ISIN
pub async fn fetch_quote_by_isin(isin: &str) -> Result<LatestQuote> {
    // Search for the security first
    let results = search(isin).await?;
    let security = results
        .first()
        .ok_or_else(|| anyhow!("Security with ISIN {} not found", isin))?;

    fetch_quote(&security.uuid).await
}

/// Fetch historical prices
///
/// # Arguments
/// * `uuid` - Portfolio Report security UUID
/// * `market` - Market code (e.g., "XFRA")
/// * `from` - Start date
/// * `to` - End date
pub async fn fetch_historical(
    uuid: &str,
    market: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Quote>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let url = format!(
        "{}/securities/uuid/{}/prices/{}?from={}&to={}",
        BASE_URL,
        uuid,
        market,
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Portfolio Report API error: {}",
            response.status()
        ));
    }

    let prices: Vec<PriceResponse> = response.json().await?;

    let quotes: Vec<Quote> = prices
        .into_iter()
        .filter_map(|p| {
            let date = NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").ok()?;
            Some(Quote {
                date,
                close: p.close,
                high: p.high,
                low: p.low,
                open: None,
                volume: None,
            })
        })
        .collect();

    Ok(quotes)
}

/// Search for securities
///
/// # Arguments
/// * `query` - Search string (ISIN, WKN, name, or symbol)
pub async fn search(query: &str) -> Result<Vec<SearchResult>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let url = format!("{}/securities/search/{}", BASE_URL, query);

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Portfolio Report search error: {}",
            response.status()
        ));
    }

    let results: Vec<SearchResult> = response.json().await?;
    Ok(results)
}

/// Get security info by ISIN
pub async fn get_security_by_isin(isin: &str) -> Result<SearchResult> {
    let results = search(isin).await?;
    results
        .into_iter()
        .find(|s| s.isin.as_deref() == Some(isin))
        .ok_or_else(|| anyhow!("Security with ISIN {} not found", isin))
}

/// Search for a security and return its identifiers (UUID, ISIN, WKN)
/// Tries ticker first, then name if not found.
pub async fn search_and_get_identifiers(
    ticker: Option<&str>,
    name: &str,
) -> Option<(String, Option<String>, Option<String>)> {
    // Try ticker first
    if let Some(ticker) = ticker {
        if let Ok(results) = search(ticker).await {
            if let Some(first) = results.first() {
                return Some((first.uuid.clone(), first.isin.clone(), first.wkn.clone()));
            }
        }
    }

    // Try name search
    if let Ok(results) = search(name).await {
        if let Some(first) = results.first() {
            return Some((first.uuid.clone(), first.isin.clone(), first.wkn.clone()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_search() {
        let results = search("IE00B4L5Y983").await; // iShares Core MSCI World
        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_fetch_by_isin() {
        let result = fetch_quote_by_isin("IE00B4L5Y983").await;
        assert!(result.is_ok());
        let quote = result.unwrap();
        assert!(quote.quote.close > 0.0);
    }
}
