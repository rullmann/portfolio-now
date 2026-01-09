//! Quote Provider Framework
//!
//! Erweiterbare Architektur für verschiedene Kursquellen:
//! - Yahoo Finance (Aktien, ETFs, Fonds)
//! - EZB (Wechselkurse)
//! - CoinGecko (Kryptowährungen)
//! - Kraken (Kryptowährungen - Börsenpreise)
//! - Alpha Vantage (Aktien, ETFs - API-Key erforderlich)
//! - Twelve Data (Schweizer Aktien, internationale Märkte - API-Key erforderlich)
//! - Portfolio Report (Deutsche Fonds, ETFs)

pub mod alphavantage;
pub mod coingecko;
pub mod ecb;
pub mod finnhub;
pub mod kraken;
pub mod portfolio_report;
pub mod twelvedata;
pub mod yahoo;

use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Einzelner Kursdatenpunkt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub date: NaiveDate,
    pub close: f64,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub open: Option<f64>,
    pub volume: Option<i64>,
}

/// Aktueller Kurs mit Metadaten
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestQuote {
    pub symbol: String,
    pub name: Option<String>,
    pub currency: Option<String>,
    pub quote: Quote,
}

/// Wechselkurs zwischen zwei Währungen
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRate {
    pub base: String,
    pub target: String,
    pub date: NaiveDate,
    pub rate: f64,
}

/// Provider-Typ für die Konfiguration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderType {
    Yahoo,
    YahooAdjustedClose,
    Ecb,
    AlphaVantage,
    TwelveData,
    PortfolioReport,
    CoinGecko,
    Kraken,
    Finnhub,
    Manual,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "YAHOO" => Some(Self::Yahoo),
            "YAHOO-ADJUSTEDCLOSE" => Some(Self::YahooAdjustedClose),
            "ECB" => Some(Self::Ecb),
            "ALPHAVANTAGE" => Some(Self::AlphaVantage),
            "TWELVEDATA" | "TWELVE_DATA" => Some(Self::TwelveData),
            "PP" | "PORTFOLIO_REPORT" => Some(Self::PortfolioReport),
            "COINGECKO" => Some(Self::CoinGecko),
            "KRAKEN" => Some(Self::Kraken),
            "FINNHUB" => Some(Self::Finnhub),
            "MANUAL" => Some(Self::Manual),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Yahoo => "YAHOO",
            Self::YahooAdjustedClose => "YAHOO-ADJUSTEDCLOSE",
            Self::Ecb => "ECB",
            Self::AlphaVantage => "ALPHAVANTAGE",
            Self::TwelveData => "TWELVEDATA",
            Self::PortfolioReport => "PP",
            Self::CoinGecko => "COINGECKO",
            Self::Kraken => "KRAKEN",
            Self::Finnhub => "FINNHUB",
            Self::Manual => "MANUAL",
        }
    }
}

/// Ergebnis einer Kursabfrage für eine Security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResult {
    pub security_id: i64,
    pub symbol: String,
    pub provider: String,
    pub success: bool,
    pub error: Option<String>,
    pub latest: Option<LatestQuote>,
    pub historical: Vec<Quote>,
}

impl QuoteResult {
    pub fn success(security_id: i64, symbol: String, provider: &str, latest: LatestQuote) -> Self {
        Self {
            security_id,
            symbol,
            provider: provider.to_string(),
            success: true,
            error: None,
            latest: Some(latest),
            historical: vec![],
        }
    }

    pub fn with_history(mut self, quotes: Vec<Quote>) -> Self {
        self.historical = quotes;
        self
    }

    pub fn error(security_id: i64, symbol: String, provider: &str, error: String) -> Self {
        Self {
            security_id,
            symbol,
            provider: provider.to_string(),
            success: false,
            error: Some(error),
            latest: None,
            historical: vec![],
        }
    }
}

/// Kurs in Datenbank-Format konvertieren (value × 10^8)
pub fn price_to_db(price: f64) -> i64 {
    (price * 100_000_000.0).round() as i64
}

/// Kurs aus Datenbank-Format konvertieren
pub fn price_from_db(value: i64) -> f64 {
    value as f64 / 100_000_000.0
}

