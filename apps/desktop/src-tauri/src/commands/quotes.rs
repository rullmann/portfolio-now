//! Quote Commands - Tauri IPC für Kursabfragen
//!
//! Commands:
//! - fetch_quotes: Aktuelle Kurse abrufen
//! - fetch_historical_prices: Historische Kurse abrufen
//! - sync_security_prices: Kurse in DB speichern
//! - sync_all_prices: Alle Securities aktualisieren
//! - fetch_exchange_rates: EZB Wechselkurse abrufen

use crate::db;
use crate::quotes::{self, alphavantage, ecb, tradingview, yahoo, ExchangeRate, LatestQuote, ProviderType, Quote, QuoteResult};
use futures::stream::{self, StreamExt};
use chrono::NaiveDate;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

/// Legacy-Format für Rückwärtskompatibilität
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyQuoteResult {
    pub symbol: String,
    pub date: NaiveDate,
    pub close: f64,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<i64>,
}

/// Aktuelle Kurse abrufen (Legacy-Kompatibilität)
#[command]
pub async fn fetch_quotes(symbols: Vec<String>) -> Result<Vec<LegacyQuoteResult>, String> {
    let mut results = Vec::new();

    for symbol in symbols {
        match yahoo::fetch_quote(&symbol, false).await {
            Ok(quote) => results.push(LegacyQuoteResult {
                symbol: quote.symbol,
                date: quote.quote.date,
                close: quote.quote.close,
                high: quote.quote.high,
                low: quote.quote.low,
                volume: quote.quote.volume,
            }),
            Err(e) => {
                log::warn!("Failed to fetch quote for {}: {}", symbol, e);
            }
        }
    }

    Ok(results)
}

/// API Keys für verschiedene Provider
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeys {
    pub finnhub: Option<String>,
    pub alpha_vantage: Option<String>,
    pub coingecko: Option<String>,
    pub twelve_data: Option<String>,
}

/// Kurse für Securities aus der DB abrufen und aktualisieren
#[command]
pub async fn sync_security_prices(
    security_ids: Vec<i64>,
    api_keys: Option<ApiKeys>,
) -> Result<Vec<QuoteResult>, String> {
    let securities = get_securities_for_sync(security_ids).map_err(|e| e.to_string())?;
    let keys = api_keys.unwrap_or_default();

    let requests: Vec<quotes::SecurityQuoteRequest> = securities
        .into_iter()
        .filter_map(|s| {
            // For current quotes: use latest_feed if set, otherwise fall back to feed
            let feed_to_use = s.latest_feed.as_ref()
                .filter(|f| !f.is_empty())
                .unwrap_or(&s.feed);
            let feed_url_to_use = if s.latest_feed.as_ref().filter(|f| !f.is_empty()).is_some() {
                s.latest_feed_url.clone()
            } else {
                s.feed_url.clone()
            };

            let provider = ProviderType::from_str(feed_to_use)?;
            // Use API key based on provider
            let api_key = match provider {
                ProviderType::Finnhub => keys.finnhub.clone(),
                ProviderType::AlphaVantage => keys.alpha_vantage.clone(),
                ProviderType::CoinGecko => keys.coingecko.clone(),
                ProviderType::TwelveData => keys.twelve_data.clone(),
                _ => None,
            };
            Some(quotes::SecurityQuoteRequest {
                id: s.id,
                symbol: s.ticker.or(s.isin).or(Some(s.name.clone()))?,
                provider,
                feed_url: feed_url_to_use,
                api_key,
                currency: s.currency,
            })
        })
        .collect();

    let results = quotes::fetch_all_quotes(requests).await;

    // Ergebnisse in DB speichern
    for result in &results {
        if result.success {
            if let Some(ref latest) = result.latest {
                if let Err(e) = save_quote_to_db(result.security_id, latest) {
                    log::error!(
                        "Failed to save quote for security {}: {}",
                        result.security_id,
                        e
                    );
                }
            }
        }
    }

    Ok(results)
}

/// Alle Securities synchronisieren
/// @param only_held - wenn true, werden nur Wertpapiere mit Bestand synchronisiert
/// @param api_keys - optionale API Keys für verschiedene Provider
#[command]
pub async fn sync_all_prices(
    only_held: Option<bool>,
    api_keys: Option<ApiKeys>,
) -> Result<SyncResult, String> {
    let securities = get_all_securities_for_sync(only_held.unwrap_or(true)).map_err(|e| e.to_string())?;
    let keys = api_keys.unwrap_or_default();

    let total = securities.len();
    log::info!("Syncing prices for {} securities", total);

    if total == 0 {
        return Ok(SyncResult {
            total: 0,
            success: 0,
            errors: 0,
            error_messages: vec!["Keine Wertpapiere mit Ticker oder ISIN gefunden".to_string()],
        });
    }

    let mut skipped = 0;
    let requests: Vec<quotes::SecurityQuoteRequest> = securities
        .into_iter()
        .filter_map(|s| {
            // For current quotes: use latest_feed if set, otherwise fall back to feed
            let feed_to_use = s.latest_feed.as_ref()
                .filter(|f| !f.is_empty())
                .map(|f| f.as_str())
                .unwrap_or(&s.feed);
            let feed_url_to_use = if s.latest_feed.as_ref().filter(|f| !f.is_empty()).is_some() {
                s.latest_feed_url.clone()
            } else {
                s.feed_url.clone()
            };

            let provider = match ProviderType::from_str(feed_to_use) {
                Some(p) => p,
                None => {
                    log::warn!("Unknown provider '{}' for security {}", feed_to_use, s.name);
                    skipped += 1;
                    return None;
                }
            };
            // Skip MANUAL provider
            if provider == ProviderType::Manual {
                skipped += 1;
                return None;
            }
            // Skip providers requiring API key if not provided
            if provider == ProviderType::Finnhub && keys.finnhub.is_none() {
                log::warn!("Skipping Finnhub security {} - no API key", s.name);
                skipped += 1;
                return None;
            }
            if provider == ProviderType::AlphaVantage && keys.alpha_vantage.is_none() {
                log::warn!("Skipping Alpha Vantage security {} - no API key", s.name);
                skipped += 1;
                return None;
            }
            if provider == ProviderType::TwelveData && keys.twelve_data.is_none() {
                log::warn!("Skipping Twelve Data security {} - no API key", s.name);
                skipped += 1;
                return None;
            }
            let symbol = s.ticker.or(s.isin).or(Some(s.name.clone()));
            if symbol.is_none() {
                log::warn!("No symbol for security {}", s.name);
                skipped += 1;
                return None;
            }
            // Use API key based on provider
            let api_key = match provider {
                ProviderType::Finnhub => keys.finnhub.clone(),
                ProviderType::AlphaVantage => keys.alpha_vantage.clone(),
                ProviderType::CoinGecko => keys.coingecko.clone(),
                ProviderType::TwelveData => keys.twelve_data.clone(),
                _ => None,
            };
            log::info!("Will fetch {} with provider {:?}", symbol.as_ref().unwrap(), provider);
            Some(quotes::SecurityQuoteRequest {
                id: s.id,
                symbol: symbol?,
                provider,
                feed_url: feed_url_to_use,
                api_key,
                currency: s.currency,
            })
        })
        .collect();

    log::info!("Fetching quotes for {} securities (skipped {})", requests.len(), skipped);

    let results = quotes::fetch_all_quotes(requests).await;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for result in &results {
        if result.success {
            if let Some(ref latest) = result.latest {
                match save_quote_to_db(result.security_id, latest) {
                    Ok(_) => {
                        log::info!("Saved quote for {}: {}", result.symbol, latest.quote.close);
                        success_count += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to save quote for {}: {}", result.symbol, e);
                        error_count += 1;
                        errors.push(format!("{}: {}", result.symbol, e));
                    }
                }
            }
        } else {
            error_count += 1;
            if let Some(ref err) = result.error {
                log::error!("Quote fetch error for {}: {}", result.symbol, err);
                errors.push(format!("{}: {}", result.symbol, err));
            }
        }
    }

    log::info!("Sync complete: {} success, {} errors", success_count, error_count);

    Ok(SyncResult {
        total,
        success: success_count,
        errors: error_count,
        error_messages: errors,
    })
}

/// Historische Kurse abrufen
#[command]
pub async fn fetch_historical_prices(
    security_id: i64,
    from: String,
    to: String,
    api_keys: Option<ApiKeys>,
) -> Result<Vec<Quote>, String> {
    let security = get_security_by_id(security_id).map_err(|e| e.to_string())?;

    let provider = ProviderType::from_str(&security.feed)
        .ok_or_else(|| format!("Unknown provider: {}", security.feed))?;

    let symbol = security
        .ticker
        .or(security.isin)
        .ok_or("Security has no ticker or ISIN")?;

    let from_date =
        NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|e| format!("Invalid from date: {}", e))?;
    let to_date =
        NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|e| format!("Invalid to date: {}", e))?;

    // Get API key for provider
    let keys = api_keys.unwrap_or_default();
    let api_key = match provider {
        ProviderType::Finnhub => keys.finnhub.as_deref(),
        ProviderType::AlphaVantage => keys.alpha_vantage.as_deref(),
        ProviderType::CoinGecko => keys.coingecko.as_deref(),
        ProviderType::TwelveData => keys.twelve_data.as_deref(),
        _ => None,
    };

    // Get exchange suffix for Yahoo
    let exchange_suffix = if matches!(provider, ProviderType::Yahoo | ProviderType::YahooAdjustedClose) {
        security.feed_url.as_deref()
    } else {
        None
    };

    let quotes = quotes::fetch_historical_quotes_with_options(&symbol, provider, from_date, to_date, api_key, exchange_suffix)
        .await
        .map_err(|e| e.to_string())?;

    // Optional: Historische Kurse in DB speichern
    if let Err(e) = save_historical_quotes_to_db(security_id, &quotes) {
        log::warn!(
            "Failed to save historical quotes for security {}: {}",
            security_id,
            e
        );
    }

    Ok(quotes)
}

/// EZB Wechselkurse abrufen
#[command]
pub async fn fetch_exchange_rates() -> Result<Vec<ExchangeRate>, String> {
    let rates = ecb::fetch_latest_rates().await.map_err(|e| e.to_string())?;

    // In DB speichern
    if let Err(e) = save_exchange_rates_to_db(&rates) {
        log::warn!("Failed to save exchange rates: {}", e);
    }

    Ok(rates)
}

/// Wechselkurs für ein Währungspaar abrufen
#[command]
pub async fn fetch_exchange_rate(base: String, target: String) -> Result<ExchangeRate, String> {
    ecb::fetch_rate(&base, &target)
        .await
        .map_err(|e| e.to_string())
}

/// Historische Wechselkurse abrufen
#[command]
pub async fn fetch_historical_exchange_rates(
    from: String,
    to: String,
) -> Result<Vec<ExchangeRate>, String> {
    let from_date =
        NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|e| format!("Invalid from date: {}", e))?;
    let to_date =
        NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|e| format!("Invalid to date: {}", e))?;

    let rates_by_date = ecb::fetch_historical_rates(from_date, to_date)
        .await
        .map_err(|e| e.to_string())?;

    // Flatten und in DB speichern
    let all_rates: Vec<ExchangeRate> = rates_by_date.into_values().flatten().collect();

    if let Err(e) = save_exchange_rates_to_db(&all_rates) {
        log::warn!("Failed to save historical exchange rates: {}", e);
    }

    Ok(all_rates)
}

/// Verfügbare Quote Provider abrufen
/// Gibt alle Provider zurück, die verwendet werden können
/// Provider, die einen API-Key benötigen, werden nur zurückgegeben wenn der Key vorhanden ist
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteProvider {
    pub id: String,
    pub name: String,
    pub requires_api_key: bool,
    pub supports_historical: bool,
}

