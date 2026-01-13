//! Performance calculation commands for Tauri

use crate::db;
use crate::performance;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;
use tauri::command;

/// Parse date string flexibly - handles both "YYYY-MM-DD" and "YYYY-MM-DD HH:MM:SS" formats
fn parse_date_flexible(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
}

/// Performance result for frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceResult {
    /// TTWROR (True Time-Weighted Rate of Return) as percentage
    pub ttwror: f64,
    /// Annualized TTWROR as percentage
    pub ttwror_annualized: f64,
    /// IRR (Internal Rate of Return) as percentage
    pub irr: f64,
    /// Whether IRR calculation converged
    pub irr_converged: bool,
    /// Number of days in the period
    pub days: i64,
    /// Start date
    pub start_date: String,
    /// End date
    pub end_date: String,
    /// Current portfolio value
    pub current_value: f64,
    /// Total invested (sum of deposits)
    pub total_invested: f64,
    /// Absolute gain/loss
    pub absolute_gain: f64,
}

/// Detailed period return for charts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodReturnData {
    pub start_date: String,
    pub end_date: String,
    pub start_value: f64,
    pub end_value: f64,
    pub cash_flow: f64,
    pub return_rate: f64,
}

/// Calculate performance metrics for a portfolio
#[command]
pub fn calculate_performance(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<PerformanceResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Parse dates or use defaults
    let start = start_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| {
            // Default to first transaction date
            get_first_transaction_date(conn, portfolio_id)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        });

    let end = end_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    // Calculate TTWROR
    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    log::info!(
        "Performance TTWROR: {} periods, total_return={:.2}%, days={}",
        ttwror_result.periods.len(),
        ttwror_result.total_return * 100.0,
        ttwror_result.days
    );

    // Get cash flows for IRR calculation
    let cash_flows = get_cash_flows_for_irr(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    log::info!("Performance IRR: {} cash flows found", cash_flows.len());
    for cf in &cash_flows {
        log::debug!("  Cash flow: {} on {}", cf.amount, cf.date);
    }

    // Get current portfolio value
    let current_value = get_current_value(conn, portfolio_id).map_err(|e| e.to_string())?;

    log::info!("Performance: current_value={:.2}", current_value);

    // Calculate IRR
    let irr_result = performance::calculate_irr(&cash_flows, current_value, end)
        .map_err(|e| e.to_string())?;

    log::info!(
        "Performance IRR result: irr={:.2}%, converged={}",
        irr_result.irr * 100.0,
        irr_result.converged
    );

    // Calculate total invested (positive cash flows = money invested)
    let total_invested: f64 = cash_flows.iter().filter(|cf| cf.amount > 0.0).map(|cf| cf.amount).sum();

    // Calculate total withdrawn (negative cash flows = money withdrawn)
    let total_withdrawn: f64 = cash_flows.iter().filter(|cf| cf.amount < 0.0).map(|cf| -cf.amount).sum();

    // Absolute gain = current value + withdrawals - investments
    let absolute_gain = current_value + total_withdrawn - total_invested;

    Ok(PerformanceResult {
        ttwror: ttwror_result.total_return * 100.0,
        ttwror_annualized: ttwror_result.annualized_return * 100.0,
        irr: irr_result.irr * 100.0,
        irr_converged: irr_result.converged,
        days: ttwror_result.days,
        start_date: start.to_string(),
        end_date: end.to_string(),
        current_value,
        total_invested,
        absolute_gain,
    })
}

/// Get period returns for detailed analysis
#[command]
pub fn get_period_returns(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<PeriodReturnData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = start_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| {
            get_first_transaction_date(conn, portfolio_id)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        });

    let end = end_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    let periods: Vec<PeriodReturnData> = ttwror_result
        .periods
        .into_iter()
        .map(|p| PeriodReturnData {
            start_date: p.start_date.to_string(),
            end_date: p.end_date.to_string(),
            start_value: p.start_value,
            end_value: p.end_value,
            cash_flow: p.cash_flow,
            return_rate: p.return_rate * 100.0,
        })
        .collect();

    Ok(periods)
}

