//! CRUD commands for managing securities, accounts, portfolios, and transactions.

use crate::db;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

// =============================================================================
// Security CRUD
// =============================================================================

/// Input data for creating a new security
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecurityRequest {
    pub name: String,
    pub currency: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub feed: Option<String>,           // Provider for historical quotes
    pub feed_url: Option<String>,       // URL/suffix for historical quotes
    pub latest_feed: Option<String>,    // Provider for current quotes
    pub latest_feed_url: Option<String>, // URL/suffix for current quotes
    pub note: Option<String>,
}

/// Input data for updating a security
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityRequest {
    pub name: Option<String>,
    pub currency: Option<String>,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub feed: Option<String>,           // Provider for historical quotes
    pub feed_url: Option<String>,       // URL/suffix for historical quotes
    pub latest_feed: Option<String>,    // Provider for current quotes
    pub latest_feed_url: Option<String>, // URL/suffix for current quotes
    pub note: Option<String>,
    pub is_retired: Option<bool>,
}

/// Security data returned after create/update
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityResult {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub currency: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub feed: Option<String>,
    pub feed_url: Option<String>,
    pub latest_feed: Option<String>,
    pub latest_feed_url: Option<String>,
    pub note: Option<String>,
    pub is_retired: bool,
}

/// Validate ISIN checksum (ISO 7812)
fn validate_isin(isin: &str) -> bool {
    if isin.len() != 12 {
        return false;
    }

    // First two characters must be letters (country code)
    let chars: Vec<char> = isin.chars().collect();
    if !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
        return false;
    }

    // Convert to digits for Luhn algorithm
    let mut digits = String::new();
    for c in chars.iter() {
        if c.is_ascii_digit() {
            digits.push(*c);
        } else if c.is_ascii_alphabetic() {
            // A=10, B=11, ..., Z=35
            let val = c.to_ascii_uppercase() as u32 - 'A' as u32 + 10;
            digits.push_str(&val.to_string());
        } else {
            return false;
        }
    }

    // Luhn algorithm
    let digit_chars: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    let mut sum = 0;
    let len = digit_chars.len();

    for (i, &digit) in digit_chars.iter().enumerate() {
        let mut d = digit;
        // Double every second digit from the right (starting from second-to-last)
        if (len - i) % 2 == 0 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }

    sum % 10 == 0
}

/// Create a new security
#[command]
pub fn create_security(data: CreateSecurityRequest) -> Result<SecurityResult, String> {
    // Validate ISIN if provided
    if let Some(ref isin) = data.isin {
        if !isin.is_empty() && !validate_isin(isin) {
            return Err(format!("Invalid ISIN checksum: {}", isin));
        }
    }

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check for duplicates
    if let Some(ref isin) = data.isin {
        if !isin.is_empty() {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pp_security WHERE isin = ?1)",
                    params![isin],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if exists {
                return Err(format!("Security with ISIN {} already exists", isin));
            }
        }
    }

    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO pp_security (uuid, name, currency, isin, wkn, ticker, feed, feed_url, latest_feed, latest_feed_url, note, is_retired, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12)
        "#,
        params![
            uuid,
            data.name,
            data.currency,
            data.isin,
            data.wkn,
            data.ticker,
            data.feed,
            data.feed_url,
            data.latest_feed,
            data.latest_feed_url,
            data.note,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(SecurityResult {
        id,
        uuid,
        name: data.name,
        currency: data.currency,
        isin: data.isin,
        wkn: data.wkn,
        ticker: data.ticker,
        feed: data.feed,
        feed_url: data.feed_url,
        latest_feed: data.latest_feed,
        latest_feed_url: data.latest_feed_url,
        note: data.note,
        is_retired: false,
    })
}

