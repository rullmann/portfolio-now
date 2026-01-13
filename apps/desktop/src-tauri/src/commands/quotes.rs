//! Quote Commands - Tauri IPC für Kursabfragen
//!
//! Commands:
//! - fetch_quotes: Aktuelle Kurse abrufen
//! - fetch_historical_prices: Historische Kurse abrufen
//! - sync_security_prices: Kurse in DB speichern
//! - sync_all_prices: Alle Securities aktualisieren
//! - fetch_exchange_rates: EZB Wechselkurse abrufen

use crate::db;
use crate::quotes::{self, alphavantage, ecb, yahoo, ExchangeRate, LatestQuote, ProviderType, Quote, QuoteResult};
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
        QuoteProvider {
            id: "PP".to_string(),
            name: "Portfolio Report".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