/// Helper: Get first transaction date
fn get_first_transaction_date(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
) -> Option<NaiveDate> {
    let sql = if portfolio_id.is_some() {
        "SELECT MIN(date) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = ?1"
    } else {
        "SELECT MIN(date) FROM pp_txn WHERE owner_type = 'portfolio'"
    };

    let result: Option<String> = if let Some(pid) = portfolio_id {
        conn.query_row(sql, [pid], |row| row.get(0)).ok()
    } else {
        conn.query_row(sql, [], |row| row.get(0)).ok()
    };

    result.and_then(|s| parse_date_flexible(&s))
}

/// Helper: Get cash flows for IRR
/// Considers both account-level (DEPOSIT/REMOVAL) and portfolio-level (BUY/SELL/DELIVERY) transactions
fn get_cash_flows_for_irr(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<performance::CashFlow>, anyhow::Error> {
    use rusqlite::params;

    let mut cash_flows = Vec::new();

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND owner_id = {}", id))
        .unwrap_or_default();

    // Get cash flows from portfolio transactions (BUY/SELL/DELIVERY)
    // These represent actual money in/out of the portfolio
    let portfolio_sql = format!(
        r#"
        SELECT date, txn_type, amount
        FROM pp_txn
        WHERE owner_type = 'portfolio'
          AND txn_type IN ('BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND')
          AND amount IS NOT NULL
          AND date(date) >= ?1 AND date(date) <= ?2
          {}
        ORDER BY date
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&portfolio_sql)?;
    let rows = stmt.query_map(
        params![start_date.to_string(), end_date.to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;

    for row in rows.flatten() {
        let (date_str, txn_type, amount) = row;
        if let Some(date) = parse_date_flexible(&date_str) {
            let amount_f = amount as f64 / 100.0;
            // For IRR convention: positive = money invested, negative = money withdrawn
            // calculate_irr() will invert these internally for NPV calculation
            let cf_amount = match txn_type.as_str() {
                "BUY" | "DELIVERY_INBOUND" => amount_f,   // Money invested (positive)
                "SELL" | "DELIVERY_OUTBOUND" => -amount_f, // Money withdrawn (negative)
                _ => 0.0,
            };
            if cf_amount != 0.0 {
                cash_flows.push(performance::CashFlow { date, amount: cf_amount });
            }
        }
    }

    // Also check for account-level DEPOSIT/REMOVAL (for portfolios with linked accounts)
    if let Some(pid) = portfolio_id {
        let account_sql = r#"
            SELECT t.date, t.txn_type, t.amount
            FROM pp_txn t
            JOIN pp_portfolio p ON p.reference_account_id = t.owner_id
            WHERE t.owner_type = 'account'
              AND t.txn_type IN ('DEPOSIT', 'REMOVAL')
              AND p.id = ?1
              AND date(t.date) >= ?2 AND date(t.date) <= ?3
            ORDER BY t.date
        "#;

        let mut stmt = conn.prepare(account_sql)?;
        let rows = stmt.query_map(
            params![pid, start_date.to_string(), end_date.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;

        for row in rows.flatten() {
            let (date_str, txn_type, amount) = row;
            if let Some(date) = parse_date_flexible(&date_str) {
                let amount_f = amount as f64 / 100.0;
                // Same convention: positive = invested, negative = withdrawn
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,   // Money invested (positive)
                    "REMOVAL" => -amount_f,  // Money withdrawn (negative)
                    _ => 0.0,
                };
                if cf_amount != 0.0 {
                    cash_flows.push(performance::CashFlow { date, amount: cf_amount });
                }
            }
        }
    }

    // Sort by date
    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(cash_flows)
}

/// Helper: Get current portfolio value
fn get_current_value(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
) -> Result<f64, anyhow::Error> {
    

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    let holdings_sql = format!(
        r#"
        SELECT
            t.security_id,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
          {}
        GROUP BY t.security_id
        HAVING net_shares > 0
        "#,
        portfolio_filter
    );

    let mut total_value = 0.0;

    {
        let mut stmt = conn.prepare(&holdings_sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            if let Ok((security_id, share_count)) = row {
                let price_sql = "SELECT value FROM pp_latest_price WHERE security_id = ?";
                let price: Option<i64> = conn.query_row(price_sql, [security_id], |row| row.get(0)).ok();

                if let Some(p) = price {
                    let shares_f = share_count as f64 / 100_000_000.0;
                    let price_f = p as f64 / 100_000_000.0;
                    total_value += shares_f * price_f;
                }
            }
        }
    }

    Ok(total_value)
}
