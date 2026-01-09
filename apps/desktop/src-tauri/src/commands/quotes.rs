use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResult {
    pub symbol: String,
    pub date: NaiveDate,
    pub close: f64,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<i64>,
}

#[command]
pub async fn fetch_quotes(symbols: Vec<String>) -> Result<Vec<QuoteResult>, String> {
    let mut results = Vec::new();

    for symbol in symbols {
        match fetch_yahoo_quote(&symbol).await {
            Ok(quote) => results.push(quote),
            Err(e) => {
                log::warn!("Failed to fetch quote for {}: {}", symbol, e);
            }
        }
    }

    Ok(results)
}

async fn fetch_yahoo_quote(symbol: &str) -> Result<QuoteResult, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
        symbol
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let chart = data
        .get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.get(0))
        .ok_or("Invalid response format")?;

    let meta = chart.get("meta").ok_or("Missing meta")?;
    let quote = chart
        .get("indicators")
        .and_then(|i| i.get("quote"))
        .and_then(|q| q.get(0))
        .ok_or("Missing quote data")?;

    let close = quote
        .get("close")
        .and_then(|c| c.get(0))
        .and_then(|c| c.as_f64())
        .ok_or("Missing close price")?;

    let high = quote
        .get("high")
        .and_then(|h| h.get(0))
        .and_then(|h| h.as_f64());

    let low = quote
        .get("low")
        .and_then(|l| l.get(0))
        .and_then(|l| l.as_f64());

    let volume = quote
        .get("volume")
        .and_then(|v| v.get(0))
        .and_then(|v| v.as_i64());

    let timestamp = meta
        .get("regularMarketTime")
        .and_then(|t| t.as_i64())
        .ok_or("Missing timestamp")?;

    let date = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or("Invalid timestamp")?
        .date_naive();

    Ok(QuoteResult {
        symbol: symbol.to_string(),
        date,
        close,
        high,
        low,
        volume,
    })
}
