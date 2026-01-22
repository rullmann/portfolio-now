//! Validation Engine
//!
//! Core logic for validating and correcting quote source configurations.

use super::ai_fallback::get_ai_suggestion;
use super::providers::{get_tradingview_exchange_prefix, get_yahoo_exchange_suffix, search_all_providers};
use super::types::*;
use crate::db::get_connection;
use crate::quotes::{self, ProviderType, SecurityQuoteRequest};
use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::params;

/// Validate a single security's quote configuration
///
/// Workflow:
/// 1. First test if CURRENT configuration works
/// 2. If current config works, mark as validated
/// 3. If not, search provider APIs for better configuration
/// 4. Return result (optionally with AI suggestion if enabled)
pub async fn validate_security(
    security: SecurityForValidation,
    api_keys: &ApiKeys,
    ai_config: Option<&AiConfig>,
    force: bool,
) -> ValidationResult {
    let security_id = security.id;
    let security_name = security.name.clone();
    let isin = security.isin.clone();
    let original_feed = security.feed.clone();
    let original_ticker = security.ticker.clone();

    // Skip retired securities
    if security.is_retired {
        return ValidationResult {
            security_id,
            security_name,
            isin,
            original_feed,
            original_ticker,
            status: ValidationStatus::Skipped,
            validated_config: None,
            ai_suggestion: None,
            provider_results: vec![],
            confidence: 0.0,
            error: Some("Security is retired".to_string()),
        };
    }

    // Skip manual feed
    if security.feed.as_deref() == Some("MANUAL") {
        return ValidationResult {
            security_id,
            security_name,
            isin,
            original_feed,
            original_ticker,
            status: ValidationStatus::Skipped,
            validated_config: None,
            ai_suggestion: None,
            provider_results: vec![],
            confidence: 0.0,
            error: Some("Manual feed - skipped".to_string()),
        };
    }

    // FIRST: Test if current configuration works
    let current_config_works = test_current_configuration(&security, api_keys).await;

    if current_config_works && !force {
        // Check cache for recent validation
        if let Some(cached) = get_cached_mapping(security_id) {
            if let Some(ref last_validated) = cached.last_validated_at {
                if let Ok(last_date) = chrono::NaiveDateTime::parse_from_str(last_validated, "%Y-%m-%d %H:%M:%S") {
                    let days_since = (Utc::now().naive_utc() - last_date).num_days();
                    if days_since < 30 && cached.validation_status == ValidationStatus::Validated {
                        return ValidationResult {
                            security_id,
                            security_name,
                            isin,
                            original_feed,
                            original_ticker,
                            status: cached.validation_status,
                            validated_config: Some(ValidatedConfig {
                                feed: cached.validated_feed,
                                feed_url: cached.validated_feed_url,
                                ticker: cached.validated_ticker,
                                exchange: cached.validated_exchange,
                            }),
                            ai_suggestion: None,
                            provider_results: vec![],
                            confidence: cached.confidence,
                            error: None,
                        };
                    }
                }
            }
        }

        // Current config works but not cached - save it
        let config = ValidatedConfig {
            feed: original_feed.clone().unwrap_or_default(),
            feed_url: security.feed_url.clone(),
            ticker: original_ticker.clone(),
            exchange: None,
        };
        let _ = save_mapping(
            security_id,
            &config,
            &[],
            ValidationStatus::Validated,
            1.0,
            ValidationMethod::Code,
            None,
        );

        return ValidationResult {
            security_id,
            security_name,
            isin,
            original_feed,
            original_ticker,
            status: ValidationStatus::Validated,
            validated_config: Some(config),
            ai_suggestion: None,
            provider_results: vec![],
            confidence: 1.0,
            error: None,
        };
    }

    // Current config DOES NOT work - need to search for better configuration
    log::info!("Security '{}' (ID {}) - current config does not work, searching providers...",
               security_name, security_id);

    // Search all providers
    let provider_results = search_all_providers(&security, api_keys).await;

    // Try to verify the best matches
    let mut best_config: Option<ValidatedConfig> = None;
    let mut best_confidence = 0.0;

    for result in provider_results.iter().take(5) {
        // Skip low confidence results
        if result.confidence < 0.5 {
            continue;
        }

        // Try to verify quote fetch works
        if let Some(config) = try_verify_quote(&security, result, api_keys).await {
            if result.confidence > best_confidence {
                best_config = Some(config);
                best_confidence = result.confidence;
                break; // First verified match is usually the best
            }
        }
    }

    // If we found a working config, save and return
    if let Some(config) = best_config {
        let mapping = save_mapping(
            security_id,
            &config,
            &provider_results,
            ValidationStatus::Validated,
            best_confidence,
            ValidationMethod::Code,
            None,
        );

        if let Err(e) = mapping {
            log::error!("Failed to save mapping for security {}: {}", security_id, e);
        }

        return ValidationResult {
            security_id,
            security_name,
            isin,
            original_feed,
            original_ticker,
            status: ValidationStatus::Validated,
            validated_config: Some(config),
            ai_suggestion: None,
            provider_results,
            confidence: best_confidence,
            error: None,
        };
    }

    // Try AI fallback if enabled and configured
    if let Some(ai_cfg) = ai_config {
        if ai_cfg.enabled {
            match get_ai_suggestion(&security, &provider_results, ai_cfg).await {
                Ok(suggestion) => {
                    // Verify AI suggestion works
                    let ai_result = ProviderSearchResult {
                        provider: suggestion.feed.clone(),
                        symbol: suggestion.ticker.clone(),
                        name: None,
                        exchange: suggestion.feed_url.clone(),
                        security_type: None,
                        currency: None,
                        isin: None,
                        confidence: suggestion.confidence,
                    };

                    if let Some(config) = try_verify_quote(&security, &ai_result, api_keys).await {
                        // AI suggestion works!
                        let _ = save_mapping(
                            security_id,
                            &config,
                            &provider_results,
                            ValidationStatus::AiSuggested,
                            suggestion.confidence,
                            ValidationMethod::Ai,
                            Some(&suggestion),
                        );

                        return ValidationResult {
                            security_id,
                            security_name,
                            isin,
                            original_feed,
                            original_ticker,
                            status: ValidationStatus::AiSuggested,
                            validated_config: Some(config),
                            ai_suggestion: Some(suggestion),
                            provider_results,
                            confidence: best_confidence,
                            error: None,
                        };
                    } else {
                        // AI suggestion didn't verify, still return it for user review
                        let _ = save_mapping(
                            security_id,
                            &ValidatedConfig {
                                feed: suggestion.feed.clone(),
                                feed_url: suggestion.feed_url.clone(),
                                ticker: Some(suggestion.ticker.clone()),
                                exchange: None,
                            },
                            &provider_results,
                            ValidationStatus::AiSuggested,
                            suggestion.confidence,
                            ValidationMethod::Ai,
                            Some(&suggestion),
                        );

                        return ValidationResult {
                            security_id,
                            security_name,
                            isin,
                            original_feed,
                            original_ticker,
                            status: ValidationStatus::AiSuggested,
                            validated_config: None,
                            ai_suggestion: Some(suggestion),
                            provider_results,
                            confidence: 0.0,
                            error: Some("AI suggestion could not be verified".to_string()),
                        };
                    }
                }
                Err(e) => {
                    log::warn!("AI suggestion failed for {}: {}", security_name, e);
                }
            }
        }
    }

    // Validation failed
    let _ = save_mapping(
        security_id,
        &ValidatedConfig {
            feed: original_feed.clone().unwrap_or_default(),
            feed_url: security.feed_url.clone(),
            ticker: original_ticker.clone(),
            exchange: None,
        },
        &provider_results,
        ValidationStatus::Failed,
        0.0,
        ValidationMethod::Code,
        None,
    );

    ValidationResult {
        security_id,
        security_name,
        isin,
        original_feed,
        original_ticker,
        status: ValidationStatus::Failed,
        validated_config: None,
        ai_suggestion: None,
        provider_results,
        confidence: 0.0,
        error: Some("Could not find valid quote source configuration".to_string()),
    }
}