/// Alle Kurse für eine Liste von Securities abrufen
pub async fn fetch_all_quotes(
    securities: Vec<SecurityQuoteRequest>,
) -> Vec<QuoteResult> {
    let mut results = Vec::new();

    for sec in securities {
        let result = fetch_quote_for_security(&sec).await;
        results.push(result);
    }

    results
}

/// Security-Anfrage für Kursabfrage
#[derive(Debug, Clone)]
pub struct SecurityQuoteRequest {
    pub id: i64,
    pub symbol: String,
    pub provider: ProviderType,
    pub feed_url: Option<String>,
    /// Optional API key for providers that require authentication
    pub api_key: Option<String>,
}

/// Kurs für eine einzelne Security abrufen
async fn fetch_quote_for_security(sec: &SecurityQuoteRequest) -> QuoteResult {
    match sec.provider {
        ProviderType::Yahoo | ProviderType::YahooAdjustedClose => {
            let adjusted = sec.provider == ProviderType::YahooAdjustedClose;
            // Append exchange suffix from feed_url if present (e.g., ".DE", ".L")
            let symbol = if let Some(ref suffix) = sec.feed_url {
                if suffix.starts_with('.') && !sec.symbol.contains('.') {
                    format!("{}{}", sec.symbol, suffix)
                } else {
                    sec.symbol.clone()
                }
            } else {
                sec.symbol.clone()
            };
            match yahoo::fetch_quote(&symbol, adjusted).await {
                Ok(quote) => QuoteResult::success(sec.id, symbol, sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, symbol, sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::CoinGecko => {
            // Try to convert symbol to CoinGecko ID
            let coin_id = coingecko::symbol_to_coin_id(&sec.symbol)
                .map(|s| s.to_string())
                .unwrap_or_else(|| sec.symbol.to_lowercase());

            match coingecko::fetch_quote(&coin_id, "EUR", sec.api_key.as_deref()).await {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::Kraken => {
            // Use feed_url as target currency or default to EUR
            let currency = sec.feed_url.as_deref().unwrap_or("EUR");

            match kraken::fetch_quote(&sec.symbol, currency).await {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::AlphaVantage => {
            // API key from sec.api_key or feed_url or environment
            let api_key = sec.api_key.clone()
                .or_else(|| sec.feed_url.clone())
                .or_else(|| std::env::var("ALPHAVANTAGE_API_KEY").ok())
                .unwrap_or_default();

            if api_key.is_empty() {
                return QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(),
                    "Alpha Vantage API key required".to_string());
            }

            match alphavantage::fetch_quote(&sec.symbol, &api_key).await {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::TwelveData => {
            // API key from sec.api_key or environment
            let api_key = sec.api_key.clone()
                .or_else(|| std::env::var("TWELVEDATA_API_KEY").ok())
                .unwrap_or_default();

            if api_key.is_empty() {
                return QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(),
                    "Twelve Data API key required".to_string());
            }

            // Convert symbol format (e.g., NESN.SW -> NESN:SIX)
            let symbol = twelvedata::convert_symbol(&sec.symbol);

            match twelvedata::fetch_quote(&symbol, &api_key).await {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::PortfolioReport => {
            // feed_url contains the Portfolio Report UUID, or use ISIN as symbol
            let result = if let Some(ref uuid) = sec.feed_url {
                portfolio_report::fetch_quote(uuid).await
            } else {
                // Try to fetch by ISIN
                portfolio_report::fetch_quote_by_isin(&sec.symbol).await
            };

            match result {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::Finnhub => {
            // API key from request, feed_url, or environment
            let api_key = sec.api_key.clone()
                .or_else(|| sec.feed_url.clone())
                .or_else(|| std::env::var("FINNHUB_API_KEY").ok())
                .unwrap_or_default();

            if api_key.is_empty() {
                return QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(),
                    "Finnhub API key required".to_string());
            }

            match finnhub::fetch_quote(&sec.symbol, &api_key).await {
                Ok(quote) => QuoteResult::success(sec.id, sec.symbol.clone(), sec.provider.as_str(), quote),
                Err(e) => QuoteResult::error(sec.id, sec.symbol.clone(), sec.provider.as_str(), e.to_string()),
            }
        }
        ProviderType::Manual => {
            QuoteResult::error(sec.id, sec.symbol.clone(), "MANUAL", "Manual quotes not fetched automatically".to_string())
        }
        ProviderType::Ecb => {
            // ECB is for exchange rates, not securities
            QuoteResult::error(sec.id, sec.symbol.clone(), "ECB", "ECB provider is for exchange rates only".to_string())
        }
    }
}

/// Historische Kurse abrufen
pub async fn fetch_historical_quotes(
    symbol: &str,
    provider: ProviderType,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Quote>> {
    fetch_historical_quotes_with_options(symbol, provider, from, to, None, None).await
}

/// Historische Kurse abrufen mit Exchange-Suffix
pub async fn fetch_historical_quotes_with_exchange(
    symbol: &str,
    provider: ProviderType,
    from: NaiveDate,
    to: NaiveDate,
    exchange_suffix: Option<&str>,
) -> Result<Vec<Quote>> {
    fetch_historical_quotes_with_options(symbol, provider, from, to, None, exchange_suffix).await
}

/// Historische Kurse abrufen mit optionalem API-Key (Legacy-Kompatibilität)
pub async fn fetch_historical_quotes_with_key(
    symbol: &str,
    provider: ProviderType,
    from: NaiveDate,
    to: NaiveDate,
    api_key: Option<&str>,
) -> Result<Vec<Quote>> {
    fetch_historical_quotes_with_options(symbol, provider, from, to, api_key, None).await
}

/// Historische Kurse abrufen mit allen Optionen
pub async fn fetch_historical_quotes_with_options(
    symbol: &str,
    provider: ProviderType,
    from: NaiveDate,
    to: NaiveDate,
    api_key: Option<&str>,
    exchange_suffix: Option<&str>,
) -> Result<Vec<Quote>> {
    match provider {
        ProviderType::Yahoo | ProviderType::YahooAdjustedClose => {
            let adjusted = provider == ProviderType::YahooAdjustedClose;
            // Apply exchange suffix if provided and symbol doesn't already have one
            let full_symbol = if let Some(suffix) = exchange_suffix {
                if suffix.starts_with('.') && !symbol.contains('.') {
                    format!("{}{}", symbol, suffix)
                } else {
                    symbol.to_string()
                }
            } else {
                symbol.to_string()
            };
            yahoo::fetch_historical(&full_symbol, from, to, adjusted).await
        }
        ProviderType::CoinGecko => {
            let coin_id = coingecko::symbol_to_coin_id(symbol)
                .map(|s| s.to_string())
                .unwrap_or_else(|| symbol.to_lowercase());
            coingecko::fetch_historical(&coin_id, "EUR", from, to, api_key).await
        }
        ProviderType::Kraken => {
            // Use exchange_suffix as target currency or default to EUR
            let currency = exchange_suffix.unwrap_or("EUR");
            kraken::fetch_historical(symbol, currency, from, to).await
        }
        ProviderType::AlphaVantage => {
            let key = api_key
                .map(|s| s.to_string())
                .or_else(|| std::env::var("ALPHAVANTAGE_API_KEY").ok())
                .unwrap_or_default();
            if key.is_empty() {
                anyhow::bail!("Alpha Vantage API key required");
            }
            alphavantage::fetch_historical(symbol, &key, from, to, true).await
        }
        ProviderType::TwelveData => {
            let key = api_key
                .map(|s| s.to_string())
                .or_else(|| std::env::var("TWELVEDATA_API_KEY").ok())
                .unwrap_or_default();
            if key.is_empty() {
                anyhow::bail!("Twelve Data API key required");
            }
            let converted = twelvedata::convert_symbol(symbol);
            twelvedata::fetch_historical(&converted, &key, from, to).await
        }
        ProviderType::Finnhub => {
            let key = api_key
                .map(|s| s.to_string())
                .or_else(|| std::env::var("FINNHUB_API_KEY").ok())
                .unwrap_or_default();
            if key.is_empty() {
                anyhow::bail!("Finnhub API key required");
            }
            finnhub::fetch_historical(symbol, &key, from, to).await
        }
        ProviderType::PortfolioReport => {
            // symbol should be the Portfolio Report UUID
            portfolio_report::fetch_historical(symbol, "XFRA", from, to).await
        }
        _ => {
            anyhow::bail!("Historical quotes not supported for provider {:?}", provider)
        }
    }
}
