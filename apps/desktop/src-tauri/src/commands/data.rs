//! Data query commands for accessing imported Portfolio Performance data.

use crate::db;
use crate::pp::common::{prices, shares};
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
    pub is_retired: bool,
    pub latest_price: Option<f64>,
    pub latest_price_date: Option<String>,
    pub prices_count: i32,
}

/// Get all securities from the database
#[command]
pub fn get_securities(import_id: Option<i64>) -> Result<Vec<SecurityData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let sql = if import_id.is_some() {
        "SELECT s.id, s.uuid, s.name, s.currency, s.isin, s.wkn, s.ticker, s.is_retired,
                lp.value, lp.date,
                (SELECT COUNT(*) FROM pp_price WHERE security_id = s.id) as prices_count
         FROM pp_security s
         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
         WHERE s.import_id = ?1
         ORDER BY s.name"
    } else {
        "SELECT s.id, s.uuid, s.name, s.currency, s.isin, s.wkn, s.ticker, s.is_retired,
                lp.value, lp.date,
                (SELECT COUNT(*) FROM pp_price WHERE security_id = s.id) as prices_count
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

    let securities: Vec<SecurityData> = rows
        .mapped(|row| {
            let latest_value: Option<i64> = row.get(8)?;
            Ok(SecurityData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                currency: row.get(3)?,
                isin: row.get(4)?,
                wkn: row.get(5)?,
                ticker: row.get(6)?,
                is_retired: row.get::<_, i32>(7)? != 0,
                latest_price: latest_value.map(prices::to_decimal),
                latest_price_date: row.get(9)?,
                prices_count: row.get(10)?,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedHolding {
    pub isin: String,
    pub name: String,
    pub currency: String,
    pub total_shares: f64,
    pub current_price: Option<f64>,
    pub current_value: Option<f64>,
    pub cost_basis: f64,
    pub gain_loss: Option<f64>,
    pub gain_loss_percent: Option<f64>,
    pub portfolios: Vec<PortfolioHolding>,
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

    // STEP 1: Calculate holdings using transaction sums (PP PortfolioSnapshot logic)
    // shares = SUM(BUY/TRANSFER_IN/DELIVERY_INBOUND) - SUM(SELL/TRANSFER_OUT/DELIVERY_OUTBOUND)
    let holdings_sql = "
        WITH portfolio_holdings AS (
            SELECT
                COALESCE(s.isin, s.uuid) as identifier,
                MAX(s.id) as security_id,
                MAX(s.name) as name,
                MAX(s.currency) as currency,
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
            lp.value as latest_price
        FROM portfolio_holdings ph
        LEFT JOIN pp_latest_price lp ON lp.security_id = ph.security_id
        ORDER BY ph.net_shares * COALESCE(lp.value, 0) DESC
    ";

    let mut holdings_stmt = conn.prepare(holdings_sql).map_err(|e| e.to_string())?;

    // STEP 2: Get cost basis from FIFO lots (separate from share count!)
    let cost_basis_sql = "
        SELECT
            COALESCE(s.isin, s.uuid) as identifier,
            SUM(CASE
                WHEN l.original_shares > 0 THEN
                    (l.remaining_shares * l.net_amount / l.original_shares)
                ELSE 0
            END) as cost_basis
        FROM pp_fifo_lot l
        JOIN pp_security s ON l.security_id = s.id
        WHERE l.remaining_shares > 0
        GROUP BY COALESCE(s.isin, s.uuid)
    ";

    let mut cost_basis_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut cost_stmt = conn.prepare(cost_basis_sql).map_err(|e| e.to_string())?;
    let cost_rows = cost_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;

    for row in cost_rows {
        if let Ok((identifier, cost)) = row {
            cost_basis_map.insert(identifier, cost);
        }
    }

    // Build holdings with transaction-based shares and FIFO-based cost basis
    let mut holdings: Vec<AggregatedHolding> = holdings_stmt
        .query_map([], |row| {
            let identifier: String = row.get(0)?;
            let name: String = row.get(1)?;
            let currency: String = row.get(2)?;
            let _security_id: i64 = row.get(3)?;
            let shares_raw: i64 = row.get(4)?;
            let latest_price_raw: Option<i64> = row.get(5)?;

            Ok((identifier, name, currency, shares_raw, latest_price_raw))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|(identifier, name, currency, shares_raw, latest_price_raw)| {
            let total_shares = shares::to_decimal(shares_raw);
            let current_price = latest_price_raw.map(prices::to_decimal);
            let current_value = current_price.map(|p| p * total_shares);

            // Get cost basis from FIFO map
            let cost_basis_cents = cost_basis_map.get(&identifier).copied().unwrap_or(0);
            let cost_basis = cost_basis_cents as f64 / 100.0;

            let gain_loss = current_value.map(|v| v - cost_basis);
            let gain_loss_percent = if cost_basis > 0.0 {
                gain_loss.map(|g| (g / cost_basis) * 100.0)
            } else {
                None
            };

            AggregatedHolding {
                isin: identifier,
                name,
                currency,
                total_shares,
                current_price,
                current_value,
                cost_basis,
                gain_loss,
                gain_loss_percent,
                portfolios: Vec::new(),
            }
        })
        .collect();

    // STEP 3: Get per-portfolio breakdown using transaction sums (NOT FIFO lots!)
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

        // Find the price for this identifier from our holdings
        let price = holdings
            .iter()
            .find(|h| h.isin == identifier)
            .and_then(|h| h.current_price);

        entry.push(PortfolioHolding {
            portfolio_name,
            shares,
            value: price.map(|p| p * shares),
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
                "SELECT COUNT(*) FROM pp_txn t
                 JOIN pp_account a ON t.owner_type = 'account' AND t.owner_id = a.id AND a.import_id = ?1
                 UNION ALL
                 SELECT COUNT(*) FROM pp_txn t
                 JOIN pp_portfolio p ON t.owner_type = 'portfolio' AND t.owner_id = p.id AND p.import_id = ?1",
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