/// Test if the CURRENT configuration of a security works
async fn test_current_configuration(
    security: &SecurityForValidation,
    api_keys: &ApiKeys,
) -> bool {
    // Skip if no feed is configured
    let feed = match &security.feed {
        Some(f) if !f.is_empty() => f,
        _ => return false,
    };

    // Get provider type
    let provider_type = match ProviderType::from_str(feed) {
        Some(p) => p,
        None => {
            log::warn!("Unknown feed type '{}' for security '{}'", feed, security.name);
            return false;
        }
    };

    // Build the request with current configuration
    let symbol = security.ticker.clone().unwrap_or_default();
    if symbol.is_empty() {
        log::warn!("No ticker configured for security '{}'", security.name);
        return false;
    }

    let request = SecurityQuoteRequest {
        id: security.id,
        symbol: symbol.clone(),
        provider: provider_type,
        feed_url: security.feed_url.clone(),
        api_key: match provider_type {
            ProviderType::CoinGecko => api_keys.coingecko_api_key.clone(),
            ProviderType::Finnhub => api_keys.finnhub_api_key.clone(),
            ProviderType::AlphaVantage => api_keys.alpha_vantage_api_key.clone(),
            ProviderType::TwelveData => api_keys.twelve_data_api_key.clone(),
            _ => None,
        },
        currency: Some(security.currency.clone()),
    };

    log::debug!("Testing current config for '{}': provider={:?}, symbol={}, feed_url={:?}",
               security.name, provider_type, symbol, security.feed_url);

    // Try to fetch a quote
    let results = quotes::fetch_all_quotes(vec![request]).await;

    if let Some(result) = results.first() {
        if result.success && result.latest.is_some() {
            log::debug!("Current config works for '{}': got price {:?}",
                       security.name, result.latest);
            return true;
        } else {
            log::debug!("Current config FAILED for '{}': success={}, has_latest={}",
                       security.name, result.success, result.latest.is_some());
        }
    } else {
        log::debug!("No result returned for '{}'", security.name);
    }

    false
}

