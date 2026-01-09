//! Data query commands for accessing imported Portfolio Performance data.

use crate::currency;
use crate::db;
use crate::pp::common::{prices, shares};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

/// Security data for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityData {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub currency: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub feed: Option<String>,           // Provider for historical quotes
    pub feed_url: Option<String>,       // URL/suffix for historical quotes
    pub latest_feed: Option<String>,    // Provider for current quotes
    pub latest_feed_url: Option<String>, // URL/suffix for current quotes
    pub is_retired: bool,
    pub latest_price: Option<f64>,
    pub latest_price_date: Option<String>,
    pub updated_at: Option<String>, // When the price was last fetched
    pub prices_count: i32,
    pub current_holdings: f64, // Total shares held across all portfolios
    pub custom_logo: Option<String>, // Base64-encoded custom logo
}

/// Get all securities from the database
#[command]
pub fn get_securities(import_id: Option<i64>) -> Result<Vec<SecurityData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Calculate current holdings by summing portfolio transactions
    // BUY, TRANSFER_IN, DELIVERY_INBOUND add shares
    // SELL, TRANSFER_OUT, DELIVERY_OUTBOUND subtract shares
    let sql = if import_id.is_some() {
        "SELECT s.id, s.uuid, s.name, s.currency, s.isin, s.wkn, s.ticker,
                s.feed, s.feed_url, s.latest_feed, s.latest_feed_url, s.is_retired,
                lp.value, lp.date, lp.updated_at,
                (SELECT COUNT(*) FROM pp_price WHERE security_id = s.id) as prices_count,
                COALESCE((
                    SELECT SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END)
                    FROM pp_txn t
                    WHERE t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
                ), 0) as current_holdings,
                s.custom_logo
         FROM pp_security s
         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
         WHERE s.import_id = ?1
         ORDER BY s.name"
    } else {
        "SELECT s.id, s.uuid, s.name, s.currency, s.isin, s.wkn, s.ticker,
                s.feed, s.feed_url, s.latest_feed, s.latest_feed_url, s.is_retired,
                lp.value, lp.date, lp.updated_at,
                (SELECT COUNT(*) FROM pp_price WHERE security_id = s.id) as prices_count,
                COALESCE((
                    SELECT SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END)
                    FROM pp_txn t
                    WHERE t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
                ), 0) as current_holdings,
                s.custom_logo
         FROM pp_security s
         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
         ORDER BY s.name"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = if let Some(id) = import_id {
        stmt.query(params![id])
    } else {
        stmt.query([])
    }
    .map_err(|e| e.to_string())?;

    // Column indices:
    // 0: id, 1: uuid, 2: name, 3: currency, 4: isin, 5: wkn, 6: ticker,
    // 7: feed, 8: feed_url, 9: latest_feed, 10: latest_feed_url, 11: is_retired,
    // 12: lp.value, 13: lp.date, 14: lp.updated_at, 15: prices_count, 16: current_holdings, 17: custom_logo
    let securities: Vec<SecurityData> = rows
        .mapped(|row| {
            let latest_value: Option<i64> = row.get(12)?;
            let holdings_raw: i64 = row.get(16)?;
            Ok(SecurityData {
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
                is_retired: row.get::<_, i32>(11)? != 0,
                latest_price: latest_value.map(prices::to_decimal),
                latest_price_date: row.get(13)?,
                updated_at: row.get(14)?,
                prices_count: row.get(15)?,
                current_holdings: shares::to_decimal(holdings_raw),
                custom_logo: row.get(17)?,
            })
        })
        .filter_map(|r| r.ok())
        .collect();

    Ok(securities)
}

/// Account data for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub currency: String,
    pub is_retired: bool,
    pub transactions_count: i32,
    pub balance: f64,
}

/// Get all accounts from the database
#[command]
pub fn get_accounts(import_id: Option<i64>) -> Result<Vec<AccountData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let sql = if import_id.is_some() {
        "SELECT a.id, a.uuid, a.name, a.currency, a.is_retired,
                (SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'account' AND owner_id = a.id) as txn_count,
                COALESCE((SELECT SUM(
                    CASE
                        WHEN txn_type IN ('DEPOSIT', 'INTEREST', 'DIVIDENDS', 'TAX_REFUND', 'FEES_REFUND', 'TRANSFER_IN') THEN amount
                        WHEN txn_type IN ('REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE', 'TRANSFER_OUT') THEN -amount
                        WHEN txn_type = 'BUY' THEN -amount
                        WHEN txn_type = 'SELL' THEN amount
                        ELSE 0
                    END
                ) FROM pp_txn WHERE owner_type = 'account' AND owner_id = a.id), 0) as balance
         FROM pp_account a
         WHERE a.import_id = ?1
         ORDER BY a.name"
    } else {
        "SELECT a.id, a.uuid, a.name, a.currency, a.is_retired,
                (SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'account' AND owner_id = a.id) as txn_count,
                COALESCE((SELECT SUM(
                    CASE
                        WHEN txn_type IN ('DEPOSIT', 'INTEREST', 'DIVIDENDS', 'TAX_REFUND', 'FEES_REFUND', 'TRANSFER_IN') THEN amount
                        WHEN txn_type IN ('REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE', 'TRANSFER_OUT') THEN -amount
                        WHEN txn_type = 'BUY' THEN -amount
                        WHEN txn_type = 'SELL' THEN amount
                        ELSE 0
                    END
                ) FROM pp_txn WHERE owner_type = 'account' AND owner_id = a.id), 0) as balance
         FROM pp_account a
         ORDER BY a.name"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = if let Some(id) = import_id {
        stmt.query(params![id])
    } else {
        stmt.query([])
    }
    .map_err(|e| e.to_string())?;

    let accounts: Vec<AccountData> = rows
        .mapped(|row| {
            let balance_cents: i64 = row.get(6)?;
            Ok(AccountData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                currency: row.get(3)?,
                is_retired: row.get::<_, i32>(4)? != 0,
                transactions_count: row.get(5)?,
                balance: balance_cents as f64 / 100.0,
            })
        })
        .filter_map(|r| r.ok())
        .collect();

    Ok(accounts)
}