#[command]
pub async fn get_available_quote_providers(api_keys: Option<ApiKeys>) -> Result<Vec<QuoteProvider>, String> {
    let keys = api_keys.unwrap_or_default();

    let mut providers = vec![
        QuoteProvider {
            id: "YAHOO".to_string(),
            name: "Yahoo Finance".to_string(),
            requires_api_key: false,
            supports_historical: true,
        },
        QuoteProvider {
            id: "YAHOO-ADJUSTEDCLOSE".to_string(),
            name: "Yahoo Finance (Adjusted)".to_string(),
            requires_api_key: false,
            supports_historical: true,
        },
        QuoteProvider {
            id: "COINGECKO".to_string(),
            name: "CoinGecko".to_string(),
            requires_api_key: false,
            supports_historical: true,
        },
    ];

    // Add Finnhub only if API key is provided
    if keys.finnhub.is_some() {
        providers.push(QuoteProvider {
            id: "FINNHUB".to_string(),
            name: "Finnhub".to_string(),
            requires_api_key: true,
            supports_historical: true,
        });
    }

    // Add Alpha Vantage only if API key is provided
    if keys.alpha_vantage.is_some() {
        providers.push(QuoteProvider {
            id: "ALPHAVANTAGE".to_string(),
            name: "Alpha Vantage".to_string(),
            requires_api_key: true,
            supports_historical: true,
        });
    }

    Ok(providers)
}

// ============== External Security Search ==============

/// Search result from external providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSecuritySearchResult {
    pub symbol: String,
    pub name: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub security_type: Option<String>,
    pub currency: Option<String>,
    pub region: Option<String>,
    pub provider: String,
    pub provider_id: Option<String>,
}

/// Response from external security search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSearchResponse {
    pub results: Vec<ExternalSecuritySearchResult>,
    pub providers_used: Vec<String>,
    pub errors: Vec<String>,
}

/// Search for securities from external providers (Yahoo Finance, Alpha Vantage)
/// Results can be added to watchlist and then to the database.
#[command]
pub async fn search_external_securities(
    query: String,
    alpha_vantage_api_key: Option<String>,
) -> Result<ExternalSearchResponse, String> {
    let query = query.trim();
    if query.len() < 2 {
        return Ok(ExternalSearchResponse {
            results: vec![],
            providers_used: vec![],
            errors: vec!["Suchanfrage muss mindestens 2 Zeichen haben".to_string()],
        });
    }

    let mut all_results: Vec<ExternalSecuritySearchResult> = Vec::new();
    let mut providers_used: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // 1. Search Yahoo Finance (no API key needed, global coverage)
    match yahoo::search(query).await {
        Ok(results) => {
            providers_used.push("YAHOO".to_string());
            for r in results {
                all_results.push(ExternalSecuritySearchResult {
                    symbol: r.symbol.clone(),
                    name: r.name,
                    isin: None, // Yahoo doesn't return ISIN
                    wkn: None,
                    security_type: Some(r.security_type),
                    currency: None, // Currency determined by exchange
                    region: Some(r.exchange),
                    provider: "YAHOO".to_string(),
                    provider_id: None,
                });
            }
        }
        Err(e) => {
            log::warn!("Yahoo Finance search error: {}", e);
            errors.push(format!("Yahoo Finance: {}", e));
        }
    }

    // 2. Search Alpha Vantage (if API key provided)
    if let Some(ref api_key) = alpha_vantage_api_key {
        if !api_key.is_empty() {
            match alphavantage::search(query, api_key).await {
                Ok(results) => {
                    providers_used.push("ALPHAVANTAGE".to_string());
                    for r in results {
                        // Avoid duplicates by symbol
                        let is_duplicate = all_results.iter().any(|existing| {
                            existing.symbol.to_uppercase() == r.symbol.to_uppercase()
                        });

                        if !is_duplicate {
                            all_results.push(ExternalSecuritySearchResult {
                                symbol: r.symbol,
                                name: r.name,
                                isin: None, // Alpha Vantage doesn't return ISIN
                                wkn: None,
                                security_type: Some(r.security_type),
                                currency: Some(r.currency),
                                region: Some(r.region),
                                provider: "ALPHAVANTAGE".to_string(),
                                provider_id: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Alpha Vantage search error: {}", e);
                    errors.push(format!("Alpha Vantage: {}", e));
                }
            }
        }
    }

    // Sort results: exact matches first, then by name length
    let query_lower = query.to_lowercase();
    all_results.sort_by(|a, b| {
        let a_exact = a.symbol.to_lowercase() == query_lower
            || a.isin.as_ref().map(|i| i.to_lowercase() == query_lower).unwrap_or(false);
        let b_exact = b.symbol.to_lowercase() == query_lower
            || b.isin.as_ref().map(|i| i.to_lowercase() == query_lower).unwrap_or(false);

        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.len().cmp(&b.name.len()),
        }
    });

    // Limit to 25 results
    all_results.truncate(25);

    Ok(ExternalSearchResponse {
        results: all_results,
        providers_used,
        errors,
    })
}

// ============== Hilfsstrukturen ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub total: usize,
    pub success: usize,
    pub errors: usize,
    pub error_messages: Vec<String>,
}

#[derive(Debug)]
#[derive(Clone)]
struct SecurityInfo {
    id: i64,
    name: String,
    feed: String,                       // Provider for historical quotes
    feed_url: Option<String>,           // URL/suffix for historical quotes
    latest_feed: Option<String>,        // Provider for current quotes (falls back to feed if None)
    latest_feed_url: Option<String>,    // URL/suffix for current quotes
    ticker: Option<String>,
    isin: Option<String>,
    currency: Option<String>,           // Security's currency (for crypto providers)
}

// ============== Datenbank-Funktionen ==============

fn get_securities_for_sync(ids: Vec<i64>) -> anyhow::Result<Vec<SecurityInfo>> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("DB not initialized"))?;

    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, name, COALESCE(feed, 'YAHOO') as feed, feed_url, latest_feed, latest_feed_url, ticker, isin, currency
         FROM pp_security WHERE id IN ({})",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

    let securities = stmt
        .query_map(params.as_slice(), |row| {
            Ok(SecurityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                feed: row.get(2)?,
                feed_url: row.get(3)?,
                latest_feed: row.get(4)?,
                latest_feed_url: row.get(5)?,
                ticker: row.get(6)?,
                isin: row.get(7)?,
                currency: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(securities)
}

fn get_all_securities_for_sync(only_held: bool) -> anyhow::Result<Vec<SecurityInfo>> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("DB not initialized"))?;

    // Get securities with valid feed configuration
    // COALESCE handles NULL, NULLIF handles empty strings
    // If only_held is true, only include securities with current holdings > 0
    let sql = if only_held {
        "SELECT s.id, s.name,
                COALESCE(NULLIF(s.feed, ''), 'YAHOO') as feed,
                s.feed_url, s.latest_feed, s.latest_feed_url, s.ticker, s.isin, s.currency
         FROM pp_security s
         WHERE s.is_retired = 0
           AND (s.ticker IS NOT NULL AND s.ticker != ''
                OR s.isin IS NOT NULL AND s.isin != '')
           AND (
               SELECT COALESCE(SUM(CASE
                   WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                   WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                   ELSE 0
               END), 0)
               FROM pp_txn t
               WHERE t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
           ) > 0"
    } else {
        "SELECT id, name,
                COALESCE(NULLIF(feed, ''), 'YAHOO') as feed,
                feed_url, latest_feed, latest_feed_url, ticker, isin, currency
         FROM pp_security
         WHERE is_retired = 0
           AND (ticker IS NOT NULL AND ticker != ''
                OR isin IS NOT NULL AND isin != '')"
    };

    let mut stmt = conn.prepare(sql)?;

    let securities: Vec<SecurityInfo> = stmt
        .query_map([], |row| {
            Ok(SecurityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                feed: row.get(2)?,
                feed_url: row.get(3)?,
                latest_feed: row.get(4)?,
                latest_feed_url: row.get(5)?,
                ticker: row.get(6)?,
                isin: row.get(7)?,
                currency: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    log::info!("Found {} securities for sync (only_held={})", securities.len(), only_held);
    Ok(securities)
}

fn get_security_by_id(id: i64) -> anyhow::Result<SecurityInfo> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("DB not initialized"))?;

    conn.query_row(
        "SELECT id, name, COALESCE(feed, 'YAHOO') as feed, feed_url, latest_feed, latest_feed_url, ticker, isin, currency
         FROM pp_security WHERE id = ?",
        params![id],
        |row| {
            Ok(SecurityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                feed: row.get(2)?,
                feed_url: row.get(3)?,
                latest_feed: row.get(4)?,
                latest_feed_url: row.get(5)?,
                ticker: row.get(6)?,
                isin: row.get(7)?,
                currency: row.get(8)?,
            })
        },
    )
    .map_err(|e| anyhow::anyhow!("Security not found: {}", e))
}

fn save_quote_to_db(security_id: i64, quote: &LatestQuote) -> anyhow::Result<()> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("DB not initialized"))?;

    let price_value = quotes::price_to_db(quote.quote.close);
    let high_value = quote.quote.high.map(quotes::price_to_db);
    let low_value = quote.quote.low.map(quotes::price_to_db);

    // Latest Price aktualisieren
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    conn.execute(
        "INSERT OR REPLACE INTO pp_latest_price (security_id, date, value, high, low, volume, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            security_id,
            quote.quote.date.to_string(),
            price_value,
            high_value,
            low_value,
            quote.quote.volume,
            now,
        ],
    )?;

    // Auch in historische Preise einfügen
    conn.execute(
        "INSERT OR REPLACE INTO pp_price (security_id, date, value)
         VALUES (?, ?, ?)",
        params![security_id, quote.quote.date.to_string(), price_value],
    )?;

    Ok(())
}

