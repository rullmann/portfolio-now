//! Watchlist management commands for Tauri
//!
//! Watchlists allow users to track securities they're interested in
//! without holding them in a portfolio.

use crate::db;
use crate::pp::common::prices;
use serde::{Deserialize, Serialize};
use tauri::command;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistData {
    pub id: i64,
    pub name: String,
    pub securities_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistSecurityData {
    pub security_id: i64,
    pub security_uuid: String,
    pub name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub currency: String,
    pub latest_price: Option<f64>,
    pub latest_date: Option<String>,
    /// Price change from previous close (absolute)
    pub price_change: Option<f64>,
    /// Price change percentage
    pub price_change_percent: Option<f64>,
    /// 52-week high
    pub high_52w: Option<f64>,
    /// 52-week low
    pub low_52w: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistWithSecurities {
    pub id: i64,
    pub name: String,
    pub securities: Vec<WatchlistSecurityData>,
}

// ============================================================================
// Watchlist CRUD
// ============================================================================

/// Get all watchlists
#[command]
pub fn get_watchlists() -> Result<Vec<WatchlistData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                w.id, w.name,
                (SELECT COUNT(*) FROM pp_watchlist_security WHERE watchlist_id = w.id) as cnt
            FROM pp_watchlist w
            ORDER BY w.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(WatchlistData {
                id: row.get(0)?,
                name: row.get(1)?,
                securities_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get a single watchlist with all securities
#[command]
pub fn get_watchlist(id: i64) -> Result<WatchlistWithSecurities, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let name: String = conn
        .query_row("SELECT name FROM pp_watchlist WHERE id = ?", [id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let securities = get_watchlist_securities_internal(conn, id)?;

    Ok(WatchlistWithSecurities { id, name, securities })
}

/// Create a new watchlist
#[command]
pub fn create_watchlist(name: String) -> Result<WatchlistData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let import_id: i64 = conn
        .query_row("SELECT id FROM pp_import ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO pp_watchlist (import_id, name) VALUES (?, ?)",
        rusqlite::params![import_id, name],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(WatchlistData {
        id,
        name,
        securities_count: 0,
    })
}

/// Rename a watchlist
#[command]
pub fn rename_watchlist(id: i64, name: String) -> Result<WatchlistData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "UPDATE pp_watchlist SET name = ? WHERE id = ?",
        rusqlite::params![name, id],
    )
    .map_err(|e| e.to_string())?;

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_watchlist_security WHERE watchlist_id = ?",
            [id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(WatchlistData {
        id,
        name,
        securities_count: count,
    })
}

/// Delete a watchlist
#[command]
pub fn delete_watchlist(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Securities are automatically removed due to CASCADE
    conn.execute("DELETE FROM pp_watchlist WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Watchlist Securities
// ============================================================================

/// Get securities in a watchlist with price data
#[command]
pub fn get_watchlist_securities(watchlist_id: i64) -> Result<Vec<WatchlistSecurityData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    get_watchlist_securities_internal(conn, watchlist_id)
}

fn get_watchlist_securities_internal(
    conn: &rusqlite::Connection,
    watchlist_id: i64,
) -> Result<Vec<WatchlistSecurityData>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                s.id, s.uuid, s.name, s.isin, s.ticker, s.currency,
                lp.value as latest_price, lp.date as latest_date,
                lp.high, lp.low
            FROM pp_watchlist_security ws
            JOIN pp_security s ON s.id = ws.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            WHERE ws.watchlist_id = ?
            ORDER BY s.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([watchlist_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut securities = Vec::new();

    for row in rows.flatten() {
        let (id, uuid, name, isin, ticker, currency, latest_price, latest_date, _high, _low) = row;

        let price = latest_price.map(prices::to_decimal);

        // Get previous close for change calculation
        let prev_close: Option<i64> = if latest_date.is_some() {
            conn.query_row(
                r#"
                SELECT value FROM pp_price
                WHERE security_id = ? AND date < ?
                ORDER BY date DESC LIMIT 1
                "#,
                rusqlite::params![id, latest_date],
                |row| row.get(0),
            )
            .ok()
        } else {
            None
        };

        let (price_change, price_change_percent) = match (price, prev_close.map(prices::to_decimal)) {
            (Some(current), Some(prev)) if prev > 0.0 => {
                let change = current - prev;
                let pct = (change / prev) * 100.0;
                (Some(change), Some(pct))
            }
            _ => (None, None),
        };

        // Get 52-week high/low from price history
        let (high_52w, low_52w) = get_52w_range(conn, id);

        securities.push(WatchlistSecurityData {
            security_id: id,
            security_uuid: uuid,
            name,
            isin,
            ticker,
            currency,
            latest_price: price,
            latest_date,
            price_change,
            price_change_percent,
            high_52w,
            low_52w,
        });
    }

    Ok(securities)
}

fn get_52w_range(conn: &rusqlite::Connection, security_id: i64) -> (Option<f64>, Option<f64>) {
    let result: Result<(Option<i64>, Option<i64>), _> = conn.query_row(
        r#"
        SELECT MAX(value), MIN(value)
        FROM pp_price
        WHERE security_id = ?
          AND date >= date('now', '-1 year')
        "#,
        [security_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((Some(high), Some(low))) => (
            Some(prices::to_decimal(high)),
            Some(prices::to_decimal(low)),
        ),
        _ => (None, None),
    }
}

/// Add a security to a watchlist
#[command]
pub fn add_to_watchlist(watchlist_id: i64, security_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO pp_watchlist_security (watchlist_id, security_id) VALUES (?, ?)",
        rusqlite::params![watchlist_id, security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove a security from a watchlist
#[command]
pub fn remove_from_watchlist(watchlist_id: i64, security_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "DELETE FROM pp_watchlist_security WHERE watchlist_id = ? AND security_id = ?",
        rusqlite::params![watchlist_id, security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Add multiple securities to a watchlist
#[command]
pub fn add_securities_to_watchlist(watchlist_id: i64, security_ids: Vec<i64>) -> Result<i32, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut added = 0;
    for security_id in security_ids {
        let result = conn.execute(
            "INSERT OR IGNORE INTO pp_watchlist_security (watchlist_id, security_id) VALUES (?, ?)",
            rusqlite::params![watchlist_id, security_id],
        );
        if result.is_ok() {
            added += 1;
        }
    }

    Ok(added)
}

/// Check if a security is in any watchlist
#[command]
pub fn get_watchlists_for_security(security_id: i64) -> Result<Vec<WatchlistData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                w.id, w.name,
                (SELECT COUNT(*) FROM pp_watchlist_security WHERE watchlist_id = w.id) as cnt
            FROM pp_watchlist w
            JOIN pp_watchlist_security ws ON ws.watchlist_id = w.id
            WHERE ws.security_id = ?
            ORDER BY w.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([security_id], |row| {
            Ok(WatchlistData {
                id: row.get(0)?,
                name: row.get(1)?,
                securities_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
