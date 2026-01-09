//! EZB (European Central Bank) Exchange Rate Provider
//!
//! Ruft Wechselkurse von der EZB ab:
//! - Tägliche Kurse (EUR als Basiswährung)
//! - Historische Kurse
//!
//! API: https://data.ecb.europa.eu/

use super::ExchangeRate;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use std::collections::HashMap;

/// EZB API URL für tägliche Kurse (XML)
const ECB_DAILY_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";

/// EZB API URL für historische Kurse (letzte 90 Tage)
const ECB_HIST_90_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml";

/// EZB API URL für alle historischen Kurse
const ECB_HIST_ALL_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.xml";

/// Aktuelle Wechselkurse abrufen (EUR-Basis)
pub async fn fetch_latest_rates() -> Result<Vec<ExchangeRate>> {
    let response = reqwest::get(ECB_DAILY_URL)
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }

    let xml = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response: {}", e))?;

    parse_ecb_xml(&xml)
}

/// Historische Wechselkurse abrufen
pub async fn fetch_historical_rates(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<HashMap<NaiveDate, Vec<ExchangeRate>>> {
    // Wähle URL basierend auf Zeitraum
    let today = chrono::Utc::now().date_naive();
    let days_back = (today - from).num_days();

    let url = if days_back <= 90 {
        ECB_HIST_90_URL
    } else {
        ECB_HIST_ALL_URL
    };

    let response = reqwest::get(url)
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }

    let xml = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response: {}", e))?;

    parse_ecb_historical_xml(&xml, from, to)
}

/// Wechselkurs für ein bestimmtes Währungspaar abrufen
pub async fn fetch_rate(base: &str, target: &str) -> Result<ExchangeRate> {
    let rates = fetch_latest_rates().await?;

    // EUR als Basiswährung
    if base == "EUR" {
        rates
            .into_iter()
            .find(|r| r.target == target)
            .ok_or_else(|| anyhow!("Rate not found for EUR/{}", target))
    } else if target == "EUR" {
        // Invertieren
        rates
            .into_iter()
            .find(|r| r.target == base)
            .map(|r| ExchangeRate {
                base: base.to_string(),
                target: "EUR".to_string(),
                date: r.date,
                rate: 1.0 / r.rate,
            })
            .ok_or_else(|| anyhow!("Rate not found for {}/EUR", base))
    } else {
        // Cross-Rate über EUR
        let base_rate = rates
            .iter()
            .find(|r| r.target == base)
            .ok_or_else(|| anyhow!("Rate not found for EUR/{}", base))?;
        let target_rate = rates
            .iter()
            .find(|r| r.target == target)
            .ok_or_else(|| anyhow!("Rate not found for EUR/{}", target))?;

        Ok(ExchangeRate {
            base: base.to_string(),
            target: target.to_string(),
            date: base_rate.date,
            rate: target_rate.rate / base_rate.rate,
        })
    }
}

/// EZB XML parsen (tägliche Kurse)
fn parse_ecb_xml(xml: &str) -> Result<Vec<ExchangeRate>> {
    let mut rates = Vec::new();
    let mut current_date: Option<NaiveDate> = None;

    // Einfaches XML-Parsing (ohne externe Lib)
    for line in xml.lines() {
        let line = line.trim();

        // Datum extrahieren: <Cube time='2024-01-15'>
        if line.contains("time=") && !line.contains("currency=") {
            if let Some(date_str) = extract_attribute(line, "time") {
                current_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
            }
        }

        // Kurs extrahieren: <Cube currency='USD' rate='1.0876'/>
        if line.contains("currency=") && line.contains("rate=") {
            if let (Some(currency), Some(rate_str)) =
                (extract_attribute(line, "currency"), extract_attribute(line, "rate"))
            {
                if let (Some(date), Ok(rate)) = (current_date, rate_str.parse::<f64>()) {
                    rates.push(ExchangeRate {
                        base: "EUR".to_string(),
                        target: currency,
                        date,
                        rate,
                    });
                }
            }
        }
    }

    if rates.is_empty() {
        return Err(anyhow!("No rates found in ECB response"));
    }

    Ok(rates)
}

/// Historische EZB-Kurse parsen
fn parse_ecb_historical_xml(
    xml: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<HashMap<NaiveDate, Vec<ExchangeRate>>> {
    let mut rates_by_date: HashMap<NaiveDate, Vec<ExchangeRate>> = HashMap::new();
    let mut current_date: Option<NaiveDate> = None;

    for line in xml.lines() {
        let line = line.trim();

        // Datum extrahieren
        if line.contains("time=") && !line.contains("currency=") {
            if let Some(date_str) = extract_attribute(line, "time") {
                current_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
            }
        }

        // Kurs extrahieren
        if line.contains("currency=") && line.contains("rate=") {
            if let (Some(currency), Some(rate_str)) =
                (extract_attribute(line, "currency"), extract_attribute(line, "rate"))
            {
                if let Some(date) = current_date {
                    // Nur im gewünschten Zeitraum
                    if date >= from && date <= to {
                        if let Ok(rate) = rate_str.parse::<f64>() {
                            rates_by_date.entry(date).or_default().push(ExchangeRate {
                                base: "EUR".to_string(),
                                target: currency,
                                date,
                                rate,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(rates_by_date)
}

/// Attributwert aus XML-Tag extrahieren
fn extract_attribute(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=", attr);
    let start = line.find(&pattern)?;
    let rest = &line[start + pattern.len()..];

    // Finde den Wert zwischen Anführungszeichen
    let quote_char = rest.chars().next()?;
    if quote_char != '\'' && quote_char != '"' {
        return None;
    }

    let value_start = 1;
    let value_end = rest[value_start..].find(quote_char)? + value_start;
    Some(rest[value_start..value_end].to_string())
}

/// Unterstützte Währungen der EZB
pub const SUPPORTED_CURRENCIES: &[&str] = &[
    "USD", "JPY", "BGN", "CZK", "DKK", "GBP", "HUF", "PLN", "RON", "SEK", "CHF", "ISK", "NOK",
    "TRY", "AUD", "BRL", "CAD", "CNY", "HKD", "IDR", "ILS", "INR", "KRW", "MXN", "MYR", "NZD",
    "PHP", "SGD", "THB", "ZAR",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_latest_rates() {
        let result = fetch_latest_rates().await;
        assert!(result.is_ok(), "Failed to fetch rates: {:?}", result.err());

        let rates = result.unwrap();
        assert!(!rates.is_empty());

        // USD sollte dabei sein
        let usd = rates.iter().find(|r| r.target == "USD");
        assert!(usd.is_some(), "USD rate not found");

        println!(
            "EUR/USD: {:.4} on {}",
            usd.unwrap().rate,
            usd.unwrap().date
        );
    }

    #[tokio::test]
    async fn test_fetch_cross_rate() {
        let result = fetch_rate("USD", "CHF").await;
        assert!(result.is_ok(), "Failed to fetch cross rate: {:?}", result.err());

        let rate = result.unwrap();
        println!("USD/CHF: {:.4}", rate.rate);
    }
}