fn save_historical_quotes_to_db(security_id: i64, quotes: &[Quote]) -> anyhow::Result<()> {
    let mut conn_guard = db::get_connection()?;
    let conn = conn_guard.as_mut().ok_or(anyhow::anyhow!("DB not initialized"))?;

    let tx = conn.transaction()?;

    for quote in quotes {
        let price_value = quotes::price_to_db(quote.close);
        tx.execute(
            "INSERT OR REPLACE INTO pp_price (security_id, date, value)
             VALUES (?, ?, ?)",
            params![security_id, quote.date.to_string(), price_value],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn save_exchange_rates_to_db(rates: &[ExchangeRate]) -> anyhow::Result<()> {
    let mut conn_guard = db::get_connection()?;
    let conn = conn_guard.as_mut().ok_or(anyhow::anyhow!("DB not initialized"))?;

    let tx = conn.transaction()?;

    for rate in rates {
        // Store rate as decimal string for precision (matches lookup_rate expectation)
        tx.execute(
            "INSERT OR REPLACE INTO pp_exchange_rate (base_currency, term_currency, date, rate)
             VALUES (?, ?, ?, ?)",
            params![rate.base, rate.target, rate.date.to_string(), rate.rate.to_string()],
        )?;
    }

    tx.commit()?;
    Ok(())
}

// ============== Stock Split Detection ==============

/// Result of detecting stock splits for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitDetectionResult {
    pub security_id: i64,
    pub security_name: String,
    pub symbol: String,
    pub splits_found: usize,
    pub splits_new: usize,
    pub splits: Vec<DetectedSplit>,
}

/// A detected stock split
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedSplit {
    pub date: String,
    pub ratio: String,
    pub numerator: f64,
    pub denominator: f64,
    pub source: String,
    pub is_new: bool,
    pub is_applied: bool,
}

/// Detect stock splits for a single security using Yahoo Finance
#[command]
pub async fn detect_security_splits(
    security_id: i64,
) -> Result<SplitDetectionResult, String> {
    let security = get_security_by_id(security_id).map_err(|e| e.to_string())?;

    // Get symbol for Yahoo
    let symbol = security.ticker
        .or(security.isin.clone())
        .ok_or_else(|| "Security has no ticker or ISIN".to_string())?;

    // Fetch historical data with splits (max range for complete split history)
    let from = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
    let to = chrono::Utc::now().date_naive();

    let data = yahoo::fetch_historical_with_splits(&symbol, from, to, false)
        .await
        .map_err(|e| format!("Failed to fetch splits for {}: {}", symbol, e))?;

    let mut detected_splits = Vec::new();
    let mut new_count = 0;

    for split in data.splits {
        let is_new = !is_split_already_recorded(security_id, &split.date.to_string())?;
        let is_applied = false; // Will be set after checking DB

        if is_new {
            // Save new split to database
            save_split_to_db(security_id, &split, "YAHOO")?;
            new_count += 1;
        }

        detected_splits.push(DetectedSplit {
            date: split.date.to_string(),
            ratio: split.ratio_str(),
            numerator: split.numerator,
            denominator: split.denominator,
            source: "YAHOO".to_string(),
            is_new,
            is_applied,
        });
    }

    log::info!(
        "Split detection for {} ({}): {} found, {} new",
        security.name, symbol, detected_splits.len(), new_count
    );

    Ok(SplitDetectionResult {
        security_id,
        security_name: security.name,
        symbol,
        splits_found: detected_splits.len(),
        splits_new: new_count,
        splits: detected_splits,
    })
}

/// Detect stock splits for all Yahoo-based securities
#[command]
pub async fn detect_all_splits(
    only_held: Option<bool>,
) -> Result<Vec<SplitDetectionResult>, String> {
    let securities = get_all_securities_for_sync(only_held.unwrap_or(true))
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for security in securities {
        // Only process Yahoo-based securities
        if !security.feed.to_uppercase().contains("YAHOO") {
            continue;
        }

        match detect_security_splits(security.id).await {
            Ok(result) => {
                if result.splits_found > 0 {
                    results.push(result);
                }
            }
            Err(e) => {
                log::warn!("Failed to detect splits for {}: {}", security.name, e);
            }
        }
    }

    Ok(results)
}

/// Get all recorded corporate actions for a security
#[command]
pub fn get_corporate_actions(
    security_id: Option<i64>,
) -> Result<Vec<CorporateActionRecord>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let sql = if security_id.is_some() {
        "SELECT ca.id, ca.security_id, s.name, ca.action_type, ca.effective_date,
                ca.ratio_from, ca.ratio_to, ca.old_identifier, ca.new_identifier,
                ca.successor_security_id, ca.source, ca.confidence,
                ca.is_applied, ca.is_confirmed, ca.note, ca.created_at
         FROM pp_corporate_action ca
         JOIN pp_security s ON s.id = ca.security_id
         WHERE ca.security_id = ?1
         ORDER BY ca.effective_date DESC"
    } else {
        "SELECT ca.id, ca.security_id, s.name, ca.action_type, ca.effective_date,
                ca.ratio_from, ca.ratio_to, ca.old_identifier, ca.new_identifier,
                ca.successor_security_id, ca.source, ca.confidence,
                ca.is_applied, ca.is_confirmed, ca.note, ca.created_at
         FROM pp_corporate_action ca
         JOIN pp_security s ON s.id = ca.security_id
         ORDER BY ca.effective_date DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let params: Vec<&dyn rusqlite::ToSql> = if let Some(id) = security_id.as_ref() {
        vec![id as &dyn rusqlite::ToSql]
    } else {
        vec![]
    };

    let actions = stmt
        .query_map(params.as_slice(), |row| {
            Ok(CorporateActionRecord {
                id: row.get(0)?,
                security_id: row.get(1)?,
                security_name: row.get(2)?,
                action_type: row.get(3)?,
                effective_date: row.get(4)?,
                ratio_from: row.get(5)?,
                ratio_to: row.get(6)?,
                old_identifier: row.get(7)?,
                new_identifier: row.get(8)?,
                successor_security_id: row.get(9)?,
                source: row.get(10)?,
                confidence: row.get(11)?,
                is_applied: row.get(12)?,
                is_confirmed: row.get(13)?,
                note: row.get(14)?,
                created_at: row.get(15)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(actions)
}

/// Corporate action record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporateActionRecord {
    pub id: i64,
    pub security_id: i64,
    pub security_name: String,
    pub action_type: String,
    pub effective_date: String,
    pub ratio_from: Option<i32>,
    pub ratio_to: Option<i32>,
    pub old_identifier: Option<String>,
    pub new_identifier: Option<String>,
    pub successor_security_id: Option<i64>,
    pub source: String,
    pub confidence: Option<f64>,
    pub is_applied: bool,
    pub is_confirmed: bool,
    pub note: Option<String>,
    pub created_at: Option<String>,
}

// ============== Split Detection Helpers ==============

fn is_split_already_recorded(security_id: i64, date: &str) -> Result<bool, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_corporate_action
             WHERE security_id = ?1 AND effective_date = ?2
             AND action_type IN ('STOCK_SPLIT', 'REVERSE_SPLIT')",
            params![security_id, date],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(count > 0)
}

fn save_split_to_db(security_id: i64, split: &quotes::SplitEvent, source: &str) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let action_type = if split.is_forward_split() {
        "STOCK_SPLIT"
    } else {
        "REVERSE_SPLIT"
    };

    conn.execute(
        "INSERT INTO pp_corporate_action
         (security_id, action_type, effective_date, ratio_from, ratio_to, source, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            security_id,
            action_type,
            split.date.to_string(),
            split.numerator as i32,
            split.denominator as i32,
            source,
            0.95  // High confidence for Yahoo data
        ],
    )
    .map_err(|e| e.to_string())?;

    log::info!(
        "Saved {} {} for security {} on {}",
        action_type, split.ratio_str(), security_id, split.date
    );

    Ok(())
}

// ============== Price Jump Heuristic ==============

/// Known split ratios and their expected price change factors
/// For a forward split n:1, price should drop to 1/n of original
const SPLIT_RATIOS: [(i32, i32, f64, f64); 8] = [
    // (numerator, denominator, min_factor, max_factor)
    // Factor is new_price / old_price
    (2, 1, 0.45, 0.55),   // 2:1 split → price ~50%
    (3, 1, 0.30, 0.37),   // 3:1 split → price ~33%
    (4, 1, 0.22, 0.28),   // 4:1 split → price ~25%
    (5, 1, 0.17, 0.23),   // 5:1 split → price ~20%
    (7, 1, 0.12, 0.17),   // 7:1 split → price ~14%
    (10, 1, 0.08, 0.12),  // 10:1 split → price ~10%
    (20, 1, 0.04, 0.06),  // 20:1 split → price ~5%
    (50, 1, 0.015, 0.025),// 50:1 split → price ~2%
];

/// Result of a potential split detected by price heuristic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicSplitResult {
    pub security_id: i64,
    pub security_name: String,
    pub date: String,
    pub ratio: String,
    pub numerator: i32,
    pub denominator: i32,
    pub price_before: f64,
    pub price_after: f64,
    pub price_change_factor: f64,
    pub confidence: f64,
    pub is_new: bool,
}

/// Result of heuristic split detection for all securities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicDetectionResult {
    pub securities_analyzed: usize,
    pub potential_splits_found: usize,
    pub new_splits_saved: usize,
    pub splits: Vec<HeuristicSplitResult>,
}

/// Detect potential stock splits by analyzing price jumps in historical data
#[command]
pub fn detect_splits_by_price_heuristic(
    security_id: Option<i64>,
    min_confidence: Option<f64>,
) -> Result<HeuristicDetectionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let min_conf = min_confidence.unwrap_or(0.6);

    // Get securities to analyze
    let securities: Vec<(i64, String)> = if let Some(id) = security_id {
        let name: String = conn
            .query_row("SELECT name FROM pp_security WHERE id = ?1", params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        vec![(id, name)]
    } else {
        let mut stmt = conn
            .prepare("SELECT id, name FROM pp_security WHERE is_retired = 0")
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    let mut all_splits = Vec::new();
    let mut new_count = 0;

    for (sec_id, sec_name) in &securities {
        // Get price history ordered by date
        let mut stmt = conn
            .prepare(
                "SELECT date, value FROM pp_price
                 WHERE security_id = ?1
                 ORDER BY date ASC"
            )
            .map_err(|e| e.to_string())?;

        let prices: Vec<(String, i64)> = stmt
            .query_map(params![sec_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        if prices.len() < 2 {
            continue;
        }

        // Analyze consecutive prices for jumps
        for window in prices.windows(2) {
            let (_date_before, value_before) = &window[0];
            let (date_after, value_after) = &window[1];

            // Convert from scaled int (10^8) to f64
            let price_before = *value_before as f64 / 100_000_000.0;
            let price_after = *value_after as f64 / 100_000_000.0;

            if price_before <= 0.0 {
                continue;
            }

            let factor = price_after / price_before;

            // Check if this matches a known split ratio
            if let Some((num, denom, confidence)) = match_split_ratio(factor, min_conf) {
                // Check if already recorded
                if is_split_already_recorded(*sec_id, date_after)? {
                    continue;
                }

                // Also check pp_security_event for PP-imported splits
                let pp_event_exists: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM pp_security_event
                         WHERE security_id = ?1 AND date = ?2 AND event_type = 'STOCK_SPLIT'",
                        params![sec_id, date_after],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                if pp_event_exists > 0 {
                    continue;
                }

                // Save to database with lower confidence (heuristic detection)
                save_heuristic_split_to_db(*sec_id, date_after, num, denom, confidence, price_before, price_after)?;
                new_count += 1;

                all_splits.push(HeuristicSplitResult {
                    security_id: *sec_id,
                    security_name: sec_name.clone(),
                    date: date_after.clone(),
                    ratio: format!("{}:{}", num, denom),
                    numerator: num,
                    denominator: denom,
                    price_before,
                    price_after,
                    price_change_factor: factor,
                    confidence,
                    is_new: true,
                });

                log::info!(
                    "Heuristic: Detected potential {}:{} split for {} on {} (price {} → {}, factor {:.3})",
                    num, denom, sec_name, date_after, price_before, price_after, factor
                );
            }
        }
    }

    Ok(HeuristicDetectionResult {
        securities_analyzed: securities.len(),
        potential_splits_found: all_splits.len(),
        new_splits_saved: new_count,
        splits: all_splits,
    })
}

/// Match a price change factor to a known split ratio
fn match_split_ratio(factor: f64, min_confidence: f64) -> Option<(i32, i32, f64)> {
    for (num, denom, min_factor, max_factor) in SPLIT_RATIOS.iter() {
        if factor >= *min_factor && factor <= *max_factor {
            // Calculate confidence based on how close to ideal ratio
            let ideal_factor = *denom as f64 / *num as f64;
            let deviation = (factor - ideal_factor).abs() / ideal_factor;
            let confidence = (1.0 - deviation * 2.0).max(0.5).min(0.85);

            if confidence >= min_confidence {
                return Some((*num, *denom, confidence));
            }
        }
    }
    None
}

/// Save a heuristically detected split to database
fn save_heuristic_split_to_db(
    security_id: i64,
    date: &str,
    numerator: i32,
    denominator: i32,
    confidence: f64,
    price_before: f64,
    price_after: f64,
) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let note = format!(
        "Heuristically detected: price {:.2} → {:.2} (factor {:.3})",
        price_before, price_after, price_after / price_before
    );

    conn.execute(
        "INSERT INTO pp_corporate_action
         (security_id, action_type, effective_date, ratio_from, ratio_to, source, confidence, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            security_id,
            "STOCK_SPLIT",
            date,
            numerator,
            denominator,
            "DETECTED",
            confidence,
            note,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Provider Status
// ============================================================================

/// Status of quote providers for the portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    /// Total securities count
    pub total_securities: usize,
    /// Securities with working providers (free or API key configured)
    pub configured_count: usize,
    /// Securities missing API keys
    pub missing_api_key_count: usize,
    /// Securities with manual/no provider
    pub manual_count: usize,
    /// List of providers that need API keys but don't have them
    pub missing_providers: Vec<ProviderInfo>,
    /// Securities grouped by provider
    pub by_provider: Vec<ProviderSecurityCount>,
    /// Securities that cannot sync (missing API key or no provider)
    pub cannot_sync: Vec<SecurityProviderInfo>,
    /// Quote sync status for today
    pub quote_status: QuoteSyncStatus,
}

/// Status of quote synchronization for held securities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteSyncStatus {
    /// Total held securities (with positions > 0)
    pub held_count: usize,
    /// Securities with quotes from today
    pub synced_today_count: usize,
    /// Securities with outdated or no quotes
    pub outdated_count: usize,
    /// Today's date (for reference)
    pub today: String,
    /// Securities with outdated quotes (name, last quote date)
    pub outdated_securities: Vec<OutdatedQuoteInfo>,
}

/// Info about a security with outdated or missing quote
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutdatedQuoteInfo {
    pub id: i64,
    pub name: String,
    pub ticker: Option<String>,
    pub last_quote_date: Option<String>,
    pub days_old: Option<i64>,
}

/// Info about a provider that needs an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub securities_count: usize,
    pub requires_api_key: bool,
    pub has_api_key: bool,
}

/// Count of securities per provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecurityCount {
    pub provider: String,
    pub count: usize,
    pub can_sync: bool,
}

/// Security that cannot sync prices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProviderInfo {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub reason: String,
}

/// Get status of all quote providers
#[command]
pub fn get_provider_status(api_keys: Option<ApiKeys>) -> Result<ProviderStatus, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let keys = api_keys.unwrap_or_default();

    // Get all securities with their providers
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name,
                COALESCE(NULLIF(s.latest_feed, ''), NULLIF(s.feed, ''), 'YAHOO') as provider,
                s.ticker, s.isin
         FROM pp_security s
         WHERE s.is_retired = 0"
    ).map_err(|e| e.to_string())?;

    let securities: Vec<(i64, String, String, Option<String>, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total_securities = securities.len();
    let mut configured_count = 0;
    let mut missing_api_key_count = 0;
    let mut manual_count = 0;
    let mut cannot_sync: Vec<SecurityProviderInfo> = Vec::new();
    let mut provider_counts: std::collections::HashMap<String, (usize, bool)> = std::collections::HashMap::new();

    // Check which providers need API keys
    let providers_needing_keys = ["FINNHUB", "ALPHA-VANTAGE", "ALPHA_VANTAGE", "TWELVE-DATA", "TWELVE_DATA"];

    for (id, name, provider, ticker, isin) in &securities {
        let provider_upper = provider.to_uppercase();
        let has_symbol = ticker.is_some() || isin.is_some();

        // Check if provider needs API key
        let needs_key = providers_needing_keys.iter().any(|p| provider_upper.contains(p));
        let has_key = if provider_upper.contains("FINNHUB") {
            keys.finnhub.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else if provider_upper.contains("ALPHA") {
            keys.alpha_vantage.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else if provider_upper.contains("TWELVE") {
            keys.twelve_data.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else {
            true // Free providers don't need keys
        };

        let is_manual = provider_upper == "MANUAL" || provider_upper.is_empty();
        let can_sync = !is_manual && has_symbol && (!needs_key || has_key);

        // Update provider counts
        let entry = provider_counts.entry(provider.clone()).or_insert((0, can_sync));
        entry.0 += 1;

        if is_manual {
            manual_count += 1;
        } else if needs_key && !has_key {
            missing_api_key_count += 1;
            cannot_sync.push(SecurityProviderInfo {
                id: *id,
                name: name.clone(),
                provider: provider.clone(),
                reason: format!("{} benötigt API-Key", provider),
            });
        } else if !has_symbol {
            cannot_sync.push(SecurityProviderInfo {
                id: *id,
                name: name.clone(),
                provider: provider.clone(),
                reason: "Kein Ticker oder ISIN".to_string(),
            });
        } else {
            configured_count += 1;
        }
    }

    // Build provider info list
    let mut missing_providers: Vec<ProviderInfo> = Vec::new();
    let mut by_provider: Vec<ProviderSecurityCount> = Vec::new();

    for (provider, (count, can_sync)) in &provider_counts {
        let provider_upper = provider.to_uppercase();
        let needs_key = providers_needing_keys.iter().any(|p| provider_upper.contains(p));
        let has_key = if provider_upper.contains("FINNHUB") {
            keys.finnhub.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else if provider_upper.contains("ALPHA") {
            keys.alpha_vantage.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else if provider_upper.contains("TWELVE") {
            keys.twelve_data.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
        } else {
            true
        };

        by_provider.push(ProviderSecurityCount {
            provider: provider.clone(),
            count: *count,
            can_sync: *can_sync,
        });

        if needs_key && !has_key {
            missing_providers.push(ProviderInfo {
                name: provider.clone(),
                securities_count: *count,
                requires_api_key: true,
                has_api_key: false,
            });
        }
    }

    // Sort by count descending
    by_provider.sort_by(|a, b| b.count.cmp(&a.count));

    // Query quote sync status for held securities
    let quote_status = get_quote_sync_status(conn)?;

    Ok(ProviderStatus {
        total_securities,
        configured_count,
        missing_api_key_count,
        manual_count,
        missing_providers,
        by_provider,
        cannot_sync,
        quote_status,
    })
}

/// Get quote sync status for held securities
fn get_quote_sync_status(conn: &rusqlite::Connection) -> Result<QuoteSyncStatus, String> {
    let today = chrono::Utc::now().date_naive();
    let today_str = today.to_string();

    // Query all held securities with their latest quote dates
    let sql = r#"
        SELECT
            s.id,
            s.name,
            s.ticker,
            lp.date as last_quote_date,
            julianday(?) - julianday(lp.date) as days_old
        FROM pp_security s
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        WHERE s.is_retired = 0
          AND (
              SELECT COALESCE(SUM(CASE
                  WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                  WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                  ELSE 0
              END), 0)
              FROM pp_txn t
              WHERE t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
          ) > 0
        ORDER BY days_old DESC NULLS FIRST
    "#;

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([&today_str], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<f64>>(4)?,
        ))
    }).map_err(|e| e.to_string())?;

    let mut held_count = 0;
    let mut synced_today_count = 0;
    let mut outdated_securities: Vec<OutdatedQuoteInfo> = Vec::new();

    for row in rows {
        let (id, name, ticker, last_quote_date, days_old_f) = row.map_err(|e| e.to_string())?;
        held_count += 1;

        let days_old = days_old_f.map(|d| d.round() as i64);

        // Check if synced today (days_old == 0 or quote date == today)
        let is_today = match &last_quote_date {
            Some(date) => date == &today_str,
            None => false,
        };

        if is_today || days_old == Some(0) {
            synced_today_count += 1;
        } else {
            outdated_securities.push(OutdatedQuoteInfo {
                id,
                name,
                ticker,
                last_quote_date,
                days_old,
            });
        }
    }

    let outdated_count = outdated_securities.len();

    Ok(QuoteSyncStatus {
        held_count,
        synced_today_count,
        outdated_count,
        today: today_str,
        outdated_securities,
    })
}

// ============================================================================
// Quote Provider Suggestions
// ============================================================================

use crate::quotes::suggestion::{self, QuoteSuggestion, SecurityForSuggestion};

/// Get quote provider suggestions for securities without configured feed
#[command]
pub fn suggest_quote_providers(
    portfolio_id: Option<i64>,
    held_only: Option<bool>,
) -> Result<Vec<QuoteSuggestion>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    // Valid feed providers that can fetch quotes automatically
    // Note: MANUAL is excluded as it cannot auto-fetch
    // Condition checks for missing/invalid feed (NULL, empty, or not in valid list)
    let missing_feed_condition = "(s.feed IS NULL OR TRIM(s.feed) = '' OR UPPER(TRIM(s.feed)) NOT IN ('YAHOO', 'YAHOO-ADJUSTEDCLOSE', 'COINGECKO', 'KRAKEN', 'FINNHUB', 'ALPHAVANTAGE', 'TWELVEDATA', 'TRADINGVIEW'))";

    // Get securities without valid auto-fetch feed
    // Three modes:
    // 1. portfolio_id = Some(id) -> only securities held in specific portfolio
    // 2. held_only = true -> only securities held in any portfolio
    // 3. else -> all non-retired securities without feed
    let sql = if let Some(pid) = portfolio_id {
        // Mode 1: Specific portfolio
        format!(
            "SELECT DISTINCT s.id, s.name, s.isin, s.ticker, s.currency
             FROM pp_security s
             JOIN pp_txn t ON t.security_id = s.id
             WHERE s.is_retired = 0
               AND {}
               AND t.owner_type = 'portfolio'
               AND t.owner_id = {}
               AND (
                   SELECT COALESCE(SUM(CASE
                       WHEN t2.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t2.shares
                       WHEN t2.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t2.shares
                       ELSE 0
                   END), 0)
                   FROM pp_txn t2
                   WHERE t2.security_id = s.id AND t2.owner_type = 'portfolio' AND t2.owner_id = {}
               ) > 0",
            missing_feed_condition,
            pid,
            pid
        )
    } else if held_only.unwrap_or(false) {
        // Mode 2: All held securities (across all portfolios)
        format!(
            "SELECT DISTINCT s.id, s.name, s.isin, s.ticker, s.currency
             FROM pp_security s
             WHERE s.is_retired = 0
               AND {}
               AND (
                   SELECT COALESCE(SUM(CASE
                       WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                       WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                       ELSE 0
                   END), 0)
                   FROM pp_txn t
                   WHERE t.security_id = s.id AND t.owner_type = 'portfolio'
               ) > 0",
            missing_feed_condition
        )
    } else {
        // Mode 3: All securities
        format!(
            "SELECT s.id, s.name, s.isin, s.ticker, s.currency
             FROM pp_security s
             WHERE s.is_retired = 0
               AND {}",
            missing_feed_condition
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let securities: Vec<SecurityForSuggestion> = stmt
        .query_map([], |row| {
            Ok(SecurityForSuggestion {
                id: row.get(0)?,
                name: row.get(1)?,
                isin: row.get(2)?,
                ticker: row.get(3)?,
                currency: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Generate suggestions for each security
    let suggestions: Vec<QuoteSuggestion> = securities
        .iter()
        .filter_map(|s| suggestion::suggest_quote_provider(s))
        .collect();

    log::info!(
        "Generated {} quote suggestions for {} securities without feed",
        suggestions.len(),
        securities.len()
    );

    Ok(suggestions)
}

/// Apply a quote provider suggestion to a security
#[command]
pub fn apply_quote_suggestion(
    security_id: i64,
    feed: String,
    feed_url: Option<String>,
    ticker: Option<String>,
) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    // Validate feed is a known provider
    let valid_feeds = ["YAHOO", "YAHOO-ADJUSTEDCLOSE", "COINGECKO", "KRAKEN", "FINNHUB", "ALPHAVANTAGE", "TWELVEDATA", "MANUAL"];
    if !valid_feeds.contains(&feed.as_str()) {
        return Err(format!("Unknown feed provider: {}", feed));
    }

    // Update security with suggested feed and optionally ticker
    if let Some(ref t) = ticker {
        conn.execute(
            "UPDATE pp_security SET feed = ?1, feed_url = ?2, ticker = ?3 WHERE id = ?4",
            params![feed, feed_url, t, security_id],
        )
        .map_err(|e| format!("Failed to update security: {}", e))?;
    } else {
        conn.execute(
            "UPDATE pp_security SET feed = ?1, feed_url = ?2 WHERE id = ?3",
            params![feed, feed_url, security_id],
        )
        .map_err(|e| format!("Failed to update security: {}", e))?;
    }

    log::info!(
        "Applied quote suggestion for security {}: feed={}, feed_url={:?}, ticker={:?}",
        security_id,
        feed,
        feed_url,
        ticker
    );

    Ok(())
}

/// Get count of securities without configured quote provider
#[command]
pub fn get_unconfigured_securities_count() -> Result<UnconfiguredSecuritiesInfo, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    // Condition checks for missing/invalid feed (NULL, empty, or not in valid list)
    let missing_feed_condition = "(feed IS NULL OR TRIM(feed) = '' OR UPPER(TRIM(feed)) NOT IN ('YAHOO', 'YAHOO-ADJUSTEDCLOSE', 'COINGECKO', 'KRAKEN', 'FINNHUB', 'ALPHAVANTAGE', 'TWELVEDATA', 'TRADINGVIEW'))";

    // Count all securities without valid auto-fetch feed
    let total_unconfigured: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM pp_security
                 WHERE is_retired = 0 AND {}",
                missing_feed_condition
            ),
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Count held securities without valid auto-fetch feed
    let missing_feed_condition_s = "(s.feed IS NULL OR TRIM(s.feed) = '' OR UPPER(TRIM(s.feed)) NOT IN ('YAHOO', 'YAHOO-ADJUSTEDCLOSE', 'COINGECKO', 'KRAKEN', 'FINNHUB', 'ALPHAVANTAGE', 'TWELVEDATA', 'TRADINGVIEW'))";
    let held_unconfigured: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(DISTINCT s.id) FROM pp_security s
                 WHERE s.is_retired = 0
                   AND {}
                   AND (
                       SELECT COALESCE(SUM(CASE
                           WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                           WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                           ELSE 0
                       END), 0)
                       FROM pp_txn t
                       WHERE t.security_id = s.id AND t.owner_type = 'portfolio'
                   ) > 0",
                missing_feed_condition_s
            ),
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(UnconfiguredSecuritiesInfo {
        total_unconfigured: total_unconfigured as usize,
        held_unconfigured: held_unconfigured as usize,
    })
}

/// Info about unconfigured securities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnconfiguredSecuritiesInfo {
    pub total_unconfigured: usize,
    pub held_unconfigured: usize,
}

// ============================================================================
// QUOTE CONFIGURATION AUDIT
// ============================================================================

/// Result of auditing a single security's quote configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteConfigAuditResult {
    pub security_id: i64,
    pub security_name: String,
    pub ticker: Option<String>,
    pub feed: String,
    /// Status: "ok", "stale", "missing", "config_error", "suspicious"
    pub status: String,
    pub last_price_date: Option<String>,
    pub days_since_last_price: Option<i64>,
    /// Error message when status is "config_error"
    pub error_message: Option<String>,
    /// Last known price from database
    pub last_known_price: Option<f64>,
    /// Price fetched during audit
    pub fetched_price: Option<f64>,
    /// Price deviation in percent (for "suspicious" status)
    pub price_deviation: Option<f64>,
}

/// Summary of audit results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteAuditSummary {
    pub total_audited: usize,
    pub ok_count: usize,
    pub stale_count: usize,         // > 7 days old
    pub missing_count: usize,       // no prices at all
    pub config_error_count: usize,  // fetch failed
    pub suspicious_count: usize,    // price deviation > 50%
    pub unconfigured_count: usize,  // no feed configured
    pub results: Vec<QuoteConfigAuditResult>,
}