/// Portfolio data for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioData {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub reference_account_name: Option<String>,
    pub is_retired: bool,
    pub transactions_count: i32,
    pub holdings_count: i32,
}

/// Get all portfolios from the database
#[command]
pub fn get_pp_portfolios(import_id: Option<i64>) -> Result<Vec<PortfolioData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let sql = if import_id.is_some() {
        "SELECT p.id, p.uuid, p.name, ra.name as ref_account_name, p.is_retired,
                (SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = p.id) as txn_count,
                (SELECT COUNT(DISTINCT security_id) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = p.id AND security_id IS NOT NULL) as holdings_count
         FROM pp_portfolio p
         LEFT JOIN pp_account ra ON ra.id = p.reference_account_id
         WHERE p.import_id = ?1
         ORDER BY p.name"
    } else {
        "SELECT p.id, p.uuid, p.name, ra.name as ref_account_name, p.is_retired,
                (SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = p.id) as txn_count,
                (SELECT COUNT(DISTINCT security_id) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = p.id AND security_id IS NOT NULL) as holdings_count
         FROM pp_portfolio p
         LEFT JOIN pp_account ra ON ra.id = p.reference_account_id
         ORDER BY p.name"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = if let Some(id) = import_id {
        stmt.query(params![id])
    } else {
        stmt.query([])
    }
    .map_err(|e| e.to_string())?;

    let portfolios: Vec<PortfolioData> = rows
        .mapped(|row| {
            Ok(PortfolioData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                reference_account_name: row.get(3)?,
                is_retired: row.get::<_, i32>(4)? != 0,
                transactions_count: row.get(5)?,
                holdings_count: row.get(6)?,
            })
        })
        .filter_map(|r| r.ok())
        .collect();

    Ok(portfolios)
}

/// Transaction data for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionData {
    pub id: i64,
    pub uuid: String,
    pub owner_type: String,
    pub owner_name: String,
    pub txn_type: String,
    pub date: String,
    pub amount: f64,
    pub currency: String,
    pub shares: Option<f64>,
    pub security_name: Option<String>,
    pub security_uuid: Option<String>,
    pub note: Option<String>,
    pub fees: f64,
    pub taxes: f64,
    pub has_forex: bool,
}