/// Try to verify a quote fetch works for a given result
async fn try_verify_quote(
    security: &SecurityForValidation,
    result: &ProviderSearchResult,
    api_keys: &ApiKeys,
) -> Option<ValidatedConfig> {
    let provider_type = ProviderType::from_str(&result.provider)?;

    // Build the quote request based on provider
    let (symbol, feed_url) = match provider_type {
        ProviderType::Yahoo | ProviderType::YahooAdjustedClose => {
            let suffix = get_yahoo_exchange_suffix(&security.currency, result.exchange.as_deref());
            let symbol = if let Some(ref s) = suffix {
                if !result.symbol.contains('.') {
                    format!("{}{}", result.symbol, s)
                } else {
                    result.symbol.clone()
                }
            } else {
                result.symbol.clone()
            };
            (symbol, suffix)
        }
        ProviderType::TradingView => {
            let prefix = get_tradingview_exchange_prefix(result.exchange.as_deref());
            let symbol = result.symbol.clone();
            (symbol, prefix)
        }
        ProviderType::CoinGecko => {
            (result.symbol.clone(), Some(security.currency.clone()))
        }
        _ => (result.symbol.clone(), None),
    };

    let request = SecurityQuoteRequest {
        id: security.id,
        symbol: symbol.clone(),
        provider: provider_type,
        feed_url: feed_url.clone(),
        api_key: match provider_type {
            ProviderType::CoinGecko => api_keys.coingecko_api_key.clone(),
            ProviderType::Finnhub => api_keys.finnhub_api_key.clone(),
            ProviderType::AlphaVantage => api_keys.alpha_vantage_api_key.clone(),
            ProviderType::TwelveData => api_keys.twelve_data_api_key.clone(),
            _ => None,
        },
        currency: Some(security.currency.clone()),
    };

    // Try to fetch a quote
    let results = quotes::fetch_all_quotes(vec![request]).await;

    if let Some(result_quote) = results.first() {
        if result_quote.success && result_quote.latest.is_some() {
            return Some(ValidatedConfig {
                feed: provider_type.as_str().to_string(),
                feed_url,
                ticker: Some(symbol),
                exchange: result.exchange.clone(),
            });
        }
    }

    None
}