/// Result of testing a quote fetch
struct QuoteFetchTestResult {
    success: bool,
    fetched_price: Option<f64>,
    error_message: Option<String>,
}

/// Check if price deviation is suspicious (> 50% change)
fn check_price_plausibility(last_known: f64, fetched: f64) -> Option<f64> {
    if last_known <= 0.0 || fetched <= 0.0 {
        return None;
    }
    let deviation = ((fetched - last_known) / last_known) * 100.0;
    if deviation.abs() > 50.0 {
        Some(deviation)
    } else {
        None
    }
}

/// Data loaded from DB for audit (before async operations)
#[derive(Clone)]
struct AuditSecurityData {
    security: SecurityInfo,
    last_price_date: Option<String>,
    last_known_price: Option<f64>,
    days_since: Option<i64>,
    feed_to_use: String,
    feed_url_to_use: Option<String>,
}

/// Load all data from DB synchronously (before async operations)
fn load_audit_data(only_held: bool) -> Result<Vec<AuditSecurityData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    // Get ALL securities (including those without configured feeds)
    // We'll determine their status later based on whether they have a valid feed
    // Use COALESCE to return empty string for NULL feeds
    let sql = if only_held {
        "SELECT s.id, s.name, COALESCE(s.feed, '') as feed, s.feed_url, s.latest_feed, s.latest_feed_url, s.ticker, s.isin, s.currency
         FROM pp_security s
         WHERE s.is_retired = 0
           AND (
               SELECT COALESCE(SUM(CASE
                   WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                   WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                   ELSE 0
               END), 0)
               FROM pp_txn t
               WHERE t.security_id = s.id AND t.owner_type = 'portfolio'
           ) > 0
         ORDER BY s.name".to_string()
    } else {
        "SELECT s.id, s.name, COALESCE(s.feed, '') as feed, s.feed_url, s.latest_feed, s.latest_feed_url, s.ticker, s.isin, s.currency
         FROM pp_security s
         WHERE s.is_retired = 0
         ORDER BY s.name".to_string()
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    // Load all security info
    let securities: Vec<SecurityInfo> = stmt
        .query_map([], |row| {
            Ok(SecurityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                feed: row.get(2)?,
                feed_url: row.get(3)?,
                latest_feed: row.get(4)?,
                latest_feed_url: row.get(5)?,
                ticker: row.get(6)?,
                isin: row.get(7)?,
                currency: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Load price info for each security
    let mut result = Vec::new();
    for security in securities {
        // Get last price info from database
        let price_info: Option<(String, f64, i64)> = conn
            .query_row(
                "SELECT date, value, (julianday('now') - julianday(date)) as days_ago
                 FROM pp_latest_price WHERE security_id = ?1",
                params![security.id],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?)),
            )
            .ok()
            .map(|(date, value, days)| (date, quotes::price_from_db(value), days as i64))
            .or_else(|| {
                conn.query_row(
                    "SELECT date, value, (julianday('now') - julianday(date)) as days_ago
                     FROM pp_price WHERE security_id = ?1 ORDER BY date DESC LIMIT 1",
                    params![security.id],
                    |row| Ok((row.get(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?)),
                )
                .ok()
                .map(|(date, value, days)| (date, quotes::price_from_db(value), days as i64))
            });

        let (last_price_date, last_known_price, days_since) = match &price_info {
            Some((date, price, days)) => (Some(date.clone()), Some(*price), Some(*days)),
            None => (None, None, None),
        };

        // Determine which feed to use (latest_feed if set, otherwise feed)
        let feed_to_use = security.latest_feed.as_ref()
            .filter(|f| !f.is_empty())
            .unwrap_or(&security.feed)
            .to_string();
        let feed_url_to_use = if security.latest_feed.as_ref().filter(|f| !f.is_empty()).is_some() {
            security.latest_feed_url.clone()
        } else {
            security.feed_url.clone()
        };

        result.push(AuditSecurityData {
            security,
            last_price_date,
            last_known_price,
            days_since,
            feed_to_use,
            feed_url_to_use,
        });
    }

    Ok(result)
}

/// Audit all configured quote sources - checks for missing, stale, or problematic prices
/// Check if a feed is a valid auto-fetch provider
fn is_valid_auto_fetch_feed(feed: &str) -> bool {
    let valid_feeds = ["YAHOO", "YAHOO-ADJUSTEDCLOSE", "COINGECKO", "KRAKEN", "FINNHUB", "ALPHAVANTAGE", "TWELVEDATA", "TRADINGVIEW"];
    let feed_upper = feed.trim().to_uppercase();
    valid_feeds.contains(&feed_upper.as_str())
}

/// Now performs actual quote fetches to verify configuration works and check plausibility
#[command]
pub async fn audit_quote_configurations(
    only_held: Option<bool>,
    api_keys: Option<ApiKeys>,
) -> Result<QuoteAuditSummary, String> {
    let only_held = only_held.unwrap_or(true);
    let keys = api_keys.unwrap_or_default();

    // Load all DB data synchronously first (releases DB connection before async)
    let audit_data = load_audit_data(only_held)?;

    let mut results = Vec::new();
    let mut ok_count = 0;
    let mut stale_count = 0;
    let mut missing_count = 0;
    let mut config_error_count = 0;
    let mut suspicious_count = 0;
    let mut unconfigured_count = 0;

    // Now do async operations (no DB connection held)
    for data in audit_data {
        // First check if feed is configured at all
        if !is_valid_auto_fetch_feed(&data.feed_to_use) {
            // No valid feed configured - mark as unconfigured (no fetch attempt)
            unconfigured_count += 1;
            results.push(QuoteConfigAuditResult {
                security_id: data.security.id,
                security_name: data.security.name.clone(),
                ticker: data.security.ticker.clone(),
                feed: data.feed_to_use.clone(),
                status: "unconfigured".to_string(),
                last_price_date: data.last_price_date.clone(),
                days_since_last_price: data.days_since,
                error_message: Some("Keine Kursquelle konfiguriert".to_string()),
                last_known_price: data.last_known_price,
                fetched_price: None,
                price_deviation: None,
            });
            continue;
        }

        // Try to fetch a quote to verify configuration works
        let fetch_result = test_quote_fetch(
            &data.security,
            &data.feed_to_use,
            data.feed_url_to_use.as_deref(),
            &keys
        ).await;

        // Determine status based on DB state and fetch result
        let (status, error_message, fetched_price, price_deviation) = if !fetch_result.success {
            // Configuration error: fetch failed
            config_error_count += 1;
            (
                "config_error".to_string(),
                fetch_result.error_message,
                None,
                None,
            )
        } else if let (Some(fetched), Some(last_known)) = (fetch_result.fetched_price, data.last_known_price) {
            // Check plausibility
            if let Some(deviation) = check_price_plausibility(last_known, fetched) {
                suspicious_count += 1;
                (
                    "suspicious".to_string(),
                    None,
                    Some(fetched),
                    Some(deviation),
                )
            } else if data.days_since.map(|d| d > 7).unwrap_or(false) {
                stale_count += 1;
                ("stale".to_string(), None, Some(fetched), None)
            } else {
                ok_count += 1;
                ("ok".to_string(), None, Some(fetched), None)
            }
        } else if data.last_known_price.is_none() {
            // No price in DB at all
            missing_count += 1;
            ("missing".to_string(), None, fetch_result.fetched_price, None)
        } else if data.days_since.map(|d| d > 7).unwrap_or(false) {
            stale_count += 1;
            ("stale".to_string(), None, fetch_result.fetched_price, None)
        } else {
            ok_count += 1;
            ("ok".to_string(), None, fetch_result.fetched_price, None)
        };

        // Only add to results if there's an issue
        if status != "ok" {
            results.push(QuoteConfigAuditResult {
                security_id: data.security.id,
                security_name: data.security.name,
                ticker: data.security.ticker,
                feed: data.feed_to_use,
                status,
                last_price_date: data.last_price_date,
                days_since_last_price: data.days_since,
                error_message,
                last_known_price: data.last_known_price,
                fetched_price,
                price_deviation,
            });
        }
    }

    // Sort: unconfigured first, then config_error, suspicious, missing, stale by days descending
    results.sort_by(|a, b| {
        let status_order = |s: &str| -> i32 {
            match s {
                "unconfigured" => 0,
                "config_error" => 1,
                "suspicious" => 2,
                "missing" => 3,
                "stale" => 4,
                _ => 5,
            }
        };
        let a_order = status_order(&a.status);
        let b_order = status_order(&b.status);
        if a_order != b_order {
            a_order.cmp(&b_order)
        } else {
            b.days_since_last_price.cmp(&a.days_since_last_price)
        }
    });

    let total = ok_count + stale_count + missing_count + config_error_count + suspicious_count + unconfigured_count;
    Ok(QuoteAuditSummary {
        total_audited: total,
        ok_count,
        stale_count,
        missing_count,
        config_error_count,
        suspicious_count,
        unconfigured_count,
        results,
    })
}

/// Test quote fetch for a single security without saving to database
async fn test_quote_fetch(
    security: &SecurityInfo,
    feed: &str,
    feed_url: Option<&str>,
    keys: &ApiKeys,
) -> QuoteFetchTestResult {
    let provider = match ProviderType::from_str(feed) {
        Some(p) => p,
        None => {
            return QuoteFetchTestResult {
                success: false,
                fetched_price: None,
                error_message: Some(format!("Unbekannter Provider: {}", feed)),
            };
        }
    };

    // Skip providers that require API key if not provided
    if provider == ProviderType::Finnhub && keys.finnhub.is_none() {
        return QuoteFetchTestResult {
            success: false,
            fetched_price: None,
            error_message: Some("Finnhub API-Key erforderlich".to_string()),
        };
    }
    if provider == ProviderType::AlphaVantage && keys.alpha_vantage.is_none() {
        return QuoteFetchTestResult {
            success: false,
            fetched_price: None,
            error_message: Some("Alpha Vantage API-Key erforderlich".to_string()),
        };
    }
    if provider == ProviderType::TwelveData && keys.twelve_data.is_none() {
        return QuoteFetchTestResult {
            success: false,
            fetched_price: None,
            error_message: Some("Twelve Data API-Key erforderlich".to_string()),
        };
    }

    // Get symbol for fetch
    let symbol = match security.ticker.clone().or(security.isin.clone()) {
        Some(s) => s,
        None => {
            return QuoteFetchTestResult {
                success: false,
                fetched_price: None,
                error_message: Some("Kein Ticker oder ISIN vorhanden".to_string()),
            };
        }
    };

    // Build request and fetch
    let api_key = match provider {
        ProviderType::Finnhub => keys.finnhub.clone(),
        ProviderType::AlphaVantage => keys.alpha_vantage.clone(),
        ProviderType::CoinGecko => keys.coingecko.clone(),
        ProviderType::TwelveData => keys.twelve_data.clone(),
        _ => None,
    };

    let request = quotes::SecurityQuoteRequest {
        id: security.id,
        symbol,
        provider,
        feed_url: feed_url.map(|s| s.to_string()),
        api_key,
        currency: security.currency.clone(),
    };

    let results = quotes::fetch_all_quotes(vec![request]).await;

    match results.first() {
        Some(r) if r.success && r.latest.is_some() => QuoteFetchTestResult {
            success: true,
            fetched_price: r.latest.as_ref().map(|l| l.quote.close),
            error_message: None,
        },
        Some(r) => QuoteFetchTestResult {
            success: false,
            fetched_price: None,
            error_message: r.error.clone().or(Some("Kursabruf fehlgeschlagen".to_string())),
        },
        None => QuoteFetchTestResult {
            success: false,
            fetched_price: None,
            error_message: Some("Keine Antwort vom Provider".to_string()),
        },
    }
}

// ============================================================================
// AUTO-FIX FOR BROKEN QUOTE CONFIGURATIONS
// ============================================================================

/// Known symbol mappings for common errors (extended)
fn get_known_symbol_fix(ticker: &str, provider: &str) -> Option<(&'static str, &'static str)> {
    // Returns (new_provider, new_symbol)
    match (provider.to_uppercase().as_str(), ticker.to_uppercase().as_str()) {
        // === COMMODITIES ===
        // Gold - TradingView doesn't work, use Yahoo Futures
        ("TRADINGVIEW", "XAUUSD") | ("TRADINGVIEW", "GOLD") => Some(("YAHOO", "GC=F")),
        (_, "GOLD") | (_, "XAUUSD") => Some(("YAHOO", "GC=F")),
        // Silver
        ("TRADINGVIEW", "XAGUSD") | ("TRADINGVIEW", "SILVER") => Some(("YAHOO", "SI=F")),
        (_, "SILVER") | (_, "XAGUSD") => Some(("YAHOO", "SI=F")),
        // Platinum
        (_, "XPTUSD") | (_, "PLATINUM") => Some(("YAHOO", "PL=F")),
        // Palladium
        (_, "XPDUSD") | (_, "PALLADIUM") => Some(("YAHOO", "PA=F")),
        // Oil
        (_, "CRUDE") | (_, "CRUDEOIL") | (_, "WTI") => Some(("YAHOO", "CL=F")),
        (_, "BRENT") => Some(("YAHOO", "BZ=F")),
        // Natural Gas
        (_, "NATGAS") | (_, "NG") => Some(("YAHOO", "NG=F")),

        // === INDICES ===
        (_, "SPX") | (_, "SP500") | (_, "S&P500") => Some(("YAHOO", "^GSPC")),
        (_, "NDX") | (_, "NASDAQ100") | (_, "NASDAQ-100") => Some(("YAHOO", "^NDX")),
        (_, "DJI") | (_, "DOWJONES") | (_, "DOW") => Some(("YAHOO", "^DJI")),
        (_, "DAX") | (_, "DAX40") | (_, "DAX30") => Some(("YAHOO", "^GDAXI")),
        (_, "STOXX50") | (_, "SX5E") | (_, "EUROSTOXX50") => Some(("YAHOO", "^STOXX50E")),
        (_, "FTSE") | (_, "FTSE100") | (_, "UKX") => Some(("YAHOO", "^FTSE")),
        (_, "SMI") | (_, "SMI20") => Some(("YAHOO", "^SSMI")),
        (_, "CAC") | (_, "CAC40") => Some(("YAHOO", "^FCHI")),
        (_, "NIKKEI") | (_, "N225") => Some(("YAHOO", "^N225")),
        (_, "HSI") | (_, "HANGSENG") => Some(("YAHOO", "^HSI")),
        (_, "VIX") => Some(("YAHOO", "^VIX")),

        // === COMMON SWISS TICKER CORRECTIONS ===
        (_, "NSN.SW") => Some(("YAHOO", "NESN.SW")),  // Nestle typo
        (_, "NESN") => Some(("YAHOO", "NESN.SW")),    // Nestle without suffix
        (_, "NOVN") => Some(("YAHOO", "NOVN.SW")),    // Novartis without suffix
        (_, "ROG") => Some(("YAHOO", "ROG.SW")),      // Roche without suffix
        (_, "UBSG") => Some(("YAHOO", "UBSG.SW")),    // UBS without suffix
        (_, "CSGN") => Some(("YAHOO", "CSGN.SW")),    // Credit Suisse (historical)
        (_, "ZURN") => Some(("YAHOO", "ZURN.SW")),    // Zurich Insurance
        (_, "ABBN") => Some(("YAHOO", "ABBN.SW")),    // ABB without suffix
        (_, "SREN") => Some(("YAHOO", "SREN.SW")),    // Swiss Re
        (_, "LONN") => Some(("YAHOO", "LONN.SW")),    // Lonza

        // === COMMON ETF CORRECTIONS ===
        (_, "VWRL") => Some(("YAHOO", "VWRL.L")),     // Vanguard FTSE All-World
        (_, "VWCE") => Some(("YAHOO", "VWCE.DE")),    // Vanguard FTSE All-World ACC
        (_, "IWDA") => Some(("YAHOO", "IWDA.AS")),    // iShares MSCI World
        (_, "EUNL") => Some(("YAHOO", "EUNL.DE")),    // iShares Core MSCI World
        (_, "CSPX") => Some(("YAHOO", "CSPX.L")),     // iShares S&P 500
        (_, "SXR8") => Some(("YAHOO", "SXR8.DE")),    // iShares S&P 500 EUR
        (_, "IUSA") => Some(("YAHOO", "IUSA.L")),     // iShares S&P 500 Dist

        // === COMMON GERMAN TICKER CORRECTIONS ===
        (_, "SAP") => Some(("YAHOO", "SAP.DE")),      // SAP without suffix
        (_, "ALV") => Some(("YAHOO", "ALV.DE")),      // Allianz without suffix
        (_, "SIE") => Some(("YAHOO", "SIE.DE")),      // Siemens without suffix
        (_, "BAS") => Some(("YAHOO", "BAS.DE")),      // BASF without suffix
        (_, "MUV2") => Some(("YAHOO", "MUV2.DE")),    // Munich Re without suffix
        (_, "DTE") => Some(("YAHOO", "DTE.DE")),      // Deutsche Telekom without suffix
        (_, "DBK") => Some(("YAHOO", "DBK.DE")),      // Deutsche Bank without suffix
        (_, "BMW") => Some(("YAHOO", "BMW.DE")),      // BMW without suffix

        _ => None,
    }
}

/// Get Yahoo exchange suffixes for a currency
fn get_suffixes_for_currency(currency: &str) -> Vec<&'static str> {
    match currency.to_uppercase().as_str() {
        "EUR" => vec![".DE", ".PA", ".AS", ".MI", ".MC"],  // DE, Paris, Amsterdam, Milan, Madrid
        "CHF" => vec![".SW"],
        "GBP" | "GBX" => vec![".L"],
        "SEK" => vec![".ST"],
        "NOK" => vec![".OL"],
        "DKK" => vec![".CO"],
        "PLN" => vec![".WA"],
        "HKD" => vec![".HK"],
        "JPY" => vec![".T"],
        "AUD" => vec![".AX"],
        "CAD" => vec![".TO"],
        "USD" => vec![""],  // US stocks don't need suffix
        _ => vec![],
    }
}

/// Unvalidated suggestion candidate
struct UnvalidatedSuggestion {
    provider: String,
    symbol: String,
    source: String,
}

/// Suggestion for fixing a broken quote configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteFixSuggestion {
    pub security_id: i64,
    pub current_provider: String,
    pub current_symbol: Option<String>,
    pub suggested_provider: String,
    pub suggested_symbol: String,
    pub suggested_feed_url: Option<String>,
    pub source: String, // "known_mapping", "isin_search", "suffix_variant", "yahoo_search", "tradingview_search"
    pub confidence: f64,
    /// Validated price from actual quote fetch (only set if validation succeeded)
    pub validated_price: Option<f64>,
}

