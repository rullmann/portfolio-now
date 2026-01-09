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
                symbol: s.ticker.or(s.isin).or(Some(s.name))?,
                provider,
                feed_url: feed_url_to_use,
                api_key,
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
}

// ============== Datenbank-Funktionen ==============

fn get_securities_for_sync(ids: Vec<i64>) -> anyhow::Result<Vec<SecurityInfo>> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("DB not initialized"))?;

    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, name, COALESCE(feed, 'YAHOO') as feed, feed_url, latest_feed, latest_feed_url, ticker, isin
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
                s.feed_url, s.latest_feed, s.latest_feed_url, s.ticker, s.isin
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
                feed_url, latest_feed, latest_feed_url, ticker, isin
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
        "SELECT id, name, COALESCE(feed, 'YAHOO') as feed, feed_url, latest_feed, latest_feed_url, ticker, isin
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