/// Get cached mapping from database
fn get_cached_mapping(security_id: i64) -> Option<SymbolMapping> {
    let conn_guard = get_connection().ok()?;
    let conn = conn_guard.as_ref()?;

    let result = conn.query_row(
        r#"
        SELECT id, security_id, validated_feed, validated_feed_url, validated_ticker,
               validated_exchange, provider_results, validation_status, confidence,
               validation_method, ai_suggestion_json, last_validated_at, price_check_success,
               created_at
        FROM pp_symbol_mapping
        WHERE security_id = ?1
        "#,
        params![security_id],
        |row| {
            Ok(SymbolMapping {
                id: row.get(0)?,
                security_id: row.get(1)?,
                validated_feed: row.get(2)?,
                validated_feed_url: row.get(3)?,
                validated_ticker: row.get(4)?,
                validated_exchange: row.get(5)?,
                provider_results: row.get(6)?,
                validation_status: ValidationStatus::from_str(&row.get::<_, String>(7)?),
                confidence: row.get(8)?,
                validation_method: row.get::<_, Option<String>>(9)?
                    .and_then(|s| ValidationMethod::from_str(&s)),
                ai_suggestion_json: row.get(10)?,
                last_validated_at: row.get(11)?,
                price_check_success: row.get::<_, i32>(12)? == 1,
                created_at: row.get(13)?,
            })
        },
    );

    result.ok()
}

/// Save or update mapping in database
fn save_mapping(
    security_id: i64,
    config: &ValidatedConfig,
    provider_results: &[ProviderSearchResult],
    status: ValidationStatus,
    confidence: f64,
    method: ValidationMethod,
    ai_suggestion: Option<&AiSuggestion>,
) -> Result<()> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    let provider_results_json = serde_json::to_string(provider_results)?;
    let ai_suggestion_json = ai_suggestion.map(|s| serde_json::to_string(s)).transpose()?;
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let price_check_success = if status == ValidationStatus::Validated { 1 } else { 0 };

    conn.execute(
        r#"
        INSERT INTO pp_symbol_mapping (
            security_id, validated_feed, validated_feed_url, validated_ticker,
            validated_exchange, provider_results, validation_status, confidence,
            validation_method, ai_suggestion_json, last_validated_at, price_check_success
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(security_id) DO UPDATE SET
            validated_feed = excluded.validated_feed,
            validated_feed_url = excluded.validated_feed_url,
            validated_ticker = excluded.validated_ticker,
            validated_exchange = excluded.validated_exchange,
            provider_results = excluded.provider_results,
            validation_status = excluded.validation_status,
            confidence = excluded.confidence,
            validation_method = excluded.validation_method,
            ai_suggestion_json = excluded.ai_suggestion_json,
            last_validated_at = excluded.last_validated_at,
            price_check_success = excluded.price_check_success
        "#,
        params![
            security_id,
            config.feed,
            config.feed_url,
            config.ticker,
            config.exchange,
            provider_results_json,
            status.as_str(),
            confidence,
            method.as_str(),
            ai_suggestion_json,
            now,
            price_check_success,
        ],
    )?;

    Ok(())
}