/// Get fix suggestions for a broken security quote configuration
///
/// Uses multi-provider search with parallel validation to find working alternatives.
#[command]
pub async fn get_quote_fix_suggestions(
    security_id: i64,
    _api_keys: Option<ApiKeys>,
) -> Result<Vec<QuoteFixSuggestion>, String> {
    // Load security info from DB
    let security_info: Option<SecurityInfo> = {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

        conn.query_row(
            "SELECT id, name, COALESCE(NULLIF(feed, ''), 'YAHOO') as feed,
                    feed_url, latest_feed, latest_feed_url, ticker, isin, currency
             FROM pp_security WHERE id = ?1",
            params![security_id],
            |row| {
                Ok(SecurityInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    feed: row.get(2)?,
                    feed_url: row.get(3)?,
                    latest_feed: row.get(4)?,
                    latest_feed_url: row.get(5)?,
                    ticker: row.get(6)?,
                    isin: row.get(7)?,
                    currency: row.get(8)?,
                })
            },
        )
        .ok()
    };

    let Some(security) = security_info else {
        return Err(format!("Security {} nicht gefunden", security_id));
    };

    let current_provider = security.latest_feed.as_ref()
        .filter(|f| !f.is_empty())
        .unwrap_or(&security.feed)
        .clone();
    let current_symbol = security.ticker.clone();

    // 1. Collect all candidates
    let candidates = collect_fix_candidates(&security, &current_provider).await;

    // 2. Validate candidates in parallel (max 5 concurrent)
    let validated = validate_candidates(candidates, security_id, &current_provider, &current_symbol).await;

    Ok(validated)
}