/// Update an existing security
#[command]
pub fn update_security(id: i64, data: UpdateSecurityRequest) -> Result<SecurityResult, String> {
    // Validate ISIN if provided
    if let Some(ref isin) = data.isin {
        if !isin.is_empty() && !validate_isin(isin) {
            return Err(format!("Invalid ISIN checksum: {}", isin));
        }
    }

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check if security exists
    let existing: Option<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i32)> = conn
        .query_row(
            "SELECT uuid, name, currency, isin, wkn, ticker, feed, feed_url, latest_feed, latest_feed_url, note, is_retired FROM pp_security WHERE id = ?1",
            params![id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
            )),
        )
        .ok();

    let (uuid, current_name, current_currency, current_isin, current_wkn, current_ticker, current_feed, current_feed_url, current_latest_feed, current_latest_feed_url, current_note, current_retired) =
        existing.ok_or_else(|| format!("Security with id {} not found", id))?;

    // Check for ISIN duplicate if changing ISIN
    if let Some(ref new_isin) = data.isin {
        if !new_isin.is_empty() && Some(new_isin.clone()) != current_isin {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pp_security WHERE isin = ?1 AND id != ?2)",
                    params![new_isin, id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if exists {
                return Err(format!("Security with ISIN {} already exists", new_isin));
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let name = data.name.unwrap_or(current_name);
    let currency = data.currency.unwrap_or(current_currency);

    // For optional fields: Some("") = clear, Some(value) = set, None = keep current
    let isin = match &data.isin {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_isin,
    };
    let wkn = match &data.wkn {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_wkn,
    };
    let ticker = match &data.ticker {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_ticker,
    };
    let feed = match &data.feed {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_feed,
    };
    let feed_url = match &data.feed_url {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_feed_url,
    };
    let latest_feed = match &data.latest_feed {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_latest_feed,
    };
    let latest_feed_url = match &data.latest_feed_url {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_latest_feed_url,
    };
    let note = match &data.note {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => current_note,
    };
    let is_retired = data.is_retired.map(|b| if b { 1 } else { 0 }).unwrap_or(current_retired);

    conn.execute(
        r#"
        UPDATE pp_security
        SET name = ?1, currency = ?2, isin = ?3, wkn = ?4, ticker = ?5,
            feed = ?6, feed_url = ?7, latest_feed = ?8, latest_feed_url = ?9,
            note = ?10, is_retired = ?11, updated_at = ?12
        WHERE id = ?13
        "#,
        params![name, currency, isin, wkn, ticker, feed, feed_url, latest_feed, latest_feed_url, note, is_retired, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(SecurityResult {
        id,
        uuid,
        name,
        currency,
        isin,
        wkn,
        ticker,
        feed,
        feed_url,
        latest_feed,
        latest_feed_url,
        note,
        is_retired: is_retired != 0,
    })
}

/// Delete a security (with referential integrity check)
#[command]
pub fn delete_security(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check for existing transactions
    let txn_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_txn WHERE security_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if txn_count > 0 {
        return Err(format!(
            "Cannot delete security: {} transactions reference this security. Mark as retired instead.",
            txn_count
        ));
    }

    // Check for FIFO lots
    let lot_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_fifo_lot WHERE security_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if lot_count > 0 {
        return Err(format!(
            "Cannot delete security: {} FIFO lots exist for this security. Mark as retired instead.",
            lot_count
        ));
    }

    // Delete prices first
    conn.execute("DELETE FROM pp_price WHERE security_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM pp_latest_price WHERE security_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // Delete security events
    conn.execute("DELETE FROM pp_security_event WHERE security_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // Delete watchlist entries
    conn.execute("DELETE FROM pp_watchlist_security WHERE security_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // Delete the security
    let rows = conn
        .execute("DELETE FROM pp_security WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err(format!("Security with id {} not found", id));
    }

    Ok(())
}

/// Search securities by name, ISIN, WKN, or ticker
#[command]
pub fn search_securities(
    query: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<SecurityResult>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let search_term = format!("%{}%", query);
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, uuid, name, currency, isin, wkn, ticker, feed, feed_url, latest_feed, latest_feed_url, note, is_retired
            FROM pp_security
            WHERE name LIKE ?1 OR isin LIKE ?1 OR wkn LIKE ?1 OR ticker LIKE ?1
            ORDER BY name
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .map_err(|e| e.to_string())?;

    let securities: Vec<SecurityResult> = stmt
        .query_map(params![search_term, limit, offset], |row| {
            Ok(SecurityResult {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                currency: row.get(3)?,
                isin: row.get(4)?,
                wkn: row.get(5)?,
                ticker: row.get(6)?,
                feed: row.get(7)?,
                feed_url: row.get(8)?,
                latest_feed: row.get(9)?,
                latest_feed_url: row.get(10)?,
                note: row.get(11)?,
                is_retired: row.get::<_, i32>(12)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(securities)
}

/// Get a single security by ID
#[command]
pub fn get_security(id: i64) -> Result<SecurityResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.query_row(
        r#"
        SELECT id, uuid, name, currency, isin, wkn, ticker, feed, feed_url, latest_feed, latest_feed_url, note, is_retired
        FROM pp_security
        WHERE id = ?1
        "#,
        params![id],
        |row| {
            Ok(SecurityResult {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                currency: row.get(3)?,
                isin: row.get(4)?,
                wkn: row.get(5)?,
                ticker: row.get(6)?,
                feed: row.get(7)?,
                feed_url: row.get(8)?,
                latest_feed: row.get(9)?,
                latest_feed_url: row.get(10)?,
                note: row.get(11)?,
                is_retired: row.get::<_, i32>(12)? != 0,
            })
        },
    )
    .map_err(|e| format!("Security not found: {}", e))
}

// =============================================================================
// Account CRUD
// =============================================================================

/// Input data for creating a new account
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub name: String,
    pub currency: String,
    pub note: Option<String>,
}

/// Input data for updating an account
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub currency: Option<String>,
    pub note: Option<String>,
    pub is_retired: Option<bool>,
}

/// Account data returned after create/update
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResult {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub currency: String,
    pub note: Option<String>,
    pub is_retired: bool,
}

/// Create a new account
#[command]
pub fn create_account(data: CreateAccountRequest) -> Result<AccountResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO pp_account (uuid, name, currency, note, is_retired, updated_at)
        VALUES (?1, ?2, ?3, ?4, 0, ?5)
        "#,
        params![uuid, data.name, data.currency, data.note, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(AccountResult {
        id,
        uuid,
        name: data.name,
        currency: data.currency,
        note: data.note,
        is_retired: false,
    })
}

/// Update an existing account
#[command]
pub fn update_account(id: i64, data: UpdateAccountRequest) -> Result<AccountResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get current values
    let existing: Option<(String, String, String, Option<String>, i32)> = conn
        .query_row(
            "SELECT uuid, name, currency, note, is_retired FROM pp_account WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .ok();

    let (uuid, current_name, current_currency, current_note, current_retired) =
        existing.ok_or_else(|| format!("Account with id {} not found", id))?;

    let now = chrono::Utc::now().to_rfc3339();
    let name = data.name.unwrap_or(current_name);
    let currency = data.currency.unwrap_or(current_currency);
    let note = data.note.or(current_note);
    let is_retired = data.is_retired.map(|b| if b { 1 } else { 0 }).unwrap_or(current_retired);

    conn.execute(
        r#"
        UPDATE pp_account
        SET name = ?1, currency = ?2, note = ?3, is_retired = ?4, updated_at = ?5
        WHERE id = ?6
        "#,
        params![name, currency, note, is_retired, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(AccountResult {
        id,
        uuid,
        name,
        currency,
        note,
        is_retired: is_retired != 0,
    })
}

/// Delete an account (with referential integrity check)
#[command]
pub fn delete_account(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check for existing transactions
    let txn_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'account' AND owner_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if txn_count > 0 {
        return Err(format!(
            "Cannot delete account: {} transactions exist. Mark as retired instead.",
            txn_count
        ));
    }

    // Check for portfolios using this as reference account
    let portfolio_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_portfolio WHERE reference_account_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if portfolio_count > 0 {
        return Err(format!(
            "Cannot delete account: {} portfolios use this as reference account.",
            portfolio_count
        ));
    }

    let rows = conn
        .execute("DELETE FROM pp_account WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err(format!("Account with id {} not found", id));
    }

    Ok(())
}

// =============================================================================
// Portfolio CRUD
// =============================================================================

/// Input data for creating a new portfolio
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePortfolioRequest {
    pub name: String,
    pub reference_account_id: Option<i64>,
    pub note: Option<String>,
}

/// Input data for updating a portfolio
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePortfolioRequest {
    pub name: Option<String>,
    pub reference_account_id: Option<i64>,
    pub note: Option<String>,
    pub is_retired: Option<bool>,
}

/// Portfolio data returned after create/update
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioResult {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub reference_account_id: Option<i64>,
    pub note: Option<String>,
    pub is_retired: bool,
}

/// Create a new portfolio
#[command]
pub fn create_pp_portfolio_new(data: CreatePortfolioRequest) -> Result<PortfolioResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Validate reference account if provided
    if let Some(ref_id) = data.reference_account_id {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pp_account WHERE id = ?1)",
                params![ref_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if !exists {
            return Err(format!("Reference account with id {} not found", ref_id));
        }
    }

    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO pp_portfolio (uuid, name, reference_account_id, note, is_retired, updated_at)
        VALUES (?1, ?2, ?3, ?4, 0, ?5)
        "#,
        params![uuid, data.name, data.reference_account_id, data.note, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(PortfolioResult {
        id,
        uuid,
        name: data.name,
        reference_account_id: data.reference_account_id,
        note: data.note,
        is_retired: false,
    })
}

/// Update an existing portfolio
#[command]
pub fn update_pp_portfolio(id: i64, data: UpdatePortfolioRequest) -> Result<PortfolioResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get current values
    let existing: Option<(String, String, Option<i64>, Option<String>, i32)> = conn
        .query_row(
            "SELECT uuid, name, reference_account_id, note, is_retired FROM pp_portfolio WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .ok();

    let (uuid, current_name, current_ref_account, current_note, current_retired) =
        existing.ok_or_else(|| format!("Portfolio with id {} not found", id))?;

    // Validate new reference account if provided
    if let Some(ref_id) = data.reference_account_id {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pp_account WHERE id = ?1)",
                params![ref_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if !exists {
            return Err(format!("Reference account with id {} not found", ref_id));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let name = data.name.unwrap_or(current_name);
    let reference_account_id = data.reference_account_id.or(current_ref_account);
    let note = data.note.or(current_note);
    let is_retired = data.is_retired.map(|b| if b { 1 } else { 0 }).unwrap_or(current_retired);

    conn.execute(
        r#"
        UPDATE pp_portfolio
        SET name = ?1, reference_account_id = ?2, note = ?3, is_retired = ?4, updated_at = ?5
        WHERE id = ?6
        "#,
        params![name, reference_account_id, note, is_retired, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(PortfolioResult {
        id,
        uuid,
        name,
        reference_account_id,
        note,
        is_retired: is_retired != 0,
    })
}

/// Delete a portfolio (with referential integrity check)
#[command]
pub fn delete_pp_portfolio(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check for existing transactions
    let txn_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if txn_count > 0 {
        return Err(format!(
            "Cannot delete portfolio: {} transactions exist. Mark as retired instead.",
            txn_count
        ));
    }

    // Check for FIFO lots
    let lot_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_fifo_lot WHERE portfolio_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if lot_count > 0 {
        return Err(format!(
            "Cannot delete portfolio: {} FIFO lots exist. Mark as retired instead.",
            lot_count
        ));
    }

    let rows = conn
        .execute("DELETE FROM pp_portfolio WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err(format!("Portfolio with id {} not found", id));
    }

    Ok(())
}

// =============================================================================
// Transaction CRUD
// =============================================================================

/// Transaction unit (fee, tax, or forex)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionUnitData {
    pub unit_type: String, // FEE, TAX, GROSS_VALUE
    pub amount: i64,       // × 10²
    pub currency: String,
    pub forex_amount: Option<i64>,
    pub forex_currency: Option<String>,
    pub exchange_rate: Option<f64>,
}

/// Input data for creating a transaction
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransactionRequest {
    pub owner_type: String,        // "account" | "portfolio"
    pub owner_id: i64,
    pub txn_type: String,          // BUY, SELL, DIVIDEND, etc.
    pub date: String,              // ISO date string
    pub amount: i64,               // × 10²
    pub currency: String,
    pub shares: Option<i64>,       // × 10⁸
    pub security_id: Option<i64>,
    pub note: Option<String>,
    pub units: Option<Vec<TransactionUnitData>>,
    // For portfolio BUY/SELL: reference account for the cash transaction
    pub reference_account_id: Option<i64>,
}

/// Transaction data returned after create/update
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResult {
    pub id: i64,
    pub uuid: String,
    pub owner_type: String,
    pub owner_id: i64,
    pub txn_type: String,
    pub date: String,
    pub amount: f64,
    pub currency: String,
    pub shares: Option<f64>,
    pub security_id: Option<i64>,
    pub note: Option<String>,
    pub cross_entry_id: Option<i64>,
}

/// Scale factors
const SHARES_SCALE: f64 = 100_000_000.0;
const AMOUNT_SCALE: f64 = 100.0;

/// Create a new transaction
/// For portfolio BUY/SELL, also creates a matching account transaction and cross-entry
#[command]
pub fn create_transaction(data: CreateTransactionRequest) -> Result<TransactionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Validate owner exists
    let owner_exists: bool = if data.owner_type == "account" {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pp_account WHERE id = ?1)",
            params![data.owner_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?
    } else {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pp_portfolio WHERE id = ?1)",
            params![data.owner_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?
    };

    if !owner_exists {
        return Err(format!(
            "{} with id {} not found",
            if data.owner_type == "account" { "Account" } else { "Portfolio" },
            data.owner_id
        ));
    }

    // Validate security if provided
    if let Some(sec_id) = data.security_id {
        let sec_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pp_security WHERE id = ?1)",
                params![sec_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if !sec_exists {
            return Err(format!("Security with id {} not found", sec_id));
        }
    }

    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // For portfolio BUY/SELL, we need to create a cross-entry with account transaction
    let cross_entry_id = if data.owner_type == "portfolio"
        && (data.txn_type == "BUY" || data.txn_type == "SELL")
    {
        // Get reference account - either from request or from portfolio
        let ref_account_id: i64 = if let Some(id) = data.reference_account_id {
            id
        } else {
            conn.query_row(
                "SELECT reference_account_id FROM pp_portfolio WHERE id = ?1",
                params![data.owner_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Portfolio has no reference account. Please specify reference_account_id.".to_string())?
        };

        // Create cross-entry
        let cross_uuid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO pp_cross_entry (uuid, entry_type, source) VALUES (?1, 'BUY_SELL', 'manual')",
            params![cross_uuid],
        )
        .map_err(|e| e.to_string())?;

        let ce_id = conn.last_insert_rowid();

        // Create account transaction (opposite of portfolio transaction)
        let account_txn_uuid = Uuid::new_v4().to_string();
        let account_txn_type = &data.txn_type; // Same type for account

        conn.execute(
            r#"
            INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note, cross_entry_id, updated_at)
            VALUES (?1, 'account', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                account_txn_uuid,
                ref_account_id,
                account_txn_type,
                data.date,
                data.amount,
                data.currency,
                data.shares,
                data.security_id,
                data.note,
                ce_id,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        let account_txn_id = conn.last_insert_rowid();

        // Update cross-entry with account transaction id
        conn.execute(
            "UPDATE pp_cross_entry SET account_txn_id = ?1 WHERE id = ?2",
            params![account_txn_id, ce_id],
        )
        .map_err(|e| e.to_string())?;

        Some(ce_id)
    } else {
        None
    };

    // Create the main transaction
    conn.execute(
        r#"
        INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note, cross_entry_id, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            uuid,
            data.owner_type,
            data.owner_id,
            data.txn_type,
            data.date,
            data.amount,
            data.currency,
            data.shares,
            data.security_id,
            data.note,
            cross_entry_id,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Update cross-entry with portfolio transaction id (if BUY/SELL)
    if let Some(ce_id) = cross_entry_id {
        conn.execute(
            "UPDATE pp_cross_entry SET portfolio_txn_id = ?1 WHERE id = ?2",
            params![id, ce_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Insert transaction units (fees, taxes)
    if let Some(units) = &data.units {
        for unit in units {
            conn.execute(
                r#"
                INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency, forex_amount, forex_currency, exchange_rate)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    id,
                    unit.unit_type,
                    unit.amount,
                    unit.currency,
                    unit.forex_amount,
                    unit.forex_currency,
                    unit.exchange_rate,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Rebuild FIFO lots if this affects a security in a portfolio
    if data.owner_type == "portfolio" && data.security_id.is_some() {
        let security_id = data.security_id.unwrap();
        if let Err(e) = crate::fifo::build_fifo_lots(conn, security_id) {
            log::warn!("Failed to rebuild FIFO lots: {}", e);
        }
    }

    Ok(TransactionResult {
        id,
        uuid,
        owner_type: data.owner_type,
        owner_id: data.owner_id,
        txn_type: data.txn_type,
        date: data.date,
        amount: data.amount as f64 / AMOUNT_SCALE,
        currency: data.currency,
        shares: data.shares.map(|s| s as f64 / SHARES_SCALE),
        security_id: data.security_id,
        note: data.note,
        cross_entry_id,
    })
}

/// Delete a transaction
/// Also deletes linked cross-entry and account transaction if applicable
#[command]
pub fn delete_transaction(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get transaction info for FIFO rebuild
    let txn_info: Option<(String, i64, Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT owner_type, owner_id, security_id, cross_entry_id FROM pp_txn WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    let (owner_type, _owner_id, security_id, cross_entry_id) =
        txn_info.ok_or_else(|| format!("Transaction with id {} not found", id))?;

    // Delete transaction units first
    conn.execute("DELETE FROM pp_txn_unit WHERE txn_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // If there's a cross-entry, delete the linked transaction too
    if let Some(ce_id) = cross_entry_id {
        // Get the other transaction id from the cross-entry
        let other_txn_id: Option<i64> = conn
            .query_row(
                r#"
                SELECT CASE
                    WHEN portfolio_txn_id = ?1 THEN account_txn_id
                    WHEN account_txn_id = ?1 THEN portfolio_txn_id
                    WHEN from_txn_id = ?1 THEN to_txn_id
                    WHEN to_txn_id = ?1 THEN from_txn_id
                    ELSE NULL
                END
                FROM pp_cross_entry WHERE id = ?2
                "#,
                params![id, ce_id],
                |row| row.get(0),
            )
            .ok();

        // Delete the other transaction's units and the transaction itself
        if let Some(other_id) = other_txn_id {
            conn.execute("DELETE FROM pp_txn_unit WHERE txn_id = ?1", params![other_id])
                .map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM pp_txn WHERE id = ?1", params![other_id])
                .map_err(|e| e.to_string())?;
        }

        // Delete the cross-entry
        conn.execute("DELETE FROM pp_cross_entry WHERE id = ?1", params![ce_id])
            .map_err(|e| e.to_string())?;
    }

    // Delete the main transaction
    conn.execute("DELETE FROM pp_txn WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // Rebuild FIFO lots if this was a portfolio transaction with a security
    if owner_type == "portfolio" && security_id.is_some() {
        let sec_id = security_id.unwrap();
        if let Err(e) = crate::fifo::build_fifo_lots(conn, sec_id) {
            log::warn!("Failed to rebuild FIFO lots: {}", e);
        }
    }

    Ok(())
}

/// Update transaction request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTransactionRequest {
    pub date: Option<String>,
    pub amount: Option<i64>,         // cents
    pub shares: Option<i64>,         // scaled by 10^8
    pub note: Option<String>,
    pub fee_amount: Option<i64>,     // cents
    pub tax_amount: Option<i64>,     // cents
}

/// Update an existing transaction
/// Only updates date, amount, shares, note, and units (fees/taxes)
/// Does not allow changing owner, type, or security (delete and recreate instead)
#[command]
pub fn update_transaction(id: i64, data: UpdateTransactionRequest) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Verify transaction exists and get info for FIFO rebuild
    let txn_info: Option<(String, i64, Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT owner_type, owner_id, security_id, cross_entry_id FROM pp_txn WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    let (owner_type, _owner_id, security_id, cross_entry_id) =
        txn_info.ok_or_else(|| format!("Transaction with id {} not found", id))?;

    // Build update query dynamically
    let mut updates = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(date) = &data.date {
        updates.push("date = ?");
        params_vec.push(Box::new(date.clone()));
    }
    if let Some(amount) = data.amount {
        updates.push("amount = ?");
        params_vec.push(Box::new(amount));
    }
    if let Some(shares) = data.shares {
        updates.push("shares = ?");
        params_vec.push(Box::new(shares));
    }
    if data.note.is_some() {
        updates.push("note = ?");
        params_vec.push(Box::new(data.note.clone()));
    }

    // Always update updated_at
    updates.push("updated_at = datetime('now')");

    if !updates.is_empty() {
        let sql = format!(
            "UPDATE pp_txn SET {} WHERE id = ?",
            updates.join(", ")
        );
        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())
            .map_err(|e| e.to_string())?;
    }

    // Update fees and taxes in transaction units
    // First, delete existing FEE and TAX units
    conn.execute(
        "DELETE FROM pp_txn_unit WHERE txn_id = ?1 AND unit_type IN ('FEE', 'TAX')",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    // Get currency from transaction
    let currency: String = conn
        .query_row(
            "SELECT currency FROM pp_txn WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Add new FEE unit if specified
    if let Some(fee) = data.fee_amount {
        if fee > 0 {
            conn.execute(
                "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency) VALUES (?1, 'FEE', ?2, ?3)",
                params![id, fee, currency],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Add new TAX unit if specified
    if let Some(tax) = data.tax_amount {
        if tax > 0 {
            conn.execute(
                "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency) VALUES (?1, 'TAX', ?2, ?3)",
                params![id, tax, currency],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // If this is a BUY/SELL with cross-entry, update the linked account transaction too
    if let Some(ce_id) = cross_entry_id {
        let other_txn_id: Option<i64> = conn
            .query_row(
                r#"
                SELECT CASE
                    WHEN portfolio_txn_id = ?1 THEN account_txn_id
                    WHEN account_txn_id = ?1 THEN portfolio_txn_id
                    ELSE NULL
                END
                FROM pp_cross_entry WHERE id = ?2
                "#,
                params![id, ce_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(other_id) = other_txn_id {
            // Update the linked transaction with same date and amount
            let mut other_updates = Vec::new();
            let mut other_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(date) = &data.date {
                other_updates.push("date = ?");
                other_params.push(Box::new(date.clone()));
            }
            if let Some(amount) = data.amount {
                other_updates.push("amount = ?");
                other_params.push(Box::new(amount));
            }
            other_updates.push("updated_at = datetime('now')");

            if !other_updates.is_empty() {
                let sql = format!(
                    "UPDATE pp_txn SET {} WHERE id = ?",
                    other_updates.join(", ")
                );
                other_params.push(Box::new(other_id));

                let params_refs: Vec<&dyn rusqlite::ToSql> = other_params.iter().map(|b| b.as_ref()).collect();
                conn.execute(&sql, params_refs.as_slice())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // Rebuild FIFO lots if this was a portfolio transaction with a security
    if owner_type == "portfolio" && security_id.is_some() {
        let sec_id = security_id.unwrap();
        if let Err(e) = crate::fifo::build_fifo_lots(conn, sec_id) {
            log::warn!("Failed to rebuild FIFO lots after update: {}", e);
        }
    }

    Ok(())
}

/// Get a single transaction by ID
#[command]
pub fn get_transaction(id: i64) -> Result<TransactionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.query_row(
        r#"
        SELECT id, uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note, cross_entry_id
        FROM pp_txn
        WHERE id = ?1
        "#,
        params![id],
        |row| {
            let amount_cents: i64 = row.get(6)?;
            let shares_raw: Option<i64> = row.get(8)?;

            Ok(TransactionResult {
                id: row.get(0)?,
                uuid: row.get(1)?,
                owner_type: row.get(2)?,
                owner_id: row.get(3)?,
                txn_type: row.get(4)?,
                date: row.get(5)?,
                amount: amount_cents as f64 / AMOUNT_SCALE,
                currency: row.get(7)?,
                shares: shares_raw.map(|s| s as f64 / SHARES_SCALE),
                security_id: row.get(9)?,
                note: row.get(10)?,
                cross_entry_id: row.get(11)?,
            })
        },
    )
    .map_err(|e| format!("Transaction not found: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isin_validation() {
        // Valid ISINs
        assert!(validate_isin("US0378331005")); // Apple
        assert!(validate_isin("DE0007164600")); // SAP
        assert!(validate_isin("GB0002634946")); // BAE Systems

        // Invalid ISINs
        assert!(!validate_isin("US0378331006")); // Wrong checksum
        assert!(!validate_isin("US037833100")); // Too short
        assert!(!validate_isin("US03783310055")); // Too long
        assert!(!validate_isin("123456789012")); // Invalid country code
    }
}