/// Validate all securities (or only held ones)
pub async fn validate_all_securities(
    only_held: bool,
    force: bool,
    api_keys: &ApiKeys,
    ai_config: Option<&AiConfig>,
) -> Result<Vec<ValidationResult>> {
    // Get securities to validate
    let securities = get_securities_for_validation(only_held)?;

    // Create validation run record
    let run_id = create_validation_run(securities.len() as i32)?;

    let mut results = Vec::new();
    let mut validated_count = 0;
    let mut failed_count = 0;
    let mut ai_suggested_count = 0;

    for security in securities {
        let result = validate_security(security, api_keys, ai_config, force).await;

        match result.status {
            ValidationStatus::Validated => validated_count += 1,
            ValidationStatus::Failed => failed_count += 1,
            ValidationStatus::AiSuggested => ai_suggested_count += 1,
            _ => {}
        }

        results.push(result);
    }

    // Update run status
    complete_validation_run(run_id, validated_count, failed_count, ai_suggested_count)?;

    Ok(results)
}

/// Get securities that need validation
fn get_securities_for_validation(only_held: bool) -> Result<Vec<SecurityForValidation>> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    let sql = if only_held {
        r#"
        SELECT DISTINCT s.id, s.name, s.isin, s.wkn, s.ticker, s.currency, s.feed, s.feed_url, s.is_retired
        FROM pp_security s
        INNER JOIN pp_txn t ON t.security_id = s.id
        WHERE t.owner_type = 'portfolio'
        AND t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND')
        AND s.is_retired = 0
        GROUP BY s.id
        HAVING SUM(CASE
            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
            ELSE 0
        END) > 0
        "#
    } else {
        r#"
        SELECT id, name, isin, wkn, ticker, currency, feed, feed_url, is_retired
        FROM pp_security
        WHERE is_retired = 0
        "#
    };

    let mut stmt = conn.prepare(sql)?;
    let securities = stmt.query_map([], |row| {
        Ok(SecurityForValidation {
            id: row.get(0)?,
            name: row.get(1)?,
            isin: row.get(2)?,
            wkn: row.get(3)?,
            ticker: row.get(4)?,
            currency: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "EUR".to_string()),
            feed: row.get(6)?,
            feed_url: row.get(7)?,
            is_retired: row.get::<_, i32>(8)? == 1,
        })
    })?;

    Ok(securities.filter_map(|r| r.ok()).collect())
}

/// Create a new validation run record
fn create_validation_run(total_securities: i32) -> Result<i64> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        r#"
        INSERT INTO pp_validation_run (started_at, total_securities, status)
        VALUES (?1, ?2, 'running')
        "#,
        params![now, total_securities],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Complete a validation run
fn complete_validation_run(
    run_id: i64,
    validated_count: i32,
    failed_count: i32,
    ai_suggested_count: i32,
) -> Result<()> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        r#"
        UPDATE pp_validation_run
        SET completed_at = ?1, validated_count = ?2, failed_count = ?3,
            ai_suggested_count = ?4, status = 'completed'
        WHERE id = ?5
        "#,
        params![now, validated_count, failed_count, ai_suggested_count, run_id],
    )?;

    Ok(())
}