/// Collect potential fix candidates from multiple sources
async fn collect_fix_candidates(
    security: &SecurityInfo,
    current_provider: &str,
) -> Vec<UnvalidatedSuggestion> {
    let mut candidates = Vec::new();

    // === Stage 1: Known mappings (highest priority) ===
    if let Some(ticker) = &security.ticker {
        if let Some((new_provider, new_symbol)) = get_known_symbol_fix(ticker, current_provider) {
            candidates.push(UnvalidatedSuggestion {
                provider: new_provider.to_string(),
                symbol: new_symbol.to_string(),
                source: "known_mapping".to_string(),
            });
        }
    }

    // === Stage 2: Intelligent suffix variants (for EUR/CHF/GBP etc.) ===
    if let Some(ticker) = &security.ticker {
        let base_ticker = ticker.split('.').next().unwrap_or(ticker);
        let currency = security.currency.as_deref().unwrap_or("USD");
        let suffixes = get_suffixes_for_currency(currency);

        for suffix in suffixes {
            let symbol = format!("{}{}", base_ticker, suffix);
            if !candidates.iter().any(|c| c.symbol == symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "YAHOO".to_string(),
                    symbol,
                    source: "suffix_variant".to_string(),
                });
            }
        }
    }

    // === Stage 3: Multi-provider search (parallel) ===
    let search_query = security.ticker.as_deref()
        .or(security.isin.as_deref())
        .unwrap_or(&security.name);

    let (yahoo_results, tv_results) = tokio::join!(
        yahoo::search(search_query),
        tradingview::search_symbols(search_query, 5)
    );

    // Yahoo search results
    if let Ok(results) = yahoo_results {
        for r in results.into_iter().take(3) {
            if !candidates.iter().any(|c| c.symbol == r.symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "YAHOO".to_string(),
                    symbol: r.symbol,
                    source: "yahoo_search".to_string(),
                });
            }
        }
    }

    // TradingView search results
    if let Ok(results) = tv_results {
        for r in results.into_iter().take(3) {
            if !candidates.iter().any(|c| c.symbol == r.symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "TRADINGVIEW".to_string(),
                    symbol: r.symbol,
                    source: "tradingview_search".to_string(),
                });
            }
        }
    }

    // === Stage 4: ISIN-based search (if available, high priority) ===
    if let Some(isin) = &security.isin {
        if let Ok(results) = yahoo::search(isin).await {
            for r in results.into_iter().take(2) {
                if !candidates.iter().any(|c| c.symbol == r.symbol) {
                    candidates.push(UnvalidatedSuggestion {
                        provider: "YAHOO".to_string(),
                        symbol: r.symbol,
                        source: "isin_search".to_string(),
                    });
                }
            }
        }
    }

    candidates
}