/// Get transactions with optional filters
#[command]
pub fn get_transactions(
    owner_type: Option<String>,
    owner_id: Option<i64>,
    security_id: Option<i64>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<TransactionData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut sql = String::from(
        "SELECT t.id, t.uuid, t.owner_type,
                CASE WHEN t.owner_type = 'account' THEN a.name ELSE p.name END as owner_name,
                t.txn_type, t.date, t.amount, t.currency, t.shares,
                s.name as security_name, s.uuid as security_uuid, t.note,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'FEE'), 0) as fees,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes,
                EXISTS(SELECT 1 FROM pp_txn_unit WHERE txn_id = t.id AND forex_amount IS NOT NULL) as has_forex
         FROM pp_txn t
         LEFT JOIN pp_account a ON t.owner_type = 'account' AND a.id = t.owner_id
         LEFT JOIN pp_portfolio p ON t.owner_type = 'portfolio' AND p.id = t.owner_id
         LEFT JOIN pp_security s ON s.id = t.security_id
         WHERE 1=1",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref ot) = owner_type {
        sql.push_str(&format!(" AND t.owner_type = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(ot.clone()));
    }

    if let Some(oid) = owner_id {
        sql.push_str(&format!(" AND t.owner_id = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(oid));
    }

    if let Some(sid) = security_id {
        sql.push_str(&format!(" AND t.security_id = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(sid));
    }

    sql.push_str(" ORDER BY t.date DESC");

    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(l));
    }

    if let Some(o) = offset {
        sql.push_str(&format!(" OFFSET ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(o));
    }

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query(params_refs.as_slice())
        .map_err(|e| e.to_string())?;

    let transactions: Vec<TransactionData> = rows
        .mapped(|row| {
            let amount_cents: i64 = row.get(6)?;
            let shares_raw: Option<i64> = row.get(8)?;
            let fees_cents: i64 = row.get(12)?;
            let taxes_cents: i64 = row.get(13)?;

            Ok(TransactionData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                owner_type: row.get(2)?,
                owner_name: row.get(3)?,
                txn_type: row.get(4)?,
                date: row.get(5)?,
                amount: amount_cents as f64 / 100.0,
                currency: row.get(7)?,
                shares: shares_raw.map(shares::to_decimal),
                security_name: row.get(9)?,
                security_uuid: row.get(10)?,
                note: row.get(11)?,
                fees: fees_cents as f64 / 100.0,
                taxes: taxes_cents as f64 / 100.0,
                has_forex: row.get::<_, i32>(14)? != 0,
            })
        })
        .filter_map(|r| r.ok())
        .collect();

    Ok(transactions)
}

/// Price history data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    pub date: String,
    pub value: f64,
}

/// Get price history for a security
#[command]
pub fn get_price_history(
    security_id: i64,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<PriceData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut sql =
        String::from("SELECT date, value FROM pp_price WHERE security_id = ?1");

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(security_id)];

    if let Some(ref s) = start_date {
        sql.push_str(&format!(" AND date >= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(s.clone()));
    }
    if let Some(ref e) = end_date {
        sql.push_str(&format!(" AND date <= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(e.clone()));
    }
    sql.push_str(" ORDER BY date");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut prices = Vec::new();
    let mut rows = stmt.query(params_refs.as_slice()).map_err(|e| e.to_string())?;

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let value_raw: i64 = row.get(1).map_err(|e| e.to_string())?;
        prices.push(PriceData {
            date: row.get(0).map_err(|e| e.to_string())?,
            value: prices::to_decimal(value_raw),
        });
    }

    Ok(prices)
}

/// Holdings data (current position in a portfolio)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingData {
    pub security_id: i64,
    pub security_uuid: String,
    pub security_name: String,
    pub currency: String,
    pub shares: f64,
    pub current_price: Option<f64>,
    pub current_value: Option<f64>,
    pub cost_basis: f64,
    pub gain_loss: Option<f64>,
    pub gain_loss_percent: Option<f64>,
}

/// Get holdings for a portfolio
#[command]
pub fn get_holdings(portfolio_id: i64) -> Result<Vec<HoldingData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let sql = "
        WITH holdings AS (
            SELECT
                t.security_id,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) as total_shares,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.amount
                    ELSE 0
                END) as cost_basis
            FROM pp_txn t
            WHERE t.owner_type = 'portfolio'
              AND t.owner_id = ?1
              AND t.security_id IS NOT NULL
            GROUP BY t.security_id
            HAVING total_shares > 0
        )
        SELECT
            h.security_id,
            s.uuid,
            s.name,
            s.currency,
            h.total_shares,
            lp.value as latest_price,
            h.cost_basis
        FROM holdings h
        JOIN pp_security s ON s.id = h.security_id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        ORDER BY s.name
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let holdings: Vec<HoldingData> = stmt
        .query_map(params![portfolio_id], |row| {
            let shares_raw: i64 = row.get(4)?;
            let latest_price_raw: Option<i64> = row.get(5)?;
            let cost_basis_cents: i64 = row.get(6)?;

            let shares_decimal = shares::to_decimal(shares_raw);
            let current_price = latest_price_raw.map(prices::to_decimal);
            let current_value = current_price.map(|p| p * shares_decimal);
            let cost_basis = cost_basis_cents as f64 / 100.0;

            let gain_loss = current_value.map(|v| v - cost_basis);
            let gain_loss_percent = if cost_basis > 0.0 {
                gain_loss.map(|g| (g / cost_basis) * 100.0)
            } else {
                None
            };

            Ok(HoldingData {
                security_id: row.get(0)?,
                security_uuid: row.get(1)?,
                security_name: row.get(2)?,
                currency: row.get(3)?,
                shares: shares_decimal,
                current_price,
                current_value,
                cost_basis,
                gain_loss,
                gain_loss_percent,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(holdings)
}

/// Holdings per portfolio for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHolding {
    pub portfolio_name: String,
    pub shares: f64,
    pub value: Option<f64>,
}

/// Holdings aggregated by ISIN across all portfolios
/// Implements Portfolio Performance's Vermögensaufstellung columns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedHolding {
    pub isin: String,
    pub name: String,
    pub currency: String,
    pub security_id: i64,
    pub total_shares: f64,
    pub current_price: Option<f64>,
    pub current_value: Option<f64>,
    /// Einstandswert (total cost basis from FIFO)
    pub cost_basis: f64,
    /// Einstandskurs (cost per share = cost_basis / shares)
    pub purchase_price: Option<f64>,
    /// Gewinn/Verlust (unrealized gain/loss)
    pub gain_loss: Option<f64>,
    /// Abs.Perf. % Seit (performance percentage)
    pub gain_loss_percent: Option<f64>,
    /// ΣDiv Seit (total dividends received for this position)
    pub dividends_total: f64,
    pub portfolios: Vec<PortfolioHolding>,
    pub custom_logo: Option<String>,
}

/// Get all holdings aggregated by ISIN across all portfolios
/// Uses transaction sums (like Portfolio Performance's PortfolioSnapshot.java)
/// NOT FIFO lots - FIFO is only for cost basis calculation!
#[command]
pub fn get_all_holdings() -> Result<Vec<AggregatedHolding>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get base currency for value conversion
    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let today = Utc::now().date_naive();

    // STEP 1: Calculate holdings using transaction sums (PP PortfolioSnapshot logic)
    // shares = SUM(BUY/TRANSFER_IN/DELIVERY_INBOUND) - SUM(SELL/TRANSFER_OUT/DELIVERY_OUTBOUND)
    let holdings_sql = "
        WITH portfolio_holdings AS (
            SELECT
                COALESCE(s.isin, s.uuid) as identifier,
                MAX(s.id) as security_id,
                MAX(s.name) as name,
                MAX(s.currency) as currency,
                MAX(s.custom_logo) as custom_logo,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) as net_shares
            FROM pp_txn t
            JOIN pp_portfolio p ON p.id = t.owner_id AND t.owner_type = 'portfolio'
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.shares IS NOT NULL
            GROUP BY COALESCE(s.isin, s.uuid)
            HAVING net_shares > 0
        )
        SELECT
            ph.identifier,
            ph.name,
            ph.currency,
            ph.security_id,
            ph.net_shares,
            lp.value as latest_price,
            ph.custom_logo
        FROM portfolio_holdings ph
        LEFT JOIN pp_latest_price lp ON lp.security_id = ph.security_id
        ORDER BY ph.net_shares * COALESCE(lp.value, 0) DESC
    ";

    let mut holdings_stmt = conn.prepare(holdings_sql).map_err(|e| e.to_string())?;

    // STEP 2: Get cost basis from FIFO lots (separate from share count!)
    // Include currency to enable conversion to base currency
    // Use gross_amount (purchase price INCLUDING fees/taxes) for cost basis, matching PP behavior
    // PP CostCalculation.java: Purchase Value = gross amount with all fees and taxes
    let cost_basis_sql = "
        SELECT
            COALESCE(s.isin, s.uuid) as identifier,
            MAX(l.currency) as lot_currency,
            SUM(CASE
                WHEN l.original_shares > 0 THEN
                    (l.remaining_shares * l.gross_amount / l.original_shares)
                ELSE 0
            END) as cost_basis
        FROM pp_fifo_lot l
        JOIN pp_security s ON l.security_id = s.id
        WHERE l.remaining_shares > 0
        GROUP BY COALESCE(s.isin, s.uuid)
    ";

    // Map identifier -> (cost_basis_cents, currency)
    let mut cost_basis_map: std::collections::HashMap<String, (i64, String)> = std::collections::HashMap::new();
    let mut cost_stmt = conn.prepare(cost_basis_sql).map_err(|e| e.to_string())?;
    let cost_rows = cost_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,  // identifier
                row.get::<_, String>(1)?,  // currency
                row.get::<_, i64>(2)?,     // cost_basis
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in cost_rows {
        if let Ok((identifier, lot_currency, cost)) = row {
            cost_basis_map.insert(identifier, (cost, lot_currency));
        }
    }

    // STEP 3: Get dividend sums per security (ΣDiv Seit)
    // Sum all DIVIDENDS transactions for each security across all accounts
    let dividends_sql = "
        SELECT
            COALESCE(s.isin, s.uuid) as identifier,
            t.currency,
            SUM(t.amount) as dividend_total
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.txn_type = 'DIVIDENDS'
          AND t.owner_type = 'account'
        GROUP BY COALESCE(s.isin, s.uuid)
    ";

    // Map identifier -> (dividend_total_cents, currency)
    let mut dividends_map: std::collections::HashMap<String, (i64, String)> = std::collections::HashMap::new();
    let mut div_stmt = conn.prepare(dividends_sql).map_err(|e| e.to_string())?;
    let div_rows = div_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,  // identifier
                row.get::<_, String>(1)?,  // currency
                row.get::<_, i64>(2)?,     // dividend_total
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in div_rows {
        if let Ok((identifier, div_currency, total)) = row {
            dividends_map.insert(identifier, (total, div_currency));
        }
    }

    // Build holdings with transaction-based shares and FIFO-based cost basis
    // First, collect raw data
    let raw_holdings: Vec<_> = holdings_stmt
        .query_map([], |row| {
            let identifier: String = row.get(0)?;
            let name: String = row.get(1)?;
            let currency: String = row.get(2)?;
            let security_id: i64 = row.get(3)?;
            let shares_raw: i64 = row.get(4)?;
            let latest_price_raw: Option<i64> = row.get(5)?;
            let custom_logo: Option<String> = row.get(6)?;

            Ok((identifier, name, currency, security_id, shares_raw, latest_price_raw, custom_logo))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Then process with currency conversion
    let mut holdings: Vec<AggregatedHolding> = raw_holdings
        .into_iter()
        .map(|(identifier, name, security_currency, security_id, shares_raw, latest_price_raw, custom_logo)| {
            let total_shares = shares::to_decimal(shares_raw);
            let current_price = latest_price_raw.map(prices::to_decimal);

            // Calculate value in security's currency first
            let value_in_security_currency = current_price.map(|p| p * total_shares);

            // Convert to base currency if different
            let current_value = if security_currency == base_currency {
                value_in_security_currency
            } else {
                value_in_security_currency.map(|v| {
                    currency::convert(conn, v, &security_currency, &base_currency, today)
                        .unwrap_or(v) // Fallback to unconverted if no rate found
                })
            };

            // Get cost basis from FIFO map and convert to base currency
            let (cost_basis_cents, lot_currency) = cost_basis_map
                .get(&identifier)
                .cloned()
                .unwrap_or((0, base_currency.clone()));
            let cost_basis_raw = cost_basis_cents as f64 / 100.0;

            // Convert cost basis to base currency if needed
            let cost_basis = if lot_currency == base_currency {
                cost_basis_raw
            } else {
                currency::convert(conn, cost_basis_raw, &lot_currency, &base_currency, today)
                    .unwrap_or(cost_basis_raw)
            };

            // Calculate purchase price per share (Einstandskurs)
            let purchase_price = if total_shares > 0.0 {
                Some(cost_basis / total_shares)
            } else {
                None
            };

            let gain_loss = current_value.map(|v| v - cost_basis);
            let gain_loss_percent = if cost_basis > 0.0 {
                gain_loss.map(|g| (g / cost_basis) * 100.0)
            } else {
                None
            };

            // Get dividend total from map and convert to base currency
            let (div_cents, div_currency) = dividends_map
                .get(&identifier)
                .cloned()
                .unwrap_or((0, base_currency.clone()));
            let div_raw = div_cents as f64 / 100.0;
            let dividends_total = if div_currency == base_currency {
                div_raw
            } else {
                currency::convert(conn, div_raw, &div_currency, &base_currency, today)
                    .unwrap_or(div_raw)
            };

            AggregatedHolding {
                isin: identifier,
                name,
                currency: security_currency,
                security_id,
                total_shares,
                current_price,
                current_value,
                cost_basis,
                purchase_price,
                gain_loss,
                gain_loss_percent,
                dividends_total,
                portfolios: Vec::new(),
                custom_logo,
            }
        })
        .collect();

    // STEP 4: Get per-portfolio breakdown using transaction sums (NOT FIFO lots!)
    let portfolio_sql = "
        SELECT
            COALESCE(s.isin, s.uuid) as identifier,
            p.name as portfolio_name,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        JOIN pp_portfolio p ON p.id = t.owner_id AND t.owner_type = 'portfolio'
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.shares IS NOT NULL
        GROUP BY COALESCE(s.isin, s.uuid), p.id
        HAVING net_shares > 0
        ORDER BY identifier, net_shares DESC
    ";

    let mut portfolio_stmt = conn.prepare(portfolio_sql).map_err(|e| e.to_string())?;
    let portfolio_rows = portfolio_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // identifier
                row.get::<_, String>(1)?, // portfolio_name
                row.get::<_, i64>(2)?,    // net_shares
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    // Build a map of identifier -> portfolio holdings
    let mut portfolio_map: std::collections::HashMap<String, Vec<PortfolioHolding>> =
        std::collections::HashMap::new();

    for (identifier, portfolio_name, shares_raw) in portfolio_rows {
        let shares = shares::to_decimal(shares_raw);
        let entry = portfolio_map.entry(identifier.clone()).or_default();

        // Find the price and currency for this identifier from our holdings
        let holding_info = holdings
            .iter()
            .find(|h| h.isin == identifier)
            .map(|h| (h.current_price, h.currency.clone()));

        let value = if let Some((Some(price), security_currency)) = holding_info {
            let value_in_security_currency = price * shares;
            // Convert to base currency if different
            if security_currency == base_currency {
                Some(value_in_security_currency)
            } else {
                Some(
                    currency::convert(conn, value_in_security_currency, &security_currency, &base_currency, today)
                        .unwrap_or(value_in_security_currency)
                )
            }
        } else {
            None
        };

        entry.push(PortfolioHolding {
            portfolio_name,
            shares,
            value,
        });
    }

    // Attach portfolio holdings to each aggregated holding
    for holding in &mut holdings {
        if let Some(portfolios) = portfolio_map.remove(&holding.isin) {
            holding.portfolios = portfolios;
        }
    }

    Ok(holdings)
}

/// Summary statistics for the entire portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSummary {
    pub total_securities: i32,
    pub total_accounts: i32,
    pub total_portfolios: i32,
    pub total_transactions: i32,
    pub total_prices: i32,
    pub date_range: Option<(String, String)>,
}

/// Get summary statistics
#[command]
pub fn get_portfolio_summary(import_id: Option<i64>) -> Result<PortfolioSummary, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (securities, accounts, portfolios, transactions, prices) = if let Some(id) = import_id {
        (
            conn.query_row::<i32, _, _>(
                "SELECT COUNT(*) FROM pp_security WHERE import_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
            conn.query_row::<i32, _, _>(
                "SELECT COUNT(*) FROM pp_account WHERE import_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
            conn.query_row::<i32, _, _>(
                "SELECT COUNT(*) FROM pp_portfolio WHERE import_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
            conn.query_row::<i32, _, _>(
                "SELECT COALESCE(SUM(cnt), 0) FROM (
                     SELECT COUNT(*) as cnt FROM pp_txn t
                     JOIN pp_account a ON t.owner_type = 'account' AND t.owner_id = a.id AND a.import_id = ?1
                     UNION ALL
                     SELECT COUNT(*) FROM pp_txn t
                     JOIN pp_portfolio p ON t.owner_type = 'portfolio' AND t.owner_id = p.id AND p.import_id = ?1
                 )",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
            conn.query_row::<i32, _, _>(
                "SELECT COUNT(*) FROM pp_price pr
                 JOIN pp_security s ON pr.security_id = s.id AND s.import_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
        )
    } else {
        (
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM pp_security", [], |r| r.get(0))
                .unwrap_or(0),
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM pp_account", [], |r| r.get(0))
                .unwrap_or(0),
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM pp_portfolio", [], |r| r.get(0))
                .unwrap_or(0),
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM pp_txn", [], |r| r.get(0))
                .unwrap_or(0),
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM pp_price", [], |r| r.get(0))
                .unwrap_or(0),
        )
    };

    let date_range: Option<(String, String)> = conn
        .query_row(
            "SELECT MIN(date), MAX(date) FROM pp_txn",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    Ok(PortfolioSummary {
        total_securities: securities,
        total_accounts: accounts,
        total_portfolios: portfolios,
        total_transactions: transactions,
        total_prices: prices,
        date_range,
    })
}

/// Historical portfolio value data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioValuePoint {
    pub date: String,
    pub value: f64,
}

/// Get historical portfolio values for charting (last 365 days)
/// Calculates portfolio value = sum of (shares × price) for each security
#[command]
pub fn get_portfolio_history() -> Result<Vec<PortfolioValuePoint>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get current holdings (shares per security) using transaction sums
    let holdings_sql = r#"
        SELECT
            s.id as security_id,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        JOIN pp_portfolio p ON p.id = t.owner_id AND t.owner_type = 'portfolio'
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.shares IS NOT NULL
        GROUP BY s.id
        HAVING net_shares > 0
    "#;

    let mut holdings: Vec<(i64, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(holdings_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((security_id, net_shares)) = row {
                holdings.push((security_id, net_shares));
            }
        }
    }

    if holdings.is_empty() {
        return Ok(vec![]);
    }

    // Get prices for last 365 days for all securities with holdings
    let security_ids: Vec<String> = holdings.iter().map(|(id, _)| id.to_string()).collect();
    let security_ids_str = security_ids.join(",");

    let prices_sql = format!(
        r#"
        SELECT date, security_id, value
        FROM pp_price
        WHERE security_id IN ({})
          AND date >= date('now', '-365 days')
        ORDER BY date
        "#,
        security_ids_str
    );

    // Build a map of security_id -> shares
    let shares_map: std::collections::HashMap<i64, i64> =
        holdings.iter().cloned().collect();

    // Build date -> total value map
    let mut value_by_date: std::collections::BTreeMap<String, f64> =
        std::collections::BTreeMap::new();

    // Track last known price for each security
    let mut last_price: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();

    {
        let mut stmt = conn.prepare(&prices_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((date, security_id, price)) = row {
                last_price.insert(security_id, price);

                // Calculate total portfolio value for this date
                let total: f64 = shares_map
                    .iter()
                    .map(|(sec_id, share_count)| {
                        let price_val = last_price.get(sec_id).copied().unwrap_or(0);
                        shares::to_decimal(*share_count) * prices::to_decimal(price_val)
                    })
                    .sum();

                value_by_date.insert(date, total);
            }
        }
    }

    // Convert to vector
    let result: Vec<PortfolioValuePoint> = value_by_date
        .into_iter()
        .map(|(date, value)| PortfolioValuePoint { date, value })
        .collect();

    Ok(result)
}

/// Get cost basis history showing how the Einstand (cost basis) evolved over time
/// Uses FIFO lots with monthly aggregation for smoother visualization
#[command]
pub fn get_invested_capital_history() -> Result<Vec<PortfolioValuePoint>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get the current actual cost basis from FIFO lots (this is the ground truth)
    let current_cost_basis: f64 = conn
        .query_row(
            r#"SELECT COALESCE(SUM(
                CASE WHEN original_shares > 0 THEN
                    remaining_shares * 1.0 / original_shares * gross_amount
                ELSE 0 END
            ), 0) / 100.0 FROM pp_fifo_lot WHERE remaining_shares > 0"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Get purchase events aggregated by MONTH (YYYY-MM format, use last day of month)
    let purchases_sql = r#"
        SELECT
            strftime('%Y-%m', purchase_date) || '-' ||
            CASE strftime('%m', purchase_date)
                WHEN '01' THEN '31' WHEN '02' THEN '28' WHEN '03' THEN '31'
                WHEN '04' THEN '30' WHEN '05' THEN '31' WHEN '06' THEN '30'
                WHEN '07' THEN '31' WHEN '08' THEN '31' WHEN '09' THEN '30'
                WHEN '10' THEN '31' WHEN '11' THEN '30' WHEN '12' THEN '31'
            END as month_end,
            SUM(gross_amount) / 100.0 as amount
        FROM pp_fifo_lot
        GROUP BY strftime('%Y-%m', purchase_date)
        ORDER BY month_end ASC
    "#;

    // Get sale events aggregated by MONTH
    let sales_sql = r#"
        SELECT
            strftime('%Y-%m', t.date) || '-' ||
            CASE strftime('%m', t.date)
                WHEN '01' THEN '31' WHEN '02' THEN '28' WHEN '03' THEN '31'
                WHEN '04' THEN '30' WHEN '05' THEN '31' WHEN '06' THEN '30'
                WHEN '07' THEN '31' WHEN '08' THEN '31' WHEN '09' THEN '30'
                WHEN '10' THEN '31' WHEN '11' THEN '30' WHEN '12' THEN '31'
            END as month_end,
            SUM(ABS(t.amount)) / 100.0 as amount
        FROM pp_txn t
        WHERE t.owner_type = 'portfolio'
          AND t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND')
          AND t.shares IS NOT NULL
        GROUP BY strftime('%Y-%m', t.date)
        ORDER BY month_end ASC
    "#;

    // Collect purchase events (monthly)
    let mut events: Vec<(String, f64, bool)> = Vec::new();
    {
        let mut stmt = conn.prepare(purchases_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((date, amount)) = row {
                events.push((date, amount, true));
            }
        }
    }

    // Collect sale events (monthly)
    {
        let mut stmt = conn.prepare(sales_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((date, amount)) = row {
                events.push((date, amount, false));
            }
        }
    }

    // Sort by date
    events.sort_by(|a, b| a.0.cmp(&b.0));

    // Calculate total purchases and total sales proceeds
    let total_purchases: f64 = events.iter().filter(|e| e.2).map(|e| e.1).sum();
    let total_sales: f64 = events.iter().filter(|e| !e.2).map(|e| e.1).sum();

    // The cost basis consumed by sales is: total_purchases - current_cost_basis
    let consumed_cost_basis = total_purchases - current_cost_basis;

    // Scale factor: how much of sale proceeds represents cost basis (vs. gain/loss)
    let sale_to_cost_ratio = if total_sales > 0.0 {
        consumed_cost_basis / total_sales
    } else {
        0.0
    };

    // Build cumulative cost basis by month
    let mut cost_basis_by_month: std::collections::BTreeMap<String, f64> =
        std::collections::BTreeMap::new();
    let mut cumulative: f64 = 0.0;

    for (date, amount, is_purchase) in events {
        if is_purchase {
            cumulative += amount;
        } else {
            cumulative -= amount * sale_to_cost_ratio;
        }
        cost_basis_by_month.insert(date, cumulative.max(0.0));
    }

    // Convert to vector
    let result: Vec<PortfolioValuePoint> = cost_basis_by_month
        .into_iter()
        .map(|(date, value)| PortfolioValuePoint { date, value })
        .collect();

    Ok(result)
}

/// Upload a custom logo for a security (base64-encoded)
#[command]
pub fn upload_security_logo(security_id: i64, logo_data: String) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "UPDATE pp_security SET custom_logo = ?1 WHERE id = ?2",
        params![logo_data, security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete the custom logo for a security
#[command]
pub fn delete_security_logo(security_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "UPDATE pp_security SET custom_logo = NULL WHERE id = ?1",
        params![security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get the custom logo for a security
#[command]
pub fn get_security_logo(security_id: i64) -> Result<Option<String>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let result: Option<String> = conn
        .query_row(
            "SELECT custom_logo FROM pp_security WHERE id = ?1",
            params![security_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(result)
}

// ============================================================================
// FIFO Cost Basis History
// ============================================================================

/// A snapshot of the FIFO cost basis at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FifoCostBasisSnapshot {
    pub date: String,
    pub shares: f64,
    pub cost_per_share: f64,
    pub total_cost_basis: f64,
}

/// Transaction with trade details for chart markers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMarker {
    pub date: String,
    pub txn_type: String,
    pub shares: f64,
    pub price_per_share: f64,
    pub amount: f64,
    pub fees: f64,
    pub taxes: f64,
}

/// Complete data for security detail chart
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityChartData {
    pub cost_basis_history: Vec<FifoCostBasisSnapshot>,
    pub trades: Vec<TradeMarker>,
}

/// Get FIFO cost basis history for a security over time
/// Returns snapshots at each transaction date showing:
/// - Total shares held
/// - Cost per share (Einstandskurs)
/// - Total cost basis (Einstandswert)
///
/// Note: This queries by ISIN to handle aggregated holdings where multiple
/// security entries may exist with the same ISIN (from different imports).
#[command]
pub fn get_fifo_cost_basis_history(security_id: i64) -> Result<SecurityChartData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // First, get the ISIN for the given security_id
    // If no ISIN, use the UUID as identifier
    let identifier: String = conn
        .query_row(
            "SELECT COALESCE(isin, uuid) FROM pp_security WHERE id = ?1",
            params![security_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Security not found: {}", e))?;

    // Get all portfolio transactions for securities with this ISIN, sorted by date
    // Same ordering as FIFO processing: BUY first, then TRANSFER, then SELL
    let mut stmt = conn.prepare(r#"
        SELECT
            t.id, t.txn_type, t.date, t.amount, t.shares,
            COALESCE(SUM(CASE WHEN u.unit_type = 'FEE' THEN u.amount ELSE 0 END), 0) as fees,
            COALESCE(SUM(CASE WHEN u.unit_type = 'TAX' THEN u.amount ELSE 0 END), 0) as taxes
        FROM pp_txn t
        LEFT JOIN pp_txn_unit u ON u.txn_id = t.id
        JOIN pp_security s ON s.id = t.security_id
        WHERE COALESCE(s.isin, s.uuid) = ?1 AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
        GROUP BY t.id
        ORDER BY
            date(t.date),
            CASE t.txn_type
                WHEN 'BUY' THEN 1
                WHEN 'DELIVERY_INBOUND' THEN 1
                WHEN 'TRANSFER_IN' THEN 2
                WHEN 'TRANSFER_OUT' THEN 3
                WHEN 'SELL' THEN 4
                WHEN 'DELIVERY_OUTBOUND' THEN 4
                ELSE 5
            END,
            t.id
    "#).map_err(|e| e.to_string())?;

    struct TxnRow {
        _id: i64,
        txn_type: String,
        date: String,
        amount: i64,
        shares: i64,
        fees: i64,
        taxes: i64,
    }

    let transactions: Vec<TxnRow> = stmt
        .query_map([&identifier], |row| {
            Ok(TxnRow {
                _id: row.get(0)?,
                txn_type: row.get(1)?,
                date: row.get(2)?,
                amount: row.get(3)?,
                shares: row.get(4)?,
                fees: row.get(5)?,
                taxes: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Process transactions to build cost basis history
    // Using simplified FIFO: track total shares and total cost
    let mut total_shares: i64 = 0;
    let mut total_cost: i64 = 0; // In cents (amount scale)
    let mut snapshots: Vec<FifoCostBasisSnapshot> = Vec::new();
    let mut trades: Vec<TradeMarker> = Vec::new();

    for txn in transactions {
        let shares_decimal = shares::to_decimal(txn.shares);
        let amount_decimal = txn.amount as f64 / 100.0;
        let fees_decimal = txn.fees as f64 / 100.0;
        let taxes_decimal = txn.taxes as f64 / 100.0;

        match txn.txn_type.as_str() {
            "BUY" | "DELIVERY_INBOUND" | "TRANSFER_IN" => {
                // Add shares and cost
                total_shares += txn.shares;
                total_cost += txn.amount; // amount includes fees/taxes for cost basis

                // Calculate price per share for trade marker
                let price_per_share = if txn.shares > 0 {
                    // Net amount (without fees/taxes) / shares
                    let net_amount = txn.amount - txn.fees - txn.taxes;
                    (net_amount as f64 / 100.0) / shares_decimal
                } else {
                    0.0
                };

                trades.push(TradeMarker {
                    date: txn.date.clone(),
                    txn_type: txn.txn_type.clone(),
                    shares: shares_decimal,
                    price_per_share,
                    amount: amount_decimal,
                    fees: fees_decimal,
                    taxes: taxes_decimal,
                });
            }
            "SELL" | "DELIVERY_OUTBOUND" | "TRANSFER_OUT" => {
                // Remove shares proportionally (FIFO average)
                if total_shares > 0 {
                    // Calculate proportional cost to remove
                    let cost_per_share = total_cost as f64 / total_shares as f64;
                    let cost_removed = (cost_per_share * txn.shares as f64) as i64;
                    total_cost -= cost_removed;
                    total_shares -= txn.shares;

                    // Ensure non-negative
                    if total_cost < 0 {
                        total_cost = 0;
                    }
                    if total_shares < 0 {
                        total_shares = 0;
                    }
                }

                // Calculate price per share for trade marker
                let price_per_share = if txn.shares > 0 {
                    let net_amount = txn.amount - txn.fees - txn.taxes;
                    (net_amount as f64 / 100.0) / shares_decimal
                } else {
                    0.0
                };

                trades.push(TradeMarker {
                    date: txn.date.clone(),
                    txn_type: txn.txn_type.clone(),
                    shares: shares_decimal,
                    price_per_share,
                    amount: amount_decimal,
                    fees: fees_decimal,
                    taxes: taxes_decimal,
                });
            }
            _ => {}
        }

        // Create snapshot after each transaction
        let shares_decimal = shares::to_decimal(total_shares);
        let total_cost_decimal = total_cost as f64 / 100.0;
        let cost_per_share = if total_shares > 0 {
            total_cost_decimal / shares_decimal
        } else {
            0.0
        };

        snapshots.push(FifoCostBasisSnapshot {
            date: txn.date,
            shares: shares_decimal,
            cost_per_share,
            total_cost_basis: total_cost_decimal,
        });
    }

    Ok(SecurityChartData {
        cost_basis_history: snapshots,
        trades,
    })
}