/// Get validation status summary
pub fn get_validation_status(only_held: bool) -> Result<ValidationStatusSummary> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    // Get counts
    let base_query = if only_held {
        r#"
        SELECT COUNT(DISTINCT s.id) as total,
               COALESCE(SUM(CASE WHEN m.validation_status = 'validated' THEN 1 ELSE 0 END), 0) as validated,
               COALESCE(SUM(CASE WHEN m.validation_status = 'pending' OR m.id IS NULL THEN 1 ELSE 0 END), 0) as pending,
               COALESCE(SUM(CASE WHEN m.validation_status = 'failed' THEN 1 ELSE 0 END), 0) as failed,
               COALESCE(SUM(CASE WHEN m.validation_status = 'ai_suggested' THEN 1 ELSE 0 END), 0) as ai_suggested,
               COALESCE(SUM(CASE WHEN m.validation_status = 'skipped' THEN 1 ELSE 0 END), 0) as skipped
        FROM pp_security s
        LEFT JOIN pp_symbol_mapping m ON m.security_id = s.id
        INNER JOIN pp_txn t ON t.security_id = s.id
        WHERE t.owner_type = 'portfolio'
        AND s.is_retired = 0
        GROUP BY s.id
        HAVING SUM(CASE
            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
            ELSE 0
        END) > 0
        "#
    } else {
        r#"
        SELECT COUNT(*) as total,
               SUM(CASE WHEN m.validation_status = 'validated' THEN 1 ELSE 0 END) as validated,
               SUM(CASE WHEN m.validation_status = 'pending' OR m.id IS NULL THEN 1 ELSE 0 END) as pending,
               SUM(CASE WHEN m.validation_status = 'failed' THEN 1 ELSE 0 END) as failed,
               SUM(CASE WHEN m.validation_status = 'ai_suggested' THEN 1 ELSE 0 END) as ai_suggested,
               SUM(CASE WHEN m.validation_status = 'skipped' THEN 1 ELSE 0 END) as skipped
        FROM pp_security s
        LEFT JOIN pp_symbol_mapping m ON m.security_id = s.id
        WHERE s.is_retired = 0
        "#
    };

    let counts: (i32, i32, i32, i32, i32, i32) = conn.query_row(base_query, [], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
        ))
    }).unwrap_or((0, 0, 0, 0, 0, 0));

    // Get last run
    let last_run: Option<ValidationRun> = conn.query_row(
        r#"
        SELECT id, started_at, completed_at, total_securities, validated_count,
               failed_count, ai_suggested_count, status
        FROM pp_validation_run
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            Ok(ValidationRun {
                id: row.get(0)?,
                started_at: row.get(1)?,
                completed_at: row.get(2)?,
                total_securities: row.get(3)?,
                validated_count: row.get(4)?,
                failed_count: row.get(5)?,
                ai_suggested_count: row.get(6)?,
                status: row.get(7)?,
            })
        },
    ).ok();

    // Get securities needing attention (failed or AI-suggested)
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.name, s.isin, s.feed, s.ticker,
               m.validation_status, m.confidence, m.ai_suggestion_json
        FROM pp_security s
        LEFT JOIN pp_symbol_mapping m ON m.security_id = s.id
        WHERE s.is_retired = 0
        AND (m.validation_status IN ('failed', 'ai_suggested') OR m.id IS NULL)
        LIMIT 50
        "#,
    )?;

    let securities_needing_attention: Vec<ValidationResult> = stmt.query_map([], |row| {
        let status_str: Option<String> = row.get(5)?;
        let status = status_str
            .map(|s| ValidationStatus::from_str(&s))
            .unwrap_or(ValidationStatus::Pending);

        let ai_json: Option<String> = row.get(7)?;
        let ai_suggestion = ai_json.and_then(|j| serde_json::from_str(&j).ok());

        Ok(ValidationResult {
            security_id: row.get(0)?,
            security_name: row.get(1)?,
            isin: row.get(2)?,
            original_feed: row.get(3)?,
            original_ticker: row.get(4)?,
            status,
            validated_config: None,
            ai_suggestion,
            provider_results: vec![],
            confidence: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
            error: None,
        })
    })?.filter_map(|r| r.ok()).collect();

    Ok(ValidationStatusSummary {
        total_securities: counts.0,
        validated_count: counts.1,
        pending_count: counts.2,
        failed_count: counts.3,
        ai_suggested_count: counts.4,
        skipped_count: counts.5,
        last_run,
        securities_needing_attention,
    })
}

/// Apply a validation result to the actual security
pub fn apply_validation_result(security_id: i64, config: &ValidatedConfig) -> Result<()> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    conn.execute(
        r#"
        UPDATE pp_security
        SET feed = ?1, feed_url = ?2, ticker = COALESCE(?3, ticker)
        WHERE id = ?4
        "#,
        params![
            config.feed,
            config.feed_url,
            config.ticker,
            security_id,
        ],
    )?;

    // Update mapping status to validated (user confirmed)
    conn.execute(
        r#"
        UPDATE pp_symbol_mapping
        SET validation_status = 'validated', validation_method = 'user', last_validated_at = ?1
        WHERE security_id = ?2
        "#,
        params![
            Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            security_id,
        ],
    )?;

    Ok(())
}