/// Validate candidates by actually fetching quotes (parallel, max 5 concurrent)
async fn validate_candidates(
    candidates: Vec<UnvalidatedSuggestion>,
    security_id: i64,
    current_provider: &str,
    current_symbol: &Option<String>,
) -> Vec<QuoteFixSuggestion> {
    let current_provider = current_provider.to_string();
    let current_symbol = current_symbol.clone();

    // Parallel validation with buffer_unordered (max 5 concurrent)
    let validated: Vec<Option<QuoteFixSuggestion>> = stream::iter(candidates)
        .map(|candidate| {
            let current_provider = current_provider.clone();
            let current_symbol = current_symbol.clone();
            async move {
                let fetch_result = match candidate.provider.as_str() {
                    "YAHOO" => yahoo::fetch_quote(&candidate.symbol, false).await,
                    "TRADINGVIEW" => tradingview::fetch_quote(&candidate.symbol).await,
                    _ => return None,
                };

                match fetch_result {
                    Ok(quote) if quote.quote.close > 0.0 => {
                        Some(QuoteFixSuggestion {
                            security_id,
                            current_provider: current_provider.clone(),
                            current_symbol: current_symbol.clone(),
                            suggested_provider: candidate.provider,
                            suggested_symbol: candidate.symbol,
                            suggested_feed_url: None,
                            source: candidate.source,
                            confidence: 1.0, // Validated = 100%
                            validated_price: Some(quote.quote.close),
                        })
                    }
                    _ => None,
                }
            }
        })
        .buffer_unordered(5)
        .collect()
        .await;

    // Filter out None values and sort by source priority
    let mut results: Vec<_> = validated.into_iter().flatten().collect();

    // Sort by: known_mapping > isin_search > suffix_variant > search
    results.sort_by(|a, b| {
        let priority = |s: &str| match s {
            "known_mapping" => 0,
            "isin_search" => 1,
            "suffix_variant" => 2,
            "yahoo_search" => 3,
            "tradingview_search" => 4,
            _ => 5,
        };
        priority(&a.source).cmp(&priority(&b.source))
    });

    // Limit to top 5 results
    results.truncate(5);
    results
}

/// Apply a fix suggestion to a security
#[command]
pub fn apply_quote_fix(
    security_id: i64,
    new_provider: String,
    new_symbol: String,
    new_feed_url: Option<String>,
) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE pp_security SET
            latest_feed = ?1,
            latest_feed_url = ?2,
            ticker = COALESCE(?3, ticker),
            updated_at = ?4
         WHERE id = ?5",
        params![new_provider, new_feed_url, new_symbol, now, security_id],
    )
    .map_err(|e| format!("Fehler beim Aktualisieren: {}", e))?;

    log::info!(
        "Applied quote fix for security {}: {} -> {} ({})",
        security_id,
        new_symbol,
        new_provider,
        new_feed_url.unwrap_or_default()
    );

    Ok(())
}

// ============================================================================
// UNIFIED QUOTE MANAGER - Combines suggest + audit + validate
// ============================================================================

/// A security that needs attention (no feed, broken feed, stale prices, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteManagerItem {
    pub security_id: i64,
    pub security_name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub currency: Option<String>,
    /// Current feed (if any)
    pub current_feed: Option<String>,
    pub current_feed_url: Option<String>,
    /// Problem status: "unconfigured", "error", "stale", "suspicious", "no_data"
    pub status: String,
    pub status_message: String,
    /// Last known price date
    pub last_price_date: Option<String>,
    /// Days since last price
    pub days_since_price: Option<i64>,
    /// Validated fix suggestions (already tested to work)
    pub suggestions: Vec<ValidatedSuggestion>,
}

/// A validated suggestion that has been tested and works
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedSuggestion {
    pub provider: String,
    pub symbol: String,
    pub feed_url: Option<String>,
    /// How the suggestion was found
    pub source: String,
    /// Validated price from actual fetch
    pub validated_price: f64,
}

/// Summary result from the unified quote manager
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteManagerResult {
    pub total_securities: usize,
    pub total_with_issues: usize,
    pub unconfigured_count: usize,
    pub error_count: usize,
    pub stale_count: usize,
    pub no_data_count: usize,
    /// All securities with issues and their validated suggestions
    pub items: Vec<QuoteManagerItem>,
}

/// Unified quote manager - finds all problematic securities and provides validated fix suggestions
///
/// This combines the functionality of:
/// - suggest_quote_providers (find securities without feed)
/// - audit_quote_configurations (check if feeds work)
/// - get_quote_fix_suggestions (find alternatives)
///
/// All in one call with pre-validated suggestions.
#[command]
pub async fn quote_manager_audit(
    only_held: Option<bool>,
    _api_keys: Option<ApiKeys>,
) -> Result<QuoteManagerResult, String> {
    let only_held = only_held.unwrap_or(true);

    // Step 1: Load all securities with their price info
    let audit_data = load_audit_data(only_held)?;
    let total_securities = audit_data.len();

    // Step 2: Find all securities with issues and collect them for processing
    let mut items_to_process = Vec::new();
    let mut unconfigured_count = 0;
    let mut error_count = 0;
    let mut stale_count = 0;
    let mut no_data_count = 0;

    for data in &audit_data {
        let (status, status_message) = determine_security_status(&data).await;

        match status.as_str() {
            "ok" => continue, // Skip securities that are fine
            "unconfigured" => unconfigured_count += 1,
            "error" => error_count += 1,
            "stale" => stale_count += 1,
            "no_data" => no_data_count += 1,
            _ => {}
        }

        items_to_process.push((data.clone(), status, status_message));
    }

    // Step 3: For each problematic security, find and validate suggestions (parallel)
    let items: Vec<QuoteManagerItem> = stream::iter(items_to_process)
        .map(|(data, status, status_message)| {
            // Clone what we need for the async block
            let security = data.security.clone();
            let feed_to_use = data.feed_to_use.clone();
            let feed_url_to_use = data.feed_url_to_use.clone();
            let last_price_date = data.last_price_date.clone();
            let days_since = data.days_since;

            async move {
                let suggestions = find_validated_suggestions_for_security(&security).await;

                QuoteManagerItem {
                    security_id: security.id,
                    security_name: security.name.clone(),
                    isin: security.isin.clone(),
                    ticker: security.ticker.clone(),
                    currency: security.currency.clone(),
                    current_feed: if feed_to_use.is_empty() { None } else { Some(feed_to_use) },
                    current_feed_url: feed_url_to_use,
                    status,
                    status_message,
                    last_price_date,
                    days_since_price: days_since,
                    suggestions,
                }
            }
        })
        .buffer_unordered(3) // Process 3 securities in parallel
        .collect()
        .await;

    // Sort: unconfigured first, then by name
    let mut items = items;
    items.sort_by(|a, b| {
        let status_priority = |s: &str| match s {
            "unconfigured" => 0,
            "error" => 1,
            "no_data" => 2,
            "stale" => 3,
            _ => 4,
        };
        status_priority(&a.status)
            .cmp(&status_priority(&b.status))
            .then_with(|| a.security_name.cmp(&b.security_name))
    });

    Ok(QuoteManagerResult {
        total_securities,
        total_with_issues: items.len(),
        unconfigured_count,
        error_count,
        stale_count,
        no_data_count,
        items,
    })
}

/// Determine the status of a security
async fn determine_security_status(data: &AuditSecurityData) -> (String, String) {
    // Check if feed is configured
    if !is_valid_auto_fetch_feed(&data.feed_to_use) {
        return ("unconfigured".to_string(), "Keine Kursquelle konfiguriert".to_string());
    }

    // Check if we have price data
    match data.days_since {
        None => {
            // No prices at all - try to fetch to see if config works
            let test = test_simple_fetch(&data.security, &data.feed_to_use, data.feed_url_to_use.as_deref()).await;
            if test {
                ("no_data".to_string(), "Konfiguration OK, aber keine Kursdaten vorhanden".to_string())
            } else {
                ("error".to_string(), "Kursabruf fehlgeschlagen - Konfiguration prüfen".to_string())
            }
        }
        Some(days) if days > 14 => {
            // Stale data - try to fetch to see if config still works
            let test = test_simple_fetch(&data.security, &data.feed_to_use, data.feed_url_to_use.as_deref()).await;
            if test {
                ("stale".to_string(), format!("Kurse {} Tage alt - Abruf funktioniert", days))
            } else {
                ("error".to_string(), format!("Kurse {} Tage alt - Abruf fehlgeschlagen", days))
            }
        }
        _ => ("ok".to_string(), "OK".to_string()),
    }
}

/// Simple fetch test - just checks if we can get a price
async fn test_simple_fetch(security: &SecurityInfo, feed: &str, feed_url: Option<&str>) -> bool {
    let provider = match ProviderType::from_str(feed) {
        Some(p) => p,
        None => return false,
    };

    let symbol = match security.ticker.clone().or(security.isin.clone()) {
        Some(s) => s,
        None => return false,
    };

    let request = quotes::SecurityQuoteRequest {
        id: security.id,
        symbol,
        provider,
        feed_url: feed_url.map(|s| s.to_string()),
        api_key: None,
        currency: security.currency.clone(),
    };

    let results = quotes::fetch_all_quotes(vec![request]).await;
    results.first().map(|r| r.success && r.latest.is_some()).unwrap_or(false)
}

