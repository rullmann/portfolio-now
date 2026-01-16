//! AI Helper commands for ChatBot integration
//!
//! These commands are designed to be called by the AI assistant
//! to perform actions on behalf of the user.

use crate::db;
use crate::quotes::{alphavantage, portfolio_report, yahoo};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSecurityMatch {
    pub id: i64,
    pub name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub currency: String,
    pub source: String, // "database" or "external"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWatchlistResult {
    pub success: bool,
    pub message: String,
    pub watchlist_name: String,
    pub security_name: Option<String>,
    pub security_ticker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWatchlistInfo {
    pub id: i64,
    pub name: String,
    pub securities: Vec<AiWatchlistSecurity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWatchlistSecurity {
    pub id: i64,
    pub name: String,
    pub ticker: Option<String>,
    pub isin: Option<String>,
}

// ============================================================================
// AI Helper Commands
// ============================================================================

/// Search for a security in the database and external providers.
/// Returns matches from database first, then external sources.
#[command]
pub async fn ai_search_security(
    query: String,
    alpha_vantage_api_key: Option<String>,
) -> Result<Vec<AiSecurityMatch>, String> {
    let query = query.trim().to_lowercase();
    if query.len() < 2 {
        return Err("Query must be at least 2 characters".to_string());
    }

    let mut results = Vec::new();

    // First, search in database
    {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, isin, ticker, currency
                FROM pp_security
                WHERE LOWER(name) LIKE ?1
                   OR LOWER(ticker) LIKE ?1
                   OR LOWER(isin) LIKE ?1
                ORDER BY
                    CASE WHEN LOWER(ticker) = ?2 THEN 0
                         WHEN LOWER(name) = ?2 THEN 1
                         ELSE 2 END,
                    name
                LIMIT 10
                "#,
            )
            .map_err(|e| e.to_string())?;

        let pattern = format!("%{}%", query);
        let rows = stmt
            .query_map(params![pattern, query], |row| {
                Ok(AiSecurityMatch {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    isin: row.get(2)?,
                    ticker: row.get(3)?,
                    currency: row.get(4)?,
                    source: "database".to_string(),
                })
            })
            .map_err(|e| e.to_string())?;

        for row in rows.flatten() {
            results.push(row);
        }
    }

    // If no database results, search external providers
    if results.is_empty() {
        // Try Yahoo Finance
        if let Ok(yahoo_results) = yahoo::search(&query).await {
            for result in yahoo_results.into_iter().take(5) {
                results.push(AiSecurityMatch {
                    id: 0, // Not in database yet
                    name: result.name,
                    isin: None,
                    ticker: Some(result.symbol),
                    currency: "USD".to_string(), // Yahoo doesn't provide currency
                    source: "yahoo".to_string(),
                });
            }
        }

        // Try Alpha Vantage if API key provided
        if let Some(api_key) = alpha_vantage_api_key {
            if !api_key.is_empty() {
                if let Ok(av_results) = alphavantage::search(&query, &api_key).await {
                    for result in av_results.into_iter().take(3) {
                        // Check if already in results by symbol
                        let symbol_lower = result.symbol.to_lowercase();
                        if !results.iter().any(|r| {
                            r.ticker
                                .as_ref()
                                .map(|t| t.to_lowercase() == symbol_lower)
                                .unwrap_or(false)
                        }) {
                            results.push(AiSecurityMatch {
                                id: 0,
                                name: result.name,
                                isin: None,
                                ticker: Some(result.symbol),
                                currency: result.currency.clone(),
                                source: "alphavantage".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Add a security to a watchlist by name.
/// Creates the watchlist if it doesn't exist.
/// Creates/finds the security by query (name, ticker, or ISIN).
#[command]
pub async fn ai_add_to_watchlist(
    watchlist_name: String,
    security_query: String,
    alpha_vantage_api_key: Option<String>,
) -> Result<AiWatchlistResult, String> {
    let watchlist_name = watchlist_name.trim();
    let security_query = security_query.trim();

    if watchlist_name.is_empty() {
        return Err("Watchlist name cannot be empty".to_string());
    }
    if security_query.is_empty() {
        return Err("Security query cannot be empty".to_string());
    }

    // Step 1: Find or create watchlist
    let watchlist_id = find_or_create_watchlist(watchlist_name)?;

    // Step 2: Find or create security
    let (security_id, security_name, security_ticker) =
        find_or_create_security(security_query, alpha_vantage_api_key).await?;

    // Step 3: Add security to watchlist
    {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO pp_watchlist_security (watchlist_id, security_id) VALUES (?1, ?2)",
            params![watchlist_id, security_id],
        )
        .map_err(|e| e.to_string())?;
    }

    let ticker_info = security_ticker
        .as_ref()
        .map(|t| format!(" ({})", t))
        .unwrap_or_default();

    Ok(AiWatchlistResult {
        success: true,
        message: format!(
            "{}{} wurde zur Watchlist \"{}\" hinzugefügt.",
            security_name, ticker_info, watchlist_name
        ),
        watchlist_name: watchlist_name.to_string(),
        security_name: Some(security_name),
        security_ticker,
    })
}

/// Remove a security from a watchlist by name.
#[command]
pub fn ai_remove_from_watchlist(
    watchlist_name: String,
    security_query: String,
) -> Result<AiWatchlistResult, String> {
    let watchlist_name = watchlist_name.trim();
    let security_query = security_query.trim().to_lowercase();

    if watchlist_name.is_empty() {
        return Err("Watchlist name cannot be empty".to_string());
    }
    if security_query.is_empty() {
        return Err("Security query cannot be empty".to_string());
    }

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Find watchlist
    let watchlist_id: i64 = conn
        .query_row(
            "SELECT id FROM pp_watchlist WHERE LOWER(name) = LOWER(?1)",
            params![watchlist_name],
            |row| row.get(0),
        )
        .map_err(|_| format!("Watchlist \"{}\" nicht gefunden.", watchlist_name))?;

    // Find security in watchlist
    let (security_id, security_name, security_ticker): (i64, String, Option<String>) = conn
        .query_row(
            r#"
            SELECT s.id, s.name, s.ticker
            FROM pp_watchlist_security ws
            JOIN pp_security s ON s.id = ws.security_id
            WHERE ws.watchlist_id = ?1
              AND (LOWER(s.name) LIKE ?2 OR LOWER(s.ticker) LIKE ?2 OR LOWER(s.isin) LIKE ?2)
            LIMIT 1
            "#,
            params![watchlist_id, format!("%{}%", security_query)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| {
            format!(
                "Security \"{}\" nicht in Watchlist \"{}\" gefunden.",
                security_query, watchlist_name
            )
        })?;

    // Remove from watchlist
    conn.execute(
        "DELETE FROM pp_watchlist_security WHERE watchlist_id = ?1 AND security_id = ?2",
        params![watchlist_id, security_id],
    )
    .map_err(|e| e.to_string())?;

    let ticker_info = security_ticker
        .as_ref()
        .map(|t| format!(" ({})", t))
        .unwrap_or_default();

    Ok(AiWatchlistResult {
        success: true,
        message: format!(
            "{}{} wurde von der Watchlist \"{}\" entfernt.",
            security_name, ticker_info, watchlist_name
        ),
        watchlist_name: watchlist_name.to_string(),
        security_name: Some(security_name),
        security_ticker,
    })
}

/// List all watchlists with their securities.
#[command]
pub fn ai_list_watchlists() -> Result<Vec<AiWatchlistInfo>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, name FROM pp_watchlist ORDER BY name")
        .map_err(|e| e.to_string())?;

    let watchlists: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();

    let mut results = Vec::new();

    for (wl_id, wl_name) in watchlists {
        let mut sec_stmt = conn
            .prepare(
                r#"
                SELECT s.id, s.name, s.ticker, s.isin
                FROM pp_watchlist_security ws
                JOIN pp_security s ON s.id = ws.security_id
                WHERE ws.watchlist_id = ?1
                ORDER BY s.name
                "#,
            )
            .map_err(|e| e.to_string())?;

        let securities: Vec<AiWatchlistSecurity> = sec_stmt
            .query_map(params![wl_id], |row| {
                Ok(AiWatchlistSecurity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    ticker: row.get(2)?,
                    isin: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();

        results.push(AiWatchlistInfo {
            id: wl_id,
            name: wl_name,
            securities,
        });
    }

    Ok(results)
}

// ============================================================================
// Transaction Query Commands
// ============================================================================

/// Transaction info for AI queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransaction {
    pub date: String,
    pub txn_type: String,
    pub security_name: Option<String>,
    pub ticker: Option<String>,
    pub shares: Option<f64>,
    pub amount: f64,
    pub currency: String,
    pub note: Option<String>,
}

/// Result of transaction query
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransactionQueryResult {
    pub transactions: Vec<AiTransaction>,
    pub total_count: i32,
    pub message: String,
}

/// Query transactions with optional filters.
/// Can filter by security name/ticker, year, and transaction type.
/// DELIVERY_INBOUND is treated as BUY, DELIVERY_OUTBOUND as SELL.
#[command]
pub fn ai_query_transactions(
    security: Option<String>,
    year: Option<i32>,
    txn_type: Option<String>,
    limit: Option<i32>,
) -> Result<AiTransactionQueryResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let limit = limit.unwrap_or(100).min(500); // Max 500 transactions

    // Build dynamic SQL based on filters
    let mut conditions = Vec::new();

    // Security filter
    if let Some(ref sec) = security {
        let sec_lower = sec.trim().to_lowercase();
        conditions.push(format!(
            "(LOWER(s.name) LIKE '%{}%' OR LOWER(s.ticker) LIKE '%{}%' OR LOWER(s.isin) LIKE '%{}%')",
            sec_lower, sec_lower, sec_lower
        ));
    }

    // Year filter
    if let Some(y) = year {
        conditions.push(format!("strftime('%Y', t.date) = '{}'", y));
    }

    // Transaction type filter (map DELIVERY to BUY/SELL for display)
    if let Some(ref tt) = txn_type {
        let tt_upper = tt.trim().to_uppercase();
        match tt_upper.as_str() {
            "BUY" | "KAUF" => {
                conditions.push("t.txn_type IN ('BUY', 'DELIVERY_INBOUND')".to_string());
            }
            "SELL" | "VERKAUF" => {
                conditions.push("t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND')".to_string());
            }
            "DIVIDEND" | "DIVIDENDE" | "DIVIDENDS" => {
                conditions.push("t.txn_type = 'DIVIDENDS'".to_string());
            }
            _ => {
                conditions.push(format!("t.txn_type = '{}'", tt_upper));
            }
        }
    }

    let where_clause = if conditions.is_empty() {
        "WHERE t.date IS NOT NULL".to_string()
    } else {
        format!("WHERE t.date IS NOT NULL AND {}", conditions.join(" AND "))
    };

    let sql = format!(
        r#"
        SELECT t.date, t.txn_type, s.name, s.ticker, t.shares, t.amount, t.currency, t.note
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        {}
        ORDER BY t.date DESC
        LIMIT {}
        "#,
        where_clause, limit
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let transactions: Vec<AiTransaction> = stmt
        .query_map([], |row| {
            let shares_raw: Option<i64> = row.get(4)?;
            let amount_raw: i64 = row.get(5)?;
            let txn_type_raw: String = row.get(1)?;

            // Map DELIVERY types to BUY/SELL for clarity
            let txn_type_display = match txn_type_raw.as_str() {
                "DELIVERY_INBOUND" => "BUY (Einlieferung)".to_string(),
                "DELIVERY_OUTBOUND" => "SELL (Auslieferung)".to_string(),
                other => other.to_string(),
            };

            Ok(AiTransaction {
                date: row.get(0)?,
                txn_type: txn_type_display,
                security_name: row.get(2)?,
                ticker: row.get(3)?,
                shares: shares_raw.map(|s| s as f64 / 100_000_000.0),
                amount: amount_raw as f64 / 100.0,
                currency: row.get(6)?,
                note: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get total count
    let count_sql = format!(
        r#"
        SELECT COUNT(*)
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        {}
        "#,
        where_clause
    );
    let total_count: i32 = conn
        .query_row(&count_sql, [], |row| row.get(0))
        .unwrap_or(0);

    let message = if transactions.is_empty() {
        "Keine Transaktionen gefunden.".to_string()
    } else {
        format!(
            "{} Transaktionen gefunden (zeige {}).",
            total_count,
            transactions.len()
        )
    };

    Ok(AiTransactionQueryResult {
        transactions,
        total_count,
        message,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn find_or_create_watchlist(name: &str) -> Result<i64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Try to find existing watchlist (case-insensitive)
    if let Ok(id) = conn.query_row::<i64, _, _>(
        "SELECT id FROM pp_watchlist WHERE LOWER(name) = LOWER(?1)",
        params![name],
        |row| row.get(0),
    ) {
        return Ok(id);
    }

    // Create new watchlist
    let import_id: i64 = conn
        .query_row(
            "SELECT id FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO pp_watchlist (import_id, name) VALUES (?1, ?2)",
        params![import_id, name],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

async fn find_or_create_security(
    query: &str,
    alpha_vantage_api_key: Option<String>,
) -> Result<(i64, String, Option<String>), String> {
    let query_lower = query.to_lowercase();

    // First, try to find in database
    {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        // Exact match on ticker or ISIN first
        if let Ok((id, name, ticker)) = conn.query_row::<(i64, String, Option<String>), _, _>(
            r#"
            SELECT id, name, ticker FROM pp_security
            WHERE LOWER(ticker) = ?1 OR LOWER(isin) = ?1
            LIMIT 1
            "#,
            params![query_lower],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ) {
            return Ok((id, name, ticker));
        }

        // Fuzzy match on name
        if let Ok((id, name, ticker)) = conn.query_row::<(i64, String, Option<String>), _, _>(
            r#"
            SELECT id, name, ticker FROM pp_security
            WHERE LOWER(name) LIKE ?1
            ORDER BY LENGTH(name)
            LIMIT 1
            "#,
            params![format!("%{}%", query_lower)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ) {
            return Ok((id, name, ticker));
        }
    }

    // Not found in database, search external providers
    let mut external_match: Option<(String, String, String)> = None; // (name, ticker, currency)

    // Try Yahoo Finance
    if let Ok(yahoo_results) = yahoo::search(query).await {
        if let Some(first) = yahoo_results.into_iter().next() {
            external_match = Some((
                first.name,
                first.symbol,
                "USD".to_string(), // Yahoo doesn't provide currency
            ));
        }
    }

    // Try Alpha Vantage if no Yahoo result
    if external_match.is_none() {
        if let Some(api_key) = alpha_vantage_api_key {
            if !api_key.is_empty() {
                if let Ok(av_results) = alphavantage::search(query, &api_key).await {
                    if let Some(first) = av_results.into_iter().next() {
                        external_match = Some((
                            first.name,
                            first.symbol,
                            first.currency.clone(),
                        ));
                    }
                }
            }
        }
    }

    // If found externally, create in database
    if let Some((name, ticker, currency)) = external_match {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        // Check if ticker already exists (might have been added meanwhile)
        if let Ok((id, db_name, db_ticker)) =
            conn.query_row::<(i64, String, Option<String>), _, _>(
                "SELECT id, name, ticker FROM pp_security WHERE LOWER(ticker) = LOWER(?1)",
                params![ticker],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
        {
            return Ok((id, db_name, db_ticker));
        }

        // Create new security
        let uuid = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO pp_security (uuid, name, currency, ticker, feed, is_retired, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'YAHOO', 0, ?5)
            "#,
            params![uuid, name, currency, ticker, now],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        let name_clone = name.clone();
        let ticker_clone = ticker.clone();

        // Spawn background tasks for enrichment and price fetching
        tokio::spawn(async move {
            // Enrich with ISIN/WKN from Portfolio Report
            enrich_security_data(id, Some(&ticker_clone), &name_clone).await;
            // Fetch current and historical prices
            fetch_prices_for_security(id).await;
        });

        return Ok((id, name, Some(ticker)));
    }

    Err(format!(
        "Security \"{}\" konnte nicht gefunden werden.",
        query
    ))
}

/// Enrich security data with ISIN, WKN from Portfolio Report.
/// Updates the security record in the database.
async fn enrich_security_data(security_id: i64, ticker: Option<&str>, name: &str) {
    // Search Portfolio Report for ISIN/WKN
    if let Some((uuid, isin, wkn)) =
        portfolio_report::search_and_get_identifiers(ticker, name).await
    {
        // Update security in database
        if let Ok(conn_guard) = db::get_connection() {
            if let Some(conn) = conn_guard.as_ref() {
                let now = chrono::Utc::now().to_rfc3339();
                let _ = conn.execute(
                    r#"
                    UPDATE pp_security
                    SET isin = COALESCE(isin, ?1),
                        wkn = COALESCE(wkn, ?2),
                        feed_url = ?3,
                        updated_at = ?4
                    WHERE id = ?5
                    "#,
                    params![isin, wkn, uuid, now, security_id],
                );
                log::info!(
                    "Enriched security {} with ISIN={:?}, WKN={:?}",
                    security_id,
                    isin,
                    wkn
                );
            }
        }
    }
}

/// Fetch current and historical prices for a newly added security.
async fn fetch_prices_for_security(security_id: i64) {

    // Get security info
    let security_info: Option<(String, Option<String>, Option<String>)> = {
        if let Ok(conn_guard) = db::get_connection() {
            if let Some(conn) = conn_guard.as_ref() {
                conn.query_row(
                    "SELECT currency, ticker, feed FROM pp_security WHERE id = ?1",
                    params![security_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .ok()
            } else {
                None
            }
        } else {
            None
        }
    };

    let Some((currency, ticker, feed)) = security_info else {
        return;
    };

    // Only fetch for YAHOO feed securities
    if feed.as_deref() != Some("YAHOO") {
        return;
    }

    let Some(ticker) = ticker else {
        return;
    };

    // Fetch current price
    if let Ok(quote) = yahoo::fetch_quote(&ticker, true).await {
        if let Ok(conn_guard) = db::get_connection() {
            if let Some(conn) = conn_guard.as_ref() {
                let price_scaled = (quote.quote.close * 100_000_000.0) as i64;
                let high_scaled = quote.quote.high.map(|h| (h * 100_000_000.0) as i64);
                let low_scaled = quote.quote.low.map(|l| (l * 100_000_000.0) as i64);
                let volume = quote.quote.volume.map(|v| v as i64);
                let date_str = quote.quote.date.format("%Y-%m-%d").to_string();

                // Insert latest price
                let _ = conn.execute(
                    r#"
                    INSERT OR REPLACE INTO pp_latest_price
                    (security_id, date, value, high, low, volume)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                    params![security_id, date_str, price_scaled, high_scaled, low_scaled, volume],
                );

                // Also insert into price history
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO pp_price (security_id, date, value) VALUES (?1, ?2, ?3)",
                    params![security_id, date_str, price_scaled],
                );

                log::info!(
                    "Fetched current price for security {}: {} {}",
                    security_id,
                    quote.quote.close,
                    currency
                );
            }
        }
    }

    // Fetch historical prices (last 3 months)
    let today = Utc::now().date_naive();
    let three_months_ago = today - chrono::Duration::days(90);

    if let Ok(history) =
        yahoo::fetch_historical(&ticker, three_months_ago, today, true).await
    {
        if let Ok(conn_guard) = db::get_connection() {
            if let Some(conn) = conn_guard.as_ref() {
                let mut inserted = 0;
                for quote in history {
                    let price_scaled = (quote.close * 100_000_000.0) as i64;
                    let date_str = quote.date.format("%Y-%m-%d").to_string();

                    if conn
                        .execute(
                            "INSERT OR IGNORE INTO pp_price (security_id, date, value) VALUES (?1, ?2, ?3)",
                            params![security_id, date_str, price_scaled],
                        )
                        .is_ok()
                    {
                        inserted += 1;
                    }
                }
                log::info!(
                    "Fetched {} historical prices for security {}",
                    inserted,
                    security_id
                );
            }
        }
    }
}

// ============================================================================
// Portfolio Value Query
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPortfolioValueResult {
    pub date: String,
    pub value: f64,
    pub currency: String,
    pub found: bool,
    pub message: String,
}

/// Query portfolio value at a specific date.
/// Used by ChatBot to answer questions like "Wie hoch stand das Depot am 04.04.2025?"
#[command]
pub fn ai_query_portfolio_value(date: String) -> Result<AiPortfolioValueResult, String> {
    use crate::commands::data::get_portfolio_history;

    // Parse and validate date
    let target_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| format!("Ungültiges Datumsformat: {}. Erwartet: YYYY-MM-DD", date))?;

    let target_str = target_date.format("%Y-%m-%d").to_string();

    // Get portfolio history
    let history = get_portfolio_history()?;

    if history.is_empty() {
        return Ok(AiPortfolioValueResult {
            date: target_str,
            value: 0.0,
            currency: "EUR".to_string(),
            found: false,
            message: "Keine historischen Portfolio-Daten verfügbar.".to_string(),
        });
    }

    // Find exact match or closest date before target
    let mut closest_value: Option<(String, f64)> = None;

    for point in &history {
        if point.date == target_str {
            // Exact match
            return Ok(AiPortfolioValueResult {
                date: target_str,
                value: point.value,
                currency: "EUR".to_string(),
                found: true,
                message: format!("Depotwert am {}: {:.2} EUR", date, point.value),
            });
        }

        if point.date < target_str {
            closest_value = Some((point.date.clone(), point.value));
        }
    }

    // Check if target is in the future (after last history point)
    if let Some(last) = history.last() {
        if target_str > last.date {
            return Ok(AiPortfolioValueResult {
                date: target_str,
                value: 0.0,
                currency: "EUR".to_string(),
                found: false,
                message: format!(
                    "Datum {} liegt in der Zukunft oder es gibt keine Daten dafür. Letzter bekannter Wert: {} am {}",
                    date, last.value, last.date
                ),
            });
        }
    }

    // Use closest date before target
    if let Some((closest_date, value)) = closest_value {
        return Ok(AiPortfolioValueResult {
            date: closest_date.clone(),
            value,
            currency: "EUR".to_string(),
            found: true,
            message: format!(
                "Kein exakter Wert für {} verfügbar. Nächster bekannter Wert: {:.2} EUR am {}",
                date, value, closest_date
            ),
        });
    }

    // Target is before first history point
    if let Some(first) = history.first() {
        return Ok(AiPortfolioValueResult {
            date: target_str,
            value: 0.0,
            currency: "EUR".to_string(),
            found: false,
            message: format!(
                "Datum {} liegt vor dem ersten aufgezeichneten Wert. Ältester bekannter Wert: {:.2} EUR am {}",
                date, first.value, first.date
            ),
        });
    }

    Ok(AiPortfolioValueResult {
        date: target_str,
        value: 0.0,
        currency: "EUR".to_string(),
        found: false,
        message: "Keine Daten gefunden.".to_string(),
    })
}

// ============================================================================
// API Key Management Commands
// ============================================================================

/// Result of API key save operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSaveApiKeyResult {
    pub success: bool,
    pub message: String,
    pub provider: String,
}

/// Save an API key to secure storage.
/// Used by ChatBot to allow users to configure API keys via conversation.
///
/// Valid providers:
/// - AI: "anthropic", "openai", "gemini", "perplexity"
/// - Quote: "finnhub", "coingecko", "alphaVantage", "twelveData", "brandfetch"
/// - Export: "divvyDiary"
#[command]
pub async fn ai_save_api_key(
    provider: String,
    api_key: String,
    app: tauri::AppHandle,
) -> Result<AiSaveApiKeyResult, String> {
    use tauri_plugin_store::StoreExt;

    let provider_lower = provider.to_lowercase();

    // Map provider names to store keys
    let store_key = match provider_lower.as_str() {
        // AI Providers
        "anthropic" | "claude" => "anthropic",
        "openai" | "gpt" | "chatgpt" => "openai",
        "gemini" | "google" => "gemini",
        "perplexity" => "perplexity",
        // Quote Providers
        "finnhub" => "finnhub",
        "coingecko" => "coingecko",
        "alphavantage" | "alpha_vantage" | "alpha-vantage" => "alphaVantage",
        "twelvedata" | "twelve_data" | "twelve-data" => "twelveData",
        "brandfetch" => "brandfetch",
        // Export Services
        "divvydiary" | "divvy_diary" | "divvy-diary" => "divvyDiary",
        _ => {
            return Ok(AiSaveApiKeyResult {
                success: false,
                message: format!(
                    "Unbekannter Provider: \"{}\". Gültige Provider: anthropic, openai, gemini, perplexity, finnhub, coingecko, alphaVantage, twelveData, brandfetch, divvyDiary",
                    provider
                ),
                provider: provider_lower,
            });
        }
    };

    // Validate API key is not empty
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Ok(AiSaveApiKeyResult {
            success: false,
            message: "API-Key darf nicht leer sein.".to_string(),
            provider: store_key.to_string(),
        });
    }

    // Store in secure storage
    let store = app
        .store("secure-keys.json")
        .map_err(|e| format!("Fehler beim Öffnen des sicheren Speichers: {}", e))?;

    // set() doesn't return Result in tauri-plugin-store v2
    store.set(store_key.to_string(), serde_json::json!(api_key));

    store
        .save()
        .map_err(|e| format!("Fehler beim Speichern: {}", e))?;

    log::info!("AI saved API key for provider: {}", store_key);

    Ok(AiSaveApiKeyResult {
        success: true,
        message: format!(
            "{} API-Key wurde erfolgreich gespeichert. Bitte die App neu starten, damit die Änderung wirksam wird.",
            match store_key {
                "anthropic" => "Claude (Anthropic)",
                "openai" => "OpenAI",
                "gemini" => "Google Gemini",
                "perplexity" => "Perplexity",
                "finnhub" => "Finnhub",
                "coingecko" => "CoinGecko",
                "alphaVantage" => "Alpha Vantage",
                "twelveData" => "Twelve Data",
                "brandfetch" => "Brandfetch",
                "divvyDiary" => "DivvyDiary",
                _ => store_key,
            }
        ),
        provider: store_key.to_string(),
    })
}