/// Find and validate suggestions for a security
async fn find_validated_suggestions_for_security(security: &SecurityInfo) -> Vec<ValidatedSuggestion> {
    let mut candidates = Vec::new();

    // === Stage 1: Known mappings ===
    if let Some(ticker) = &security.ticker {
        if let Some((provider, symbol)) = get_known_symbol_fix(ticker, "YAHOO") {
            candidates.push(UnvalidatedSuggestion {
                provider: provider.to_string(),
                symbol: symbol.to_string(),
                source: "known_mapping".to_string(),
            });
        }
    }

    // === Stage 2: ISIN-based search (high priority) ===
    if let Some(isin) = &security.isin {
        if let Ok(results) = yahoo::search(isin).await {
            for r in results.into_iter().take(3) {
                if !candidates.iter().any(|c| c.symbol == r.symbol) {
                    candidates.push(UnvalidatedSuggestion {
                        provider: "YAHOO".to_string(),
                        symbol: r.symbol,
                        source: "isin_search".to_string(),
                    });
                }
            }
        }
    }

    // === Stage 3: Suffix variants based on currency ===
    if let Some(ticker) = &security.ticker {
        let base_ticker = ticker.split('.').next().unwrap_or(ticker);
        let currency = security.currency.as_deref().unwrap_or("USD");
        let suffixes = get_suffixes_for_currency(currency);

        for suffix in suffixes {
            let symbol = format!("{}{}", base_ticker, suffix);
            if !candidates.iter().any(|c| c.symbol == symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "YAHOO".to_string(),
                    symbol,
                    source: "suffix_variant".to_string(),
                });
            }
        }
    }

    // === Stage 4: Name/ticker search ===
    let search_query = security.ticker.as_deref()
        .or(security.name.split_whitespace().next())
        .unwrap_or(&security.name);

    if let Ok(results) = yahoo::search(search_query).await {
        for r in results.into_iter().take(3) {
            if !candidates.iter().any(|c| c.symbol == r.symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "YAHOO".to_string(),
                    symbol: r.symbol,
                    source: "name_search".to_string(),
                });
            }
        }
    }

    // === Stage 5: TradingView search ===
    if let Ok(results) = tradingview::search_symbols(search_query, 3).await {
        for r in results {
            if !candidates.iter().any(|c| c.symbol == r.symbol) {
                candidates.push(UnvalidatedSuggestion {
                    provider: "TRADINGVIEW".to_string(),
                    symbol: r.symbol,
                    source: "tradingview_search".to_string(),
                });
            }
        }
    }

    // === Validate candidates in parallel ===
    let validated: Vec<Option<ValidatedSuggestion>> = stream::iter(candidates)
        .map(|candidate| async move {
            let fetch_result = match candidate.provider.as_str() {
                "YAHOO" => yahoo::fetch_quote(&candidate.symbol, false).await,
                "TRADINGVIEW" => tradingview::fetch_quote(&candidate.symbol).await,
                _ => return None,
            };

            match fetch_result {
                Ok(quote) if quote.quote.close > 0.0 => {
                    Some(ValidatedSuggestion {
                        provider: candidate.provider,
                        symbol: candidate.symbol,
                        feed_url: None,
                        source: candidate.source,
                        validated_price: quote.quote.close,
                    })
                }
                _ => None,
            }
        })
        .buffer_unordered(5)
        .collect()
        .await;

    // Filter and sort by priority
    let mut results: Vec<_> = validated.into_iter().flatten().collect();
    results.sort_by(|a, b| {
        let priority = |s: &str| match s {
            "known_mapping" => 0,
            "isin_search" => 1,
            "suffix_variant" => 2,
            "name_search" => 3,
            "tradingview_search" => 4,
            _ => 5,
        };
        priority(&a.source).cmp(&priority(&b.source))
    });

    // Return top 3
    results.truncate(3);
    results
}

/// Apply a suggestion from the quote manager
#[command]
pub fn apply_quote_manager_suggestion(
    security_id: i64,
    provider: String,
    symbol: String,
    feed_url: Option<String>,
) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let now = chrono::Utc::now().to_rfc3339();

    // Update both feed and ticker
    conn.execute(
        "UPDATE pp_security SET
            feed = ?1,
            feed_url = ?2,
            ticker = ?3,
            latest_feed = ?1,
            latest_feed_url = ?2,
            updated_at = ?4
         WHERE id = ?5",
        params![provider, feed_url, symbol, now, security_id],
    )
    .map_err(|e| format!("Fehler beim Aktualisieren: {}", e))?;

    log::info!(
        "Applied quote manager suggestion for security {}: {} @ {}",
        security_id,
        symbol,
        provider
    );

    Ok(())
}

// ============== AI Quote Assistant ==============

use crate::ai::types::{
    QuoteAssistantRequest, QuoteAssistantResponse, ProblematicSecurity,
};
use crate::ai::prompts::{build_quote_assistant_system_prompt, build_quote_assistant_user_message};
use crate::quotes::assistant::{parse_ai_suggestion, validate_suggestion, get_problematic_securities, apply_suggestion};

/// Get securities with quote problems (no provider, fetch error, or stale)
#[command]
pub fn get_quote_problem_securities(
    stale_days: Option<i32>,
) -> Result<Vec<ProblematicSecurity>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    get_problematic_securities(conn, stale_days.unwrap_or(7))
        .map_err(|e| e.to_string())
}

/// Chat with the AI quote assistant to find optimal quote sources
#[command]
pub async fn chat_with_quote_assistant(
    request: QuoteAssistantRequest,
) -> Result<QuoteAssistantResponse, String> {
    // Build the full prompt with system instructions and context
    let system_prompt = build_quote_assistant_system_prompt();
    let user_message = if let Some(ref msg) = request.user_message {
        msg.clone()
    } else {
        build_quote_assistant_user_message(
            &request.security_context.security_name,
            request.security_context.isin.as_deref(),
            request.security_context.ticker.as_deref(),
            &request.security_context.currency,
            request.security_context.current_feed.as_deref(),
            request.security_context.current_feed_url.as_deref(),
            &request.security_context.problem,
            request.security_context.last_error.as_deref(),
        )
    };

    // Build conversation history as text
    let mut conversation = String::new();
    for msg in &request.history {
        let role = if msg.role == "user" { "User" } else { "Assistant" };
        conversation.push_str(&format!("\n{}: {}\n", role, msg.content));
    }
    conversation.push_str(&format!("\nUser: {}\n\nAssistant:", user_message));

    // Combine system prompt with conversation
    let full_prompt = format!(
        "{}\n\n---\n\n## Konversation\n{}",
        system_prompt, conversation
    );

    // Call the AI using simple completion
    let response_text = match request.provider.as_str() {
        "claude" => {
            crate::ai::claude::complete_text(&request.model, &request.api_key, &full_prompt)
                .await
                .map_err(|e| format!("Claude error: {}", e.message))?
        }
        "openai" => {
            crate::ai::openai::complete_text(&request.model, &request.api_key, &full_prompt)
                .await
                .map_err(|e| format!("OpenAI error: {}", e.message))?
        }
        "gemini" => {
            crate::ai::gemini::complete_text(&request.model, &request.api_key, &full_prompt)
                .await
                .map_err(|e| format!("Gemini error: {}", e.message))?
        }
        "perplexity" => {
            crate::ai::perplexity::complete_text(&request.model, &request.api_key, &full_prompt)
                .await
                .map_err(|e| format!("Perplexity error: {}", e.message))?
        }
        _ => {
            return Err(format!("Unknown AI provider: {}", request.provider));
        }
    };

    // Try to parse suggestion from response
    let suggestion = match parse_ai_suggestion(&response_text) {
        Ok(parsed) => {
            log::info!("Parsed AI suggestion: {:?}", parsed);
            // Validate by fetching a test quote
            let api_keys = ApiKeys {
                finnhub: None,
                alpha_vantage: None,
                coingecko: None,
                twelve_data: None,
            };
            let validated = validate_suggestion(&parsed, Some(&api_keys)).await;
            Some(validated)
        }
        Err(e) => {
            log::warn!("Could not parse AI suggestion: {}", e);
            None
        }
    };

    Ok(QuoteAssistantResponse {
        message: response_text,
        suggestion,
        tokens_used: None, // Simple completion doesn't return token usage
    })
}

/// Apply a validated AI quote suggestion to a security
#[command]
pub fn apply_quote_assistant_suggestion(
    security_id: i64,
    provider: String,
    ticker: String,
    feed_url: Option<String>,
) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("DB not initialized")?;

    let suggestion = crate::ai::types::AiQuoteSuggestion {
        provider,
        ticker,
        feed_url,
        confidence: 1.0,
        reason: "Applied from AI assistant".to_string(),
    };

    apply_suggestion(conn, security_id, &suggestion)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_symbol_fix() {
        // Commodities
        assert_eq!(
            get_known_symbol_fix("XAUUSD", "TRADINGVIEW"),
            Some(("YAHOO", "GC=F"))
        );
        assert_eq!(
            get_known_symbol_fix("GOLD", "YAHOO"),
            Some(("YAHOO", "GC=F"))
        );
        assert_eq!(
            get_known_symbol_fix("BRENT", "YAHOO"),
            Some(("YAHOO", "BZ=F"))
        );

        // Indices
        assert_eq!(
            get_known_symbol_fix("DAX", "TRADINGVIEW"),
            Some(("YAHOO", "^GDAXI"))
        );
        assert_eq!(
            get_known_symbol_fix("SPX", "YAHOO"),
            Some(("YAHOO", "^GSPC"))
        );

        // Swiss tickers
        assert_eq!(
            get_known_symbol_fix("NSN.SW", "YAHOO"),
            Some(("YAHOO", "NESN.SW"))
        );
        assert_eq!(
            get_known_symbol_fix("NESN", "YAHOO"),
            Some(("YAHOO", "NESN.SW"))
        );
        assert_eq!(
            get_known_symbol_fix("UBSG", "YAHOO"),
            Some(("YAHOO", "UBSG.SW"))
        );

        // German tickers
        assert_eq!(
            get_known_symbol_fix("SAP", "YAHOO"),
            Some(("YAHOO", "SAP.DE"))
        );

        // ETFs
        assert_eq!(
            get_known_symbol_fix("VWRL", "YAHOO"),
            Some(("YAHOO", "VWRL.L"))
        );
        assert_eq!(
            get_known_symbol_fix("IWDA", "YAHOO"),
            Some(("YAHOO", "IWDA.AS"))
        );

        // Unknown ticker
        assert_eq!(get_known_symbol_fix("AAPL", "YAHOO"), None);
    }

    #[test]
    fn test_get_suffixes_for_currency() {
        assert_eq!(get_suffixes_for_currency("EUR"), vec![".DE", ".PA", ".AS", ".MI", ".MC"]);
        assert_eq!(get_suffixes_for_currency("CHF"), vec![".SW"]);
        assert_eq!(get_suffixes_for_currency("GBP"), vec![".L"]);
        assert_eq!(get_suffixes_for_currency("USD"), vec![""]);
        assert!(get_suffixes_for_currency("XYZ").is_empty());
    }

    #[test]
    fn test_match_split_ratio_4_to_1() {
        // Apple 4:1 split: price drops to ~25%
        let result = match_split_ratio(0.25, 0.5);
        assert!(result.is_some());
        let (num, denom, _conf) = result.unwrap();
        assert_eq!(num, 4);
        assert_eq!(denom, 1);
    }

    #[test]
    fn test_match_split_ratio_5_to_1() {
        // Tesla 5:1 split: price drops to ~20%
        let result = match_split_ratio(0.20, 0.5);
        assert!(result.is_some());
        let (num, denom, _conf) = result.unwrap();
        assert_eq!(num, 5);
        assert_eq!(denom, 1);
    }

    #[test]
    fn test_match_split_ratio_10_to_1() {
        // Nvidia 10:1 split: price drops to ~10%
        let result = match_split_ratio(0.10, 0.5);
        assert!(result.is_some());
        let (num, denom, _conf) = result.unwrap();
        assert_eq!(num, 10);
        assert_eq!(denom, 1);
    }

    #[test]
    fn test_match_split_ratio_20_to_1() {
        // Amazon 20:1 split: price drops to ~5%
        let result = match_split_ratio(0.05, 0.5);
        assert!(result.is_some());
        let (num, denom, _conf) = result.unwrap();
        assert_eq!(num, 20);
        assert_eq!(denom, 1);
    }

    #[test]
    fn test_no_match_for_normal_price_change() {
        // Normal 15% drop is not a split
        let result = match_split_ratio(0.85, 0.5);
        assert!(result.is_none());

        // Normal 30% drop could be market crash, not 3:1 split (outside range)
        let result = match_split_ratio(0.70, 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn test_confidence_calculation() {
        // Exact 4:1 ratio (0.25) should have high confidence
        let result = match_split_ratio(0.25, 0.5);
        let (_, _, conf) = result.unwrap();
        assert!(conf > 0.7);

        // Slightly off (0.24) should still match but confidence is capped at 0.85
        let result = match_split_ratio(0.24, 0.5);
        let (_, _, conf) = result.unwrap();
        assert!(conf >= 0.5);
        assert!(conf <= 0.85);
    }
}
